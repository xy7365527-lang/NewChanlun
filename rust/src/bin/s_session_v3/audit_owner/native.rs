//! #1394 Native 的独立上界账。这里从不写入 Owned requested Layout 账。
//! 许可先占完整新块，扩容返回后才释放旧块；临时许可由 Drop 归还。
use crate::v3::audit_retention::{hash_slots, sum, tree_slots, vec_bytes, Retained};
use std::{
    alloc::Layout,
    collections::{BTreeMap, HashMap},
    hash::Hash,
    mem::size_of,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BudgetExceeded;
#[derive(Debug)]
pub(crate) enum Failure {
    BudgetExceeded,
    Invalid(String),
}

#[cfg(test)]
mod r6_codec_work_tests {
    use super::*;
    use newchan_rust::s_content::{self as sc, capture_allocation::scratch::WorkError};

    #[test]
    fn r6_codec_completed_scope_keeps_readable_bytes_and_rejects_writes() {
        let budget = Budget::new(4096, 0).unwrap();
        let base = budget.live();
        let mut bytes = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let mut bytes = scope.bytes();
                bytes.try_reserve_exact(32)?;
                bytes.try_extend_bytes(b"kept")?;
                Ok::<_, sc::CodecError>(bytes)
            })
            .unwrap();
        assert_eq!(&*bytes, b"kept");
        assert!(bytes.capacity() >= 32);
        assert!(budget.scratch_observation().0 >= 32);
        assert_eq!(budget.scratch_observation().2, CodecWork::metadata_bytes());
        let before = budget.live();
        for error in [
            bytes.try_push(b'x').unwrap_err(),
            bytes.try_extend_bytes(b"tail").unwrap_err(),
            bytes.try_reserve_exact(0).unwrap_err(),
        ] {
            assert_eq!(error.scratch_failure(), Some(ScratchFailure::Ledger));
        }
        assert_eq!(&*bytes, b"kept");
        assert_eq!(budget.live(), before);
        drop(bytes);
        assert_eq!(budget.live(), base);
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[test]
    fn r6_codec_real_verify_render_lookup_writer_plain_and_scoped_memo() {
        use crate::v3::{
            audit_store::{CodecRead, CodecStore, RefValue},
            business_json,
            history::HashReader,
        };
        let value = sc::Value::Object(vec![
            (
                "items".into(),
                sc::Value::Array(vec![sc::Value::Null, sc::Value::Bool(true)]),
            ),
            ("name".into(), sc::Value::String("中😀\n\"".repeat(1800))),
            (
                "nested".into(),
                sc::Value::JsonText(Box::new(sc::JsonValue::String("escaped\n😀".into()))),
            ),
        ]);
        let mut nodes = sc::MemoryNodes::new();
        let reference = sc::write_value(&mut nodes, &value).unwrap().content_ref();
        let mut expected = Vec::new();
        sc::render_canonical(&mut nodes, &reference, &mut expected).unwrap();
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let before = capture.retention();
        for _ in 0..2 {
            let budget = Budget::new(8 * 1024 * 1024, 0).unwrap();
            let base = budget.live();
            budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let mut reader = HashReader::captured_for_audit(&capture, None)
                        .unwrap()
                        .with_codec_work(scope, &budget);
                    reader.enable_full_arrays_for_test();
                    assert!(reader.allocation_budget().is_none());
                    reader.verify_many_ref([(&reference).into()], sc::Mode::Public)?;
                    assert_eq!(scope.snapshot().live, 0, "verification memo 随操作析构");
                    let mut actual = Vec::new();
                    reader.render_ref((&reference).into(), &mut actual)?;
                    assert_eq!(actual, expected);
                    let items = reader.lookup_field_ref((&reference).into(), "items")?;
                    assert!(matches!(items, RefValue::Plain(_)));
                    let items = reader.lookup_array_ref(items.view())?;
                    assert!(items.allocation_budget().is_none());
                    assert_eq!(items.len(), 2);
                    let fields = reader
                        .lookup_fields_ref((&reference).into(), &["items", "name", "nested"])?;
                    assert!(matches!(
                        fields,
                        crate::v3::audit_store::RefFields::Plain(_)
                    ));
                    let rebuilt = reader.write_value(&value)?;
                    assert_eq!(rebuilt.content_ref(), reference);
                    assert_eq!(scope.snapshot().live, 0);
                    // 真正消费临时 scratch 输出，再在 Native 解析许可下产出 Plain。
                    let parsed =
                        business_json::render_json(&mut reader, (&reference).into()).unwrap();
                    assert!(matches!(parsed, business_json::JsonOwner::Plain(_)));
                    assert_eq!(scope.snapshot().live, 0);
                    assert!(budget.live() > base + CodecWork::metadata_bytes());
                    eprintln!(
                        "R6_CODEC_FLOW snapshot={:?} parent_parts={:?}",
                        scope.snapshot(),
                        budget.scratch_observation()
                    );
                    Ok::<_, sc::CodecError>(())
                })
                .unwrap();
            assert_eq!(budget.scratch_observation().0, 0);
            assert_eq!(budget.scratch_observation().2, 0);
            let after = capture.retention();
            assert_eq!(before["public"], after["public"], "不能回填跨 scope memo");
            assert_eq!(before["template"], after["template"]);
        }
        let base = Budget::new(4096, 0).unwrap().live();
        let tight = Budget::new(base + CodecWork::metadata_bytes() + 1, 0).unwrap();
        let result = tight.codec_work().unwrap().run(|scope| {
            let mut reader = HashReader::captured_for_audit(&capture, None)
                .unwrap()
                .with_codec_work(scope, &tight);
            let result = reader.verify_ref((&reference).into(), sc::Mode::Public);
            assert_eq!(
                scope.snapshot().attempts,
                0,
                "实际 verifier 在首个分配前 typed 拒绝"
            );
            result
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(tight.live(), base);
        let budget = Budget::new(8 * 1024 * 1024, 0).unwrap();
        let mut ordinary = HashReader::captured_for_audit(&capture, None).unwrap();
        let expected_error = ordinary
            .lookup_field_ref((&reference).into(), "absent")
            .unwrap_err();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut reader = HashReader::captured_for_audit(&capture, None)
                .unwrap()
                .with_codec_work(scope, &budget);
            reader.lookup_field_ref((&reference).into(), "absent")
        });
        match result {
            Err(WorkError::Operation(e)) => assert_eq!(e, expected_error),
            _ => panic!("普通 lookup 语义失败不得变成资源拒绝"),
        }
        assert_eq!(budget.live(), base);
    }

    #[test]
    fn r6_codec_render_output_refusal_keeps_write_cause() {
        use crate::v3::{audit_store::CodecRead, business_json, history::HashReader};
        let mut nodes = sc::MemoryNodes::new();
        let reference = sc::write_value(&mut nodes, &sc::Value::String("x".repeat(48_000)))
            .unwrap()
            .content_ref();
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let budget = Budget::new(65_536, 0).unwrap();
        let base = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut reader = HashReader::captured_for_audit(&capture, None)
                .unwrap()
                .with_codec_work(scope, &budget);
            reader
                .verify_ref((&reference).into(), sc::Mode::Public)
                .unwrap();
            let rendered = business_json::render_json(&mut reader, (&reference).into());
            assert!(rendered.is_err());
            assert_eq!(scope.snapshot().live, 0);
            eprintln!("R6_CODEC_RENDER_REFUSAL snapshot={:?}", scope.snapshot());
            rendered
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(budget.live(), base);
    }

    #[test]
    fn r6_codec_parent_metadata_growth_refusal_and_real_drop() {
        let base = Budget::new(4096, 0).unwrap().live();
        let meta = CodecWork::metadata_bytes();
        let tight = Budget::new(base + meta - 1, 0).unwrap();
        assert!(matches!(tight.codec_work(), Err(Failure::BudgetExceeded)));
        assert_eq!(tight.live(), base);
        for payload in [95, 96] {
            let b = Budget::new(base + meta + payload, 0).unwrap();
            let w = b.codec_work().unwrap();
            let result = w.run(|scope| {
                let mut bytes = scope.bytes();
                bytes.try_reserve_exact(32)?;
                bytes.try_extend_bytes(&[7; 32])?;
                let before = scope.snapshot();
                let grown = bytes.try_reserve_exact(32);
                if payload == 95 {
                    let e = grown.unwrap_err();
                    assert_eq!(e.scratch_failure(), Some(ScratchFailure::Budget));
                    assert_eq!(&*bytes, &[7; 32]);
                    assert_eq!(bytes.capacity(), 32);
                    assert_eq!(scope.snapshot().attempts, before.attempts);
                    assert_eq!(b.live(), base + meta + 32);
                    return Err(e);
                }
                grown?;
                assert_eq!(&*bytes, &[7; 32]);
                assert_eq!(b.peak(), base + meta + 96);
                assert_eq!(scope.snapshot().peak, 96);
                Ok(())
            });
            if payload == 95 {
                assert!(matches!(
                    result,
                    Err(WorkError::Resource(ScratchFailure::Budget))
                ));
            } else {
                result.unwrap();
            }
            assert_eq!(b.live(), base);
            assert_eq!(b.scratch_observation().0, 0);
            assert_eq!(b.scratch_observation().2, 0);
            eprintln!(
                "R6_CODEC_GROW payload={payload} base={base} metadata={meta} parts={:?} peak={}",
                b.scratch_observation(),
                b.peak()
            );
        }
        let b = Budget::new(4096, 0).unwrap();
        let result = CodecWork::new(b.0.clone(), 31).unwrap().run(|scope| {
            let mut bytes = scope.bytes();
            let result = bytes.try_reserve_exact(32);
            assert_eq!(scope.snapshot().attempts, 0, "子账拒绝必须在真实 alloc 前");
            assert_eq!(b.live(), base + meta, "父预留已回滚");
            result
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(b.live(), base);

        let memory = super::super::allocation_probe::Window::start();
        let work = b.codec_work().unwrap();
        let observed = memory.finish();
        assert_eq!(observed["allocated_bytes"], meta, "当前 std Arc 元数据实测");
        let bytes = work
            .run(|scope| {
                let mut bytes = scope.bytes();
                bytes.try_reserve_exact(32)?;
                Ok::<_, sc::CodecError>(bytes)
            })
            .unwrap();
        assert_eq!(b.live(), base + meta + 32, "容器离开闭包仍持元数据账");
        let memory = super::super::allocation_probe::Window::start();
        drop(bytes);
        let released = memory.finish();
        assert_eq!(released["released_bytes"], meta + 32);
        assert_eq!(b.live(), base);
        eprintln!("R6_CODEC_METADATA layout={meta} allocate={observed} release={released}");
        fn send_sync<T: Send + Sync>() {}
        send_sync::<sc::capture_allocation::OwnedBudget>();
        send_sync::<Budget>();
    }
}
impl From<BudgetExceeded> for Failure {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}
impl From<String> for Failure {
    fn from(e: String) -> Self {
        Self::Invalid(e)
    }
}
#[derive(Clone)]
pub(crate) struct Budget(Arc<Ledger>, u64, u64, usize);
// 请求号在不可变Budget句柄上；父账只保存紧凑的来源序号和活动scope计数。
// 两个计数都checked拒绝溢出，不回绕、不复用旧来源；避免扩大原128字节小帽探针。
struct Domain {
    remaining: usize,
    low_water: usize,
    total_limit: usize,
    // #1488 v4：只由活动MoveIn写入的已真实预留移入池；不是无票unassigned余额。
    // 同尺寸替换原字段，保持128字节父账probe输入不变。
    moved_in: usize,
    serial: u32,
    active_scopes: u16,
    state: u8,
}
const BROKEN: u8 = 1;
const BOUND: u8 = 2;
// 同一父账同时至多一个活动MoveIn；不参与冻结/绑定状态判定。
const MOVING: u8 = 4;
struct Ledger {
    domain: Mutex<Domain>,
    live: AtomicUsize,
    peak: AtomicUsize,
    scratch_live: AtomicUsize,
    scratch_peak: AtomicUsize,
    metadata_live: AtomicUsize,
    metadata_peak: AtomicUsize,
    native_live: AtomicUsize,
    native_peak: AtomicUsize,
}
pub(crate) struct Permit {
    budget: Budget,
    bytes: usize,
    origin: u64,
}
/// 字段按声明顺序析构：被保护的工作集先真正释放，许可随后归还。
/// 避免 `(value, permit)` 局部绑定逆序析构时，先放掉许可再释放 value。
pub(crate) struct Reserved<T> {
    inner: T,
    _permit: Permit,
}
/// 可选父账路径也用同一个持有者，错误出口不会先释放费用后释放载荷。
pub(crate) struct Tracked<T> {
    inner: T,
    fee: Option<Permit>,
}
impl<T> Tracked<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self { inner, fee: None }
    }
    pub(crate) fn attach(&mut self, fee: Option<Permit>) {
        assert!(self.fee.is_none());
        self.fee = fee;
    }
    pub(crate) fn has_permit(&self) -> bool {
        self.fee.is_some()
    }
}
impl<T> std::ops::Deref for Tracked<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}
impl<T> std::ops::DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<T: Retained> Retained for Tracked<T> {
    fn heap_bytes(&self) -> usize {
        self.inner.heap_bytes()
    }
}
impl<'a, T> IntoIterator for &'a mut Tracked<Vec<T>> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}
impl<'a, T> IntoIterator for &'a Tracked<Vec<T>> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
impl<T> std::ops::Deref for Reserved<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.budget
            .0
            .release_total(self.bytes, self.origin)
            .expect("Native许可返还失配");
    }
}
impl Permit {
    pub(crate) fn hold<T>(self, value: T) -> Reserved<T> {
        Reserved {
            inner: value,
            _permit: self,
        }
    }
    /// 新块已构造后只归还未使用部分；返回的许可必须随载荷持有。
    pub(crate) fn finish(mut self, actual: usize) -> Self {
        assert!(actual <= self.bytes, "预留必须覆盖新保留费");
        self.budget
            .0
            .release_total(self.bytes - actual, self.origin)
            .expect("Native许可结算");
        self.bytes = actual;
        self
    }
    pub(crate) fn split(&mut self, bytes: usize) -> Result<Self, BudgetExceeded> {
        let remaining = self.bytes.checked_sub(bytes).ok_or(BudgetExceeded)?;
        self.bytes = remaining;
        Ok(Self {
            budget: self.budget.clone(),
            bytes,
            origin: self.origin,
        })
    }
    pub(crate) fn replace_part(
        &mut self,
        old: usize,
        mut next: Self,
    ) -> Result<(), BudgetExceeded> {
        if !self.budget.same_parent(&next.budget) || self.origin != next.origin {
            return Err(BudgetExceeded);
        }
        self.bytes
            .checked_sub(old)
            .and_then(|n| n.checked_add(next.bytes))
            .ok_or(BudgetExceeded)?;
        drop(self.split(old)?);
        self.merge(&mut next)
    }
    pub(crate) fn merge(&mut self, other: &mut Self) -> Result<(), BudgetExceeded> {
        if !self.budget.same_parent(&other.budget) || self.origin != other.origin {
            return Err(BudgetExceeded);
        }
        let bytes = self.bytes.checked_add(other.bytes).ok_or(BudgetExceeded)?;
        self.bytes = bytes;
        other.bytes = 0;
        Ok(())
    }
}
impl<T> std::ops::DerefMut for Reserved<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<T: Retained> Retained for Reserved<T> {
    fn heap_bytes(&self) -> usize {
        self.inner.heap_bytes()
    }
}

impl Budget {
    #[cfg(test)]
    pub(super) fn native_observation(&self) -> (usize, usize) {
        (
            self.0.native_live.load(Ordering::Acquire),
            self.0.native_peak.load(Ordering::Acquire),
        )
    }
    pub(crate) fn metadata_bytes() -> usize {
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<Ledger>())
            .expect("父账元数据布局")
            .0
            .pad_to_align()
            .size()
    }
    pub(crate) fn pristine_cold_parent(&self) -> bool {
        let metadata = Self::metadata_bytes();
        self.0.domain.lock().is_ok_and(|d| {
            d.serial == 0
                && self.1 == 0
                && d.state == 0
                && d.moved_in == 0
                && d.remaining == self.3 - metadata
                && d.low_water == d.remaining
        })
    }
    pub(crate) fn new(limit: usize, retained: usize) -> Result<Self, BudgetExceeded> {
        // 当前 Rust 1.98 ArcInner 元数据；在 Arc::new 前预留，不声称 fallible Arc。
        let metadata = Self::metadata_bytes();
        let live = retained.checked_add(metadata).ok_or(BudgetExceeded)?;
        if live > limit {
            return Err(BudgetExceeded);
        }
        Ok(Self(
            Arc::new(Ledger {
                domain: Mutex::new(Domain {
                    serial: 0,
                    remaining: limit - live,
                    low_water: limit - live,
                    total_limit: limit,
                    moved_in: retained,
                    active_scopes: 0,
                    state: 0,
                }),
                live: AtomicUsize::new(live),
                peak: AtomicUsize::new(live),
                scratch_live: AtomicUsize::new(0),
                scratch_peak: AtomicUsize::new(0),
                metadata_live: AtomicUsize::new(0),
                metadata_peak: AtomicUsize::new(0),
                native_live: AtomicUsize::new(0),
                native_peak: AtomicUsize::new(0),
            }),
            0,
            0,
            limit,
        ))
    }
    pub(crate) fn check_current(&self) -> Result<(), BudgetExceeded> {
        let d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & BROKEN != 0 || u64::from(d.serial) != self.1 {
            return Err(BudgetExceeded);
        }
        Ok(())
    }
    pub(crate) fn total_limit(&self) -> Result<usize, BudgetExceeded> {
        self.check_current()?;
        Ok(self
            .0
            .domain
            .lock()
            .map_err(|_| BudgetExceeded)?
            .total_limit)
    }
    /// #1488 冻结合同同时限制总容量和本请求增量；旧费用不能再次扣A。
    pub(crate) fn bind_frozen(
        &self,
        request: u64,
        total: usize,
        legacy: usize,
        old_pool: usize,
    ) -> Result<Self, BudgetExceeded> {
        let allowance = total
            .checked_sub(legacy)
            .and_then(|n| n.checked_sub(old_pool))
            .ok_or(BudgetExceeded)?;
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & BROKEN != 0 || self.1 != u64::from(d.serial) || request == 0 {
            return Err(BudgetExceeded);
        }
        if self.2 == 0 {
            if d.state & (BROKEN | BOUND) != 0 || self.3 != allowance {
                return Err(BudgetExceeded);
            }
            d.state |= BOUND;
            return Ok(Self(self.0.clone(), self.1, request, allowance));
        }
        if self.2 == request {
            return if self.3 == allowance {
                Ok(self.clone())
            } else {
                Err(BudgetExceeded)
            };
        }
        let live = self.live();
        if request < self.2 || d.active_scopes != 0 || live > old_pool {
            return Err(BudgetExceeded);
        }
        // old_pool中剩余的独立旧池与legacy仍冻结：此父最多占L+A。
        // checked的L<=O同时证明(L+A)+(O-L)+legacy == total。
        let total_limit = live.checked_add(allowance).ok_or(BudgetExceeded)?;
        let serial = d.serial.checked_add(1).ok_or(BudgetExceeded)?;
        d.serial = serial;
        d.total_limit = total_limit;
        d.remaining = allowance;
        d.low_water = allowance;
        Ok(Self(self.0.clone(), u64::from(serial), request, allowance))
    }
    /// 工作域和新块一起预检；预算拒绝不能先接纳原件或改变峰值。
    fn preflight_codec_vec<T>(
        &self,
        v: &newchan_rust::s_content::CodecVec<T>,
        additional: usize,
    ) -> Result<(), Failure> {
        self.check_current()?;
        let parent: Arc<dyn ScratchParent> = self.0.clone();
        if v.is_native_bound() && !v.native_parent_matches(&parent) {
            return Err(Failure::Invalid("Native原件来自外来父账".into()));
        }
        if !v.is_native_bound() && !v.can_adopt_native() {
            return Err(Failure::Invalid("Native不接其它allocator容器".into()));
        }
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        let growth = if needed > v.capacity() {
            needed
                .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
                .max(4)
                .checked_mul(size_of::<T>())
                .ok_or(BudgetExceeded)?
        } else {
            0
        };
        let bytes = growth
            .checked_add(CodecWork::metadata_bytes())
            .ok_or(BudgetExceeded)?;
        let d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if bytes > d.remaining
            || self
                .live()
                .checked_add(bytes)
                .filter(|n| *n <= d.total_limit)
                .is_none()
            || (!v.is_native_bound()
                && v.capacity()
                    .checked_mul(size_of::<T>())
                    .filter(|n| *n <= d.moved_in)
                    .is_none())
        {
            return Err(BudgetExceeded.into());
        }
        Ok(())
    }
    pub(crate) fn same_parent(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn bind_or_handoff(
        &self,
        request: u64,
        allowance: usize,
    ) -> Result<Self, BudgetExceeded> {
        if self.2 == 0 {
            self.bind_initial(request, allowance)
        } else {
            self.handoff(request, allowance)
        }
    }
    pub(crate) fn bind_initial(
        &self,
        request: u64,
        allowance: usize,
    ) -> Result<Self, BudgetExceeded> {
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if u64::from(d.serial) != self.1
            || d.state & (BROKEN | BOUND) != 0
            || self.3 != allowance
            || request == 0
        {
            return Err(BudgetExceeded);
        }
        d.state |= BOUND;
        Ok(Self(self.0.clone(), self.1, request, allowance))
    }
    /// #1488：同父账接纳新请求，只开启新的增量计数；累计观察值和旧许可不动。
    pub(crate) fn handoff(&self, request: u64, allowance: usize) -> Result<Self, BudgetExceeded> {
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & BROKEN != 0 || self.1 != u64::from(d.serial) || self.2 == 0 || request == 0 {
            return Err(BudgetExceeded);
        }
        if self.2 == request {
            return if self.3 == allowance {
                Ok(self.clone())
            } else {
                Err(BudgetExceeded)
            };
        }
        if request < self.2 || d.active_scopes != 0 {
            return Err(BudgetExceeded);
        }
        let total_limit = self.live().checked_add(allowance).ok_or(BudgetExceeded)?;
        let serial = d.serial.checked_add(1).ok_or(BudgetExceeded)?;
        d.serial = serial;
        d.total_limit = total_limit;
        // 这是新冻结域的增量读数；Ledger.live/peak及各分项历史高水位始终不重置。
        // 旧来源仍由每个不可复制许可携带，返还只动累计占用，不回补本域。
        d.remaining = allowance;
        d.low_water = allowance;
        Ok(Self(self.0.clone(), u64::from(serial), request, allowance))
    }
    pub(crate) fn matches_request(&self, request: u64, allowance: usize) -> bool {
        self.0.domain.lock().is_ok_and(|d| {
            d.state & BROKEN == 0
                && u64::from(d.serial) == self.1
                && self.2 == request
                && self.3 == allowance
        })
    }
    pub(crate) fn remaining(&self) -> usize {
        self.0
            .domain
            .lock()
            .ok()
            .filter(|d| u64::from(d.serial) == self.1)
            .map(|d| d.remaining)
            .unwrap_or(0)
    }
    pub(crate) fn request_peak(&self) -> usize {
        self.0.domain.lock().map_or(usize::MAX, |d| {
            if u64::from(d.serial) == self.1 {
                self.3 - d.low_water
            } else {
                usize::MAX
            }
        })
    }
    pub(crate) fn adopt(&self, bytes: usize) -> Result<Permit, BudgetExceeded> {
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if u64::from(d.serial) != self.1 || d.state & BROKEN != 0 {
            return Err(BudgetExceeded);
        }
        d.moved_in = d.moved_in.checked_sub(bytes).ok_or(BudgetExceeded)?;
        Ok(Permit {
            budget: self.clone(),
            bytes,
            origin: 0,
        })
    }
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Permit, BudgetExceeded> {
        let origin = self
            .0
            .reserve_total(bytes, Some(self.1))
            .map_err(|_| BudgetExceeded)?;
        Ok(Permit {
            budget: self.clone(),
            bytes,
            origin,
        })
    }
    pub(crate) fn live(&self) -> usize {
        self.0.live.load(Ordering::Acquire)
    }
    pub(crate) fn peak(&self) -> usize {
        self.0.peak.load(Ordering::Acquire)
    }
    pub(crate) fn codec_work(&self) -> Result<CodecWork, Failure> {
        let d = self
            .0
            .domain
            .lock()
            .map_err(|_| Failure::Invalid("父账锁损坏".into()))?;
        if u64::from(d.serial) != self.1 {
            return Err(Failure::Invalid("旧冻结域不可新建工作域".into()));
        }
        let allowance = self.3;
        drop(d);
        CodecWork::new_in_origin(self.0.clone(), allowance, self.1).map_err(Failure::from)
    }
    pub(crate) fn scratch_observation(&self) -> (usize, usize, usize, usize) {
        (
            self.0.scratch_live.load(Ordering::Acquire),
            self.0.scratch_peak.load(Ordering::Acquire),
            self.0.metadata_live.load(Ordering::Acquire),
            self.0.metadata_peak.load(Ordering::Acquire),
        )
    }
    pub(crate) fn adopt_codec_vec<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
    ) -> Result<(), Failure> {
        self.check_current()?;
        if v.is_native_bound() {
            let parent: Arc<dyn ScratchParent> = self.0.clone();
            return if v.native_parent_matches(&parent) {
                Ok(())
            } else {
                Err(Failure::Invalid("Native原件来自外来父账".into()))
            };
        }
        if !v.can_adopt_native() {
            return Err(Failure::Invalid("Native不接其它allocator容器".into()));
        }
        let bytes = v
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(BudgetExceeded)?;
        let permit =
            newchan_rust::s_content::capture_allocation::scratch::NativeReservation::adopt(
                self.0.clone(),
                bytes,
                self.1,
            )?;
        v.attach_native_reservation(permit)
            .map_err(|e| Failure::Invalid(e.to_string()))
    }
    pub(crate) fn codec_vec_grow<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
        additional: usize,
    ) -> Result<(), Failure> {
        self.preflight_codec_vec(v, additional)?;
        // 工作域元数据先准入。Plain原件的接纳由容器在新块准入/分配成功后提交。
        self.codec_work()?
            .run(|w| v.try_reserve_native_adopting(w, additional))
            .map_err(|e| match e {
                WorkError::Resource(e) => e.into(),
                WorkError::Operation(e) => Failure::Invalid(e.to_string()),
            })
    }
    pub(crate) fn empty<T: Default>(&self) -> Result<Reserved<T>, BudgetExceeded> {
        Ok(self.reserve(0)?.hold(T::default()))
    }
    pub(crate) fn string(&self, value: &str) -> Result<Reserved<String>, BudgetExceeded> {
        let permit = self.reserve(value.len())?;
        let mut out = String::new();
        out.try_reserve_exact(value.len())
            .map_err(|_| BudgetExceeded)?;
        out.push_str(value);
        Ok(permit.finish(out.capacity()).hold(out))
    }
    pub(crate) fn vec_grow<T>(
        &self,
        v: &mut Reserved<Vec<T>>,
        additional: usize,
    ) -> Result<(), BudgetExceeded> {
        self.grow_vec(&mut v.inner, &mut v._permit, additional)
    }
    pub(crate) fn grow_vec<T>(
        &self,
        v: &mut Vec<T>,
        fee: &mut Permit,
        additional: usize,
    ) -> Result<(), BudgetExceeded> {
        self.check_current()?;
        if !self.same_parent(&fee.budget) || fee.bytes != vec_bytes::<T>(v.capacity()) {
            return Err(BudgetExceeded);
        }
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        if needed <= v.capacity() {
            return Ok(());
        }
        let next = needed
            .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
            .max(4);
        let permit = self.reserve(vec_bytes::<T>(next))?;
        v.try_reserve_exact(next - v.len())
            .map_err(|_| BudgetExceeded)?;
        *fee = permit.finish(vec_bytes::<T>(v.capacity()));
        Ok(())
    }
    pub(crate) fn hash_room<K: Eq + Hash, V>(
        &self,
        m: &mut Reserved<HashMap<K, V>>,
        key: &K,
    ) -> Result<(), BudgetExceeded> {
        self.check_current()?;
        if !self.same_parent(&m._permit.budget)
            || m._permit.bytes != hash_slots::<K, V>(m.capacity())
        {
            return Err(BudgetExceeded);
        }
        if m.contains_key(key) || m.len() < m.capacity() {
            return Ok(());
        }
        let cap = hash_capacity(m.len().checked_add(1).ok_or(BudgetExceeded)?)?;
        let permit = self.reserve(hash_slots::<K, V>(cap))?;
        m.inner.try_reserve(1).map_err(|_| BudgetExceeded)?;
        m._permit = permit.finish(hash_slots::<K, V>(m.capacity()));
        Ok(())
    }
    pub(crate) fn tree_insert<V: Retained>(
        &self,
        m: &mut Tree<V>,
        k: Reserved<String>,
        v: V,
    ) -> Result<(), BudgetExceeded> {
        self.check_current()?;
        if !self.same_parent(&k._permit.budget)
            || !self.same_parent(&m.fees._permit.budget)
            || k._permit.origin != self.1
        {
            return Err(BudgetExceeded);
        }
        if let Some((old, _)) = m.entries.get_mut(k.as_str()) {
            *old = v;
            return Ok(());
        }
        let mut node = self.reserve(tree_growth::<String, (V, usize)>(m.entries.len()))?;
        let index = match m.fees.iter().position(Option::is_none) {
            Some(index) => index,
            None => {
                self.vec_grow(&mut m.fees, 1)?;
                m.fees.push(None);
                m.fees.len() - 1
            }
        };
        if node.origin != k._permit.origin {
            return Err(BudgetExceeded);
        }
        let (key, mut key_fee) = k.into_parts();
        m.entries.insert(key, (v, index));
        node = node.finish(tree_slots::<String, (V, usize)>(1));
        node.merge(&mut key_fee)?;
        m.fees[index] = Some(node);
        Ok(())
    }
}
impl<T> Reserved<T> {
    pub(crate) fn into_parts(self) -> (T, Permit) {
        (self.inner, self._permit)
    }
}
impl std::borrow::Borrow<str> for Reserved<String> {
    fn borrow(&self) -> &str {
        self.inner.as_str()
    }
}
impl<T: std::hash::Hash> std::hash::Hash for Reserved<T> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.inner.hash(h)
    }
}
impl<T: PartialEq> PartialEq for Reserved<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<T: Eq> Eq for Reserved<T> {}
impl<T: std::fmt::Debug> std::fmt::Debug for Reserved<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
impl<T: PartialEq> PartialEq<Vec<T>> for Reserved<Vec<T>> {
    fn eq(&self, other: &Vec<T>) -> bool {
        &self.inner == other
    }
}
impl<T> IntoIterator for Reserved<Vec<T>> {
    type Item = T;
    type IntoIter = HeldIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        HeldIter {
            values: self.inner.into_iter(),
            _permit: self._permit,
        }
    }
}
pub(crate) struct HeldIter<T> {
    values: std::vec::IntoIter<T>,
    _permit: Permit,
}
impl<T> Iterator for HeldIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.values.next()
    }
}
impl<'a, T> IntoIterator for &'a Reserved<Vec<T>> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Reserved<Vec<T>> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}
/// #1488：树节点/键许可在entries实际析构后释放；槽数组本身也先准入。
pub(crate) struct Tree<V> {
    entries: BTreeMap<String, (V, usize)>,
    fees: Reserved<Vec<Option<Permit>>>,
}
impl<V> Tree<V> {
    pub(crate) fn new(budget: &Budget) -> Result<Self, BudgetExceeded> {
        Ok(Self {
            entries: BTreeMap::new(),
            fees: budget.empty()?,
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.entries.iter().map(|(k, (v, _))| (k, v))
    }
    pub(crate) fn remove(&mut self, key: &str) {
        if let Some((k, (v, index))) = self.entries.remove_entry(key) {
            if self.entries.is_empty() {
                self.entries.clear();
            }
            drop((k, v));
            drop(self.fees[index].take());
        }
        if self.entries.is_empty() {
            // 最后一项移除连同许可槽也真正释放，保持原空树返还合同。
            let empty = self.fees._permit.split(0).expect("空许可");
            self.fees = empty.hold(Vec::new());
        }
    }
}
impl<V: Retained> Retained for Tree<V> {
    fn heap_bytes(&self) -> usize {
        sum([
            tree_slots::<String, (V, usize)>(self.entries.len()),
            self.entries
                .iter()
                .map(|(k, (v, _))| sum([k.heap_bytes(), v.heap_bytes()]))
                .sum(),
            vec_bytes::<Option<Permit>>(self.fees.capacity()),
        ])
    }
}

use newchan_rust::s_content::capture_allocation::scratch::{
    CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkError,
};
impl From<ScratchFailure> for Failure {
    fn from(e: ScratchFailure) -> Self {
        match e {
            ScratchFailure::Budget | ScratchFailure::Allocation => Self::BudgetExceeded,
            ScratchFailure::Ledger => Self::Invalid("Native codec 工作账损坏".into()),
        }
    }
}
impl<E: Into<Failure>> From<WorkError<E>> for Failure {
    fn from(e: WorkError<E>) -> Self {
        match e {
            WorkError::Resource(e) => e.into(),
            WorkError::Operation(e) => e.into(),
        }
    }
}
impl Ledger {
    fn reserve_total(&self, bytes: usize, expected: Option<u64>) -> Result<u64, ScratchFailure> {
        self.reserve_account(bytes, expected, None)
    }
    fn reserve_account(
        &self,
        bytes: usize,
        expected: Option<u64>,
        kind: Option<ScratchCharge>,
    ) -> Result<u64, ScratchFailure> {
        let mut d = self.domain.lock().map_err(|_| ScratchFailure::Ledger)?;
        if d.state & BROKEN != 0 || expected.is_some_and(|e| e != u64::from(d.serial)) {
            return Err(ScratchFailure::Ledger);
        }
        let next = d
            .remaining
            .checked_sub(bytes)
            .ok_or(ScratchFailure::Budget)?;
        let total = self
            .live
            .load(Ordering::Acquire)
            .checked_add(bytes)
            .filter(|n| *n <= d.total_limit)
            .ok_or(ScratchFailure::Budget)?;
        let part = kind
            .map(|k| {
                let (live, peak) = self.component(k);
                live.load(Ordering::Acquire)
                    .checked_add(bytes)
                    .map(|n| (live, peak, n))
                    .ok_or(ScratchFailure::Ledger)
            })
            .transpose()?;
        let active_scopes = if matches!(kind, Some(ScratchCharge::Metadata)) {
            d.active_scopes
                .checked_add(1)
                .ok_or(ScratchFailure::Ledger)?
        } else {
            d.active_scopes
        };
        d.active_scopes = active_scopes;
        d.remaining = next;
        d.low_water = d.low_water.min(next);
        if let Some((live, peak, n)) = part {
            live.store(n, Ordering::Release);
            peak.fetch_max(n, Ordering::Relaxed);
        }
        self.live.store(total, Ordering::Release);
        self.peak.fetch_max(total, Ordering::Relaxed);
        Ok(u64::from(d.serial))
    }
    fn release_total(&self, bytes: usize, origin: u64) -> Result<(), ScratchFailure> {
        let result = self.release_account(bytes, origin, None);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn release_account(
        &self,
        bytes: usize,
        origin: u64,
        kind: Option<ScratchCharge>,
    ) -> Result<(), ScratchFailure> {
        let mut d = self.domain.lock().map_err(|_| ScratchFailure::Ledger)?;
        let total = self
            .live
            .load(Ordering::Acquire)
            .checked_sub(bytes)
            .ok_or(ScratchFailure::Ledger)?;
        if origin > u64::from(d.serial) {
            return Err(ScratchFailure::Ledger);
        }
        let current = if origin == u64::from(d.serial) {
            d.remaining
                .checked_add(bytes)
                .ok_or(ScratchFailure::Ledger)?
        } else {
            d.remaining
        };
        let part = kind
            .map(|k| {
                let (live, _) = self.component(k);
                live.load(Ordering::Acquire)
                    .checked_sub(bytes)
                    .map(|n| (live, n))
                    .ok_or(ScratchFailure::Ledger)
            })
            .transpose()?;
        d.remaining = current;
        self.live.store(total, Ordering::Release);
        if let Some((live, n)) = part {
            live.store(n, Ordering::Release);
        }
        Ok(())
    }
    fn component(&self, kind: ScratchCharge) -> (&AtomicUsize, &AtomicUsize) {
        match kind {
            ScratchCharge::Layout => (&self.scratch_live, &self.scratch_peak),
            ScratchCharge::Metadata => (&self.metadata_live, &self.metadata_peak),
            ScratchCharge::Native => (&self.native_live, &self.native_peak),
        }
    }
}
impl ScratchParent for Ledger {
    fn origin(&self) -> u64 {
        self.domain.lock().map_or(u64::MAX, |d| u64::from(d.serial))
    }
    fn adopt_native(&self, bytes: usize, expected: u64) -> Result<u64, ScratchFailure> {
        let mut d = self.domain.lock().map_err(|_| ScratchFailure::Ledger)?;
        if d.state & BROKEN != 0 || expected != u64::from(d.serial) {
            return Err(ScratchFailure::Ledger);
        }
        let moved_in = d
            .moved_in
            .checked_sub(bytes)
            .ok_or(ScratchFailure::Ledger)?;
        let native = self
            .native_live
            .load(Ordering::Acquire)
            .checked_add(bytes)
            .ok_or(ScratchFailure::Ledger)?;
        d.moved_in = moved_in;
        self.native_live.store(native, Ordering::Release);
        self.native_peak.fetch_max(native, Ordering::Relaxed);
        Ok(0)
    }
    fn close_scope(&self, origin: u64) -> Result<(), ScratchFailure> {
        let mut d = self.domain.lock().map_err(|_| ScratchFailure::Ledger)?;
        if u64::from(d.serial) != origin {
            return Err(ScratchFailure::Ledger);
        }
        d.active_scopes = d
            .active_scopes
            .checked_sub(1)
            .ok_or(ScratchFailure::Ledger)?;
        Ok(())
    }
    fn health(&self) -> Result<(), ScratchFailure> {
        let d = self.domain.lock().map_err(|_| ScratchFailure::Ledger)?;
        if d.state & BROKEN != 0 {
            Err(ScratchFailure::Ledger)
        } else {
            Ok(())
        }
    }
    fn invalidate(&self) {
        if let Ok(mut d) = self.domain.lock() {
            d.state |= BROKEN;
        }
    }
    fn reserve_in_origin(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        expected: u64,
    ) -> Result<u64, ScratchFailure> {
        self.reserve_account(bytes, Some(expected), Some(kind))
    }
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.reserve_account(bytes, None, Some(kind))
    }
    fn release(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        origin: u64,
    ) -> Result<(), ScratchFailure> {
        let result = self.release_account(bytes, origin, Some(kind));
        if result.is_err() {
            self.invalidate();
        }
        result
    }
}
/// 固定 std SwissTable：小表 4/8 桶只空一槽，>=16 桶的装载率 7/8；控制组最多16字节。
/// 返回下一次实际可用 capacity 的上界，再沿既有 hash_slots 公式结算。
pub(crate) fn hash_capacity(needed: usize) -> Result<usize, BudgetExceeded> {
    if needed == 0 {
        return Ok(0);
    }
    if needed <= 3 {
        return Ok(3);
    }
    if needed <= 7 {
        return Ok(7);
    }
    let buckets = needed
        .checked_mul(8)
        .and_then(|n| n.checked_add(6))
        .map(|n| n / 7)
        .and_then(usize::checked_next_power_of_two)
        .ok_or(BudgetExceeded)?;
    Ok(buckets / 8 * 7)
}
/// std BTree B=6；每层至多一次 split 和一个新根。最小分支数>=2，故 log2(n)+2 覆盖节点数。
/// tree_slots 保留公式仍每项一个完整节点；额外 split 仅在许可活期计入峰值。
pub(crate) fn tree_growth<K, V>(len: usize) -> usize {
    let height = if len == 0 {
        0
    } else {
        usize::BITS as usize - len.leading_zeros() as usize
    };
    tree_slots::<K, V>(height.saturating_add(2))
}

/// 无分配词法计数，只产生内存上界；仍调用原 strict_json/原业务谓词，不赋予语法合法资格。
/// 每个 value token 计一个可能数组槽（包含对象值/键会高估）；Vec 扩容峰值<=3*max(4,2*n)。
/// 每个字段计一完整BTree节点；解码UTF-8不长于原字符串体；serde scratch最多下一2次幂块共存。
#[derive(Default, Debug)]
pub(crate) struct JsonShape {
    pub(crate) text: usize,
    pub(crate) values: usize,
    pub(crate) fields: usize,
    pub(crate) arrays: usize,
    pub(crate) depth: usize,
    longest: usize,
}
impl JsonShape {
    pub(crate) fn scan(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        let mut i = 0;
        let mut depth = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    let start = i + 1;
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        if bytes[i] == b'\\' {
                            i = i.saturating_add(1);
                        }
                        i = i.saturating_add(1);
                    }
                    let n = i.saturating_sub(start);
                    out.text = out.text.saturating_add(n);
                    out.longest = out.longest.max(n);
                    out.values = out.values.saturating_add(1);
                }
                b'{' | b'[' => {
                    out.values = out.values.saturating_add(1);
                    out.arrays += usize::from(bytes[i] == b'[');
                    depth += 1;
                    out.depth = out.depth.max(depth);
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                b':' => out.fields += 1,
                b',' | b' ' | b'\n' | b'\r' | b'\t' => {}
                _ => {
                    out.values += 1;
                    while i + 1 < bytes.len() && !b",]} \r\n\t:".contains(&bytes[i + 1]) {
                        i += 1;
                    }
                }
            }
            i = i.saturating_add(1);
        }
        out
    }
    pub(crate) fn of_value(value: &serde_json::Value) -> Self {
        fn visit(value: &serde_json::Value, depth: usize, out: &mut JsonShape) {
            out.values = out.values.saturating_add(1);
            out.depth = out.depth.max(depth);
            match value {
                serde_json::Value::String(text) => {
                    out.text = out.text.saturating_add(text.len());
                    out.longest = out.longest.max(text.len());
                }
                serde_json::Value::Array(items) => {
                    out.arrays = out.arrays.saturating_add(1);
                    for item in items {
                        visit(item, depth.saturating_add(1), out);
                    }
                }
                serde_json::Value::Object(fields) => {
                    out.fields = out.fields.saturating_add(fields.len());
                    for (key, value) in fields {
                        out.values = out.values.saturating_add(1);
                        out.text = out.text.saturating_add(key.len());
                        out.longest = out.longest.max(key.len());
                        visit(value, depth.saturating_add(1), out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Self::default();
        visit(value, 0, &mut out);
        out
    }
    pub(crate) fn parse_peak(&self) -> usize {
        sum([
            self.text,
            // 各空数组零分配；把4槽初次增长也按所有数组计入，比实际宽。
            vec_bytes::<serde_json::Value>(
                self.values
                    .saturating_mul(6)
                    .saturating_add(self.arrays.saturating_mul(12)),
            ),
            tree_slots::<String, serde_json::Value>(self.fields),
            self.longest
                .checked_next_power_of_two()
                .unwrap_or(usize::MAX)
                .saturating_mul(3),
        ])
    }
    /// canonical_json 的每层 keys/parts、join和format共存。6是单字节转义的最长UTF-8输出。
    pub(crate) fn canonical_peak(&self) -> usize {
        let output = sum([
            self.text.saturating_mul(6),
            self.values.saturating_mul(32),
            self.fields.saturating_mul(3),
        ]);
        sum([
            output
                .saturating_mul(self.depth.saturating_add(1))
                .saturating_mul(3),
            self.values
                .saturating_mul(size_of::<String>() + size_of::<&String>())
                .saturating_mul(2),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_permits_release_and_growth_keeps_old_block_until_reserve() {
        let b = Budget::new(4096, 0).unwrap();
        let base = b.live();
        let p = b.reserve(1000).unwrap();
        assert_eq!(b.live(), base + 1000);
        assert!(b.reserve(4096).is_err());
        drop(p);
        assert_eq!(b.live(), base);
        let mut v = b.empty::<Vec<u64>>().unwrap();
        b.vec_grow(&mut v, 4).unwrap();
        v.extend([1, 2, 3, 4]);
        let old = v.capacity() * size_of::<u64>();
        let before = b.live();
        b.vec_grow(&mut v, 1).unwrap();
        assert_eq!(b.live(), base + v.capacity() * size_of::<u64>());
        assert!(b.peak() >= before + v.capacity() * size_of::<u64>());
        let mut blocked = vec![1u64, 2, 3, 4];
        let old_block = vec_bytes::<u64>(blocked.capacity());
        let tight = Budget::new(base + old_block + vec_bytes::<u64>(8) - 1, old_block).unwrap();
        let capacity = blocked.capacity();
        let mut blocked = tight.adopt(old_block).unwrap().hold(blocked);
        assert!(tight.vec_grow(&mut blocked, 1).is_err());
        assert_eq!(blocked.capacity(), capacity);
        assert_eq!(&blocked[..], &[1, 2, 3, 4], "新块被拒绝时旧Vec必须完整保留");
        assert_eq!(&v[..], &[1, 2, 3, 4]);
        assert!(v.capacity() * size_of::<u64>() > old);
        assert!(matches!(
            Budget::new(usize::MAX, usize::MAX),
            Err(BudgetExceeded)
        ));
        assert!(b.reserve(usize::MAX).is_err());
        struct BeforeRelease(Budget, usize);
        impl Drop for BeforeRelease {
            fn drop(&mut self) {
                assert_eq!(self.0.live(), self.1, "实际工作集析构前许可仍须占账");
            }
        }
        let live = b.live();
        let permit = b.reserve(100).unwrap();
        let held = permit.hold(BeforeRelease(b.clone(), live + 100));
        drop(held);
        assert_eq!(b.live(), live);
    }
    #[test]
    fn native_hash_bucket_bound_covers_actual_growth_and_json_shapes_cover_heap() {
        let mut map = HashMap::<String, u64>::new();
        for i in 0..1024 {
            let need = map.len() + 1;
            if need > map.capacity() {
                let upper = hash_capacity(need).unwrap();
                map.try_reserve(1).unwrap();
                assert!(
                    map.capacity() <= upper,
                    "{need}: {} > {upper}",
                    map.capacity()
                );
            }
            map.insert(i.to_string(), i);
        }
        for bytes in [
            br#"{"a":[1,2,3,4,5,6,7,8,9],"b":{"x":"\u4e2d\u6587\n\uD83D\uDE00"}}"#.as_slice(),
            br#"[[],{},[[],[true,false,null]],"abcdefghijklmnopqrstuvwxyz"]"#.as_slice(),
        ] {
            let shape = JsonShape::scan(bytes);
            let value = newchan_rust::session_protocol::strict_json(bytes).unwrap();
            assert!(value.heap_bytes() <= shape.parse_peak(), "{shape:?}");
            let canonical = newchan_rust::session_protocol::canonical_json(&value);
            assert!(canonical.capacity() <= shape.canonical_peak());
        }
    }
}

#[cfg(test)]
mod r8_native_admission_tests {
    use super::*;
    use crate::v3::{audit_store::CodecRead, history::HashReader};
    use newchan_rust::s_content::{self as sc, NodeSource};

    #[test]
    fn r8_native_frame_body_release_and_preallocation_refusal() {
        let mut nodes = sc::MemoryNodes::new();
        let body = vec![b'x'; 4096];
        let digest = sc::frame(sc::tag::BYTES_CHUNK, &body);
        sc::NodeSink::put(&mut nodes, digest, sc::tag::BYTES_CHUNK, &body).unwrap();
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let budget = Budget::new(65536, 0).unwrap();
        let base = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let mut reader = HashReader::captured_for_audit(&capture, None)
                    .unwrap()
                    .with_codec_work(scope, &budget);
                let before = budget.live();
                let frame = reader.checked_read_scoped(&digest)?.unwrap();
                assert_eq!(frame.digest(), &digest);
                assert_eq!(budget.live(), before + body.len());
                drop(frame);
                assert_eq!(budget.live(), before, "先释放正文，随后归还父账");
                assert_eq!(budget.native_observation(), (0, body.len()));
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(budget.live(), base);
        let tight = Budget::new(base + CodecWork::metadata_bytes() + body.len() - 1, 0).unwrap();
        let result = tight.codec_work().unwrap().run(|scope| {
            let mut reader = HashReader::captured_for_audit(&capture, None)
                .unwrap()
                .with_codec_work(scope, &tight);
            let before = capture.retention()["capture_reads"].as_u64().unwrap();
            let result = reader.checked_read_scoped(&digest);
            assert!(result.is_err());
            assert_eq!(
                capture.retention()["capture_reads"].as_u64().unwrap(),
                before,
                "额度在实际复制/读取前拒绝"
            );
            result.map(|_| ())
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(tight.live(), base);
        eprintln!(
            "R8_NATIVE_BODY base={base} bytes={} peak={} parts={:?}",
            body.len(),
            budget.peak(),
            budget.native_observation()
        );
    }

    #[test]
    fn r8_native_local_material_memo_consumes_work_and_releases() {
        struct Sink<'a> {
            work: Option<&'a sc::capture_allocation::scratch::WorkScope>,
            calls: usize,
        }
        impl sc::NodeSink for Sink<'_> {
            fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
                self.work
            }
            fn put(
                &mut self,
                digest: sc::NodeDigest,
                tag: u8,
                body: &[u8],
            ) -> Result<(), sc::CodecError> {
                assert_eq!(sc::frame(tag, body), digest);
                self.calls += 1;
                Ok(())
            }
        }
        let value = sc::Value::Array(vec![sc::Value::String("重复正文".repeat(900)); 3]);
        let mut plain = Sink {
            work: None,
            calls: 0,
        };
        let expected = sc::write_value_native_local_for_testonly(&mut plain, &value).unwrap();
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let base = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let mut sink = Sink {
                    work: Some(scope),
                    calls: 0,
                };
                let actual = sc::write_value_native_local_for_testonly(&mut sink, &value)?;
                assert_eq!(actual, expected);
                assert_eq!(sink.calls, plain.calls, "完整正文 memo 保持真实 put 次数");
                assert!(scope.snapshot().attempts > 0);
                assert_eq!(scope.snapshot().live, 0, "memo 和正文随这次编码释放");
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(budget.live(), base);
        let tight = Budget::new(base + CodecWork::metadata_bytes() + 1, 0).unwrap();
        let result = tight.codec_work().unwrap().run(|scope| {
            let mut sink = Sink {
                work: Some(scope),
                calls: 0,
            };
            let result = sc::write_value_native_local_for_testonly(&mut sink, &value);
            assert_eq!(sink.calls, 0, "真实 put 前拒绝工作集分配");
            result
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(tight.live(), base);
        eprintln!(
            "R8_NATIVE_MEMO calls={} scratch={:?} parent_peak={}",
            plain.calls,
            budget.scratch_observation(),
            budget.peak()
        );
    }

    #[test]
    fn r8_native_business_map_growth_uses_parent_ledger() {
        use crate::v3::audit_store::NativeAdmission;
        use sc::capture_allocation::audit_json::BusinessMap;
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let admission = NativeAdmission(Some(scope));
                let mut map = BusinessMap::<i64, i64>::plain();
                for n in 0..200 {
                    admission.native_map_room(&mut map)?;
                    let capacity = map.capacity();
                    map.insert(n, n * 2)?;
                    assert_eq!(map.capacity(), capacity, "实际插入不再暗中扩桶");
                }
                assert_eq!(map.len(), 200);
                assert!(budget.native_observation().0 > 0);
                assert_eq!(scope.snapshot().live, 0, "Plain map 不伪装成 Owned/scratch");
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        let base = Budget::new(4096, 0).unwrap().live();
        let tight = Budget::new(base + CodecWork::metadata_bytes() + 1, 0).unwrap();
        let result = tight.codec_work().unwrap().run(|scope| {
            let mut map = BusinessMap::<i64, i64>::plain();
            let result = NativeAdmission(Some(scope)).native_map_room(&mut map);
            assert_eq!(map.capacity(), 0);
            assert!(map.is_empty());
            result
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(tight.live(), base);
        eprintln!(
            "R8_NATIVE_MAP retained_bound_peak={:?}",
            budget.native_observation()
        );
    }
}

#[cfg(test)]
// NOT_RUN：本轮禁止编译及执行Rust测试；以下均为待运行回归锁。
mod cross_request_budget_tests {
    use super::*;
    use newchan_rust::s_content::CodecVec;

    #[test]
    fn retained_scratch_layout_drops_in_old_origin_after_scope_closes() {
        let first = Budget::new(4096, 0).unwrap();
        let first = first.bind_initial(35, 4096).unwrap();
        let base = first.live();
        let mut bytes = first
            .codec_work()
            .unwrap()
            .run(|w| {
                let mut bytes = w.bytes();
                bytes.try_extend_bytes(b"kept-layout")?;
                Ok::<_, newchan_rust::s_content::CodecError>(bytes)
            })
            .unwrap();
        let peak = first.peak();
        let next = first.handoff(36, 64).unwrap();
        let all = next.reserve(64).unwrap();
        assert!(bytes.try_push(b'x').is_err());
        assert_eq!(&*bytes, b"kept-layout");
        drop(bytes);
        assert_eq!(next.live(), base + 64);
        assert_eq!(next.remaining(), 0);
        assert!(next.peak() >= peak);
        drop(all);
        assert_eq!(next.live(), base);
    }

    #[test]
    fn native_reservation_split_merge_rejects_foreign_and_new_origin() {
        let first = Budget::new(4096, 0).unwrap();
        let first = first.bind_initial(40, 4096).unwrap();
        let mut kept = first
            .codec_work()
            .unwrap()
            .run(|w| w.reserve_native(96))
            .unwrap();
        let mut part = kept.split(32).unwrap();
        kept.merge(&mut part).unwrap();
        assert!(part.split(1).is_err());
        let second = first.handoff(41, 4096).unwrap();
        let mut new = second
            .codec_work()
            .unwrap()
            .run(|w| w.reserve_native(32))
            .unwrap();
        let remaining = second.remaining();
        assert!(kept.merge(&mut new).is_err());
        let foreign = Budget::new(4096, 0).unwrap();
        let mut other = foreign
            .codec_work()
            .unwrap()
            .run(|w| w.reserve_native(32))
            .unwrap();
        assert!(kept.merge(&mut other).is_err());
        drop(kept);
        assert_eq!(second.remaining(), remaining);
        drop(new);
        assert_eq!(second.remaining(), 4096);
    }

    #[test]
    fn old_release_does_not_refill_allowance_with_real_native_payload() {
        let old = Budget::new(4096, 0).unwrap();
        let old = old.bind_initial(1, 4096).unwrap();
        let base = old.live();
        let bytes = old
            .codec_work()
            .unwrap()
            .run(|w| {
                let mut bytes = CodecVec::default();
                bytes.try_reserve_native(w, 128)?;
                bytes.try_extend_bytes(b"original")?;
                Ok::<_, newchan_rust::s_content::CodecError>(bytes)
            })
            .unwrap();
        let live = old.live();
        let peak = old.peak();
        let current = old.handoff(2, 64).unwrap();
        assert!(old.same_parent(&current));
        assert_eq!((current.live(), current.peak()), (live, peak));
        let current_fee = current.reserve(64).unwrap();
        drop(bytes);
        assert_eq!(current.live(), base + 64);
        assert_eq!(current.remaining(), 0);
        assert!(current.reserve(1).is_err());
        assert!(old.reserve(1).is_err());
        drop(current_fee);
        assert_eq!(current.remaining(), 64);
        assert_eq!(current.live(), base);
        assert!(current.peak() >= peak);
    }

    #[test]
    fn native_growth_across_two_requests_keeps_payload_and_charge_origin() {
        let first = Budget::new(4096, 0).unwrap();
        let first = first.bind_initial(10, 4096).unwrap();
        let base = first.live();
        let mut values = first
            .codec_work()
            .unwrap()
            .run(|w| {
                let mut values = CodecVec::default();
                for n in 0u64..4 {
                    values.try_push_native(n, w)?;
                }
                Ok::<_, newchan_rust::s_content::CodecError>(values)
            })
            .unwrap();
        let second = first.handoff(11, 4096).unwrap();
        second
            .codec_work()
            .unwrap()
            .run(|w| values.try_push_native(4, w))
            .unwrap();
        assert_eq!(&*values, &[0, 1, 2, 3, 4]);
        assert_eq!(second.live(), base + values.capacity() * size_of::<u64>());
        let before = second.peak();
        let third = second.handoff(12, 32).unwrap();
        drop(values);
        assert_eq!(third.live(), base);
        assert_eq!(third.remaining(), 32);
        assert_eq!(third.peak(), before);
    }

    #[test]
    fn cross_request_denied_growth_preserves_old_payload_and_origin() {
        let first = Budget::new(4096, 0).unwrap();
        let first = first.bind_initial(15, 4096).unwrap();
        let mut values = first
            .codec_work()
            .unwrap()
            .run(|w| {
                let mut values = CodecVec::default();
                for n in 0u64..4 {
                    values.try_push_native(n, w)?;
                }
                Ok::<_, newchan_rust::s_content::CodecError>(values)
            })
            .unwrap();
        let live = first.live();
        let allowance = CodecWork::metadata_bytes() + 63;
        let next = first.handoff(16, allowance).unwrap();
        let result = next
            .codec_work()
            .unwrap()
            .run(|w| values.try_push_native(4, w));
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(&*values, &[0, 1, 2, 3]);
        assert_eq!(values.capacity(), 4);
        assert_eq!(next.live(), live);
        assert_eq!(next.remaining(), allowance);
        drop(values);
        assert_eq!(next.remaining(), allowance);
    }

    #[test]
    fn handoff_active_scope_overflow_and_changed_contract_are_atomic() {
        let budget = Budget::new(4096, 0).unwrap();
        let budget = budget.bind_initial(20, 4096).unwrap();
        let work = budget.codec_work().unwrap();
        let before = (budget.live(), budget.peak(), budget.remaining());
        assert!(budget.handoff(21, 4096).is_err());
        assert_eq!((budget.live(), budget.peak(), budget.remaining()), before);
        drop(work);
        let before = (budget.live(), budget.peak(), budget.remaining());
        assert!(budget.handoff(21, usize::MAX).is_err());
        assert!(budget.handoff(20, 4095).is_err());
        assert_eq!((budget.live(), budget.peak(), budget.remaining()), before);
        let all = budget.reserve(budget.remaining()).unwrap();
        let full = (budget.live(), budget.peak());
        assert!(budget.reserve(1).is_err());
        assert_eq!((budget.live(), budget.peak()), full);
        drop(all);
    }

    #[test]
    fn permit_split_merge_foreign_source_and_duplicate_consumption_reject() {
        let a = Budget::new(4096, 0).unwrap();
        let b = Budget::new(4096, 0).unwrap();
        let base = a.live();
        let mut whole = a.reserve(96).unwrap();
        let mut part = whole.split(32).unwrap();
        let mut foreign = b.reserve(32).unwrap();
        assert!(whole.merge(&mut foreign).is_err());
        whole.merge(&mut part).unwrap();
        assert!(part.split(1).is_err());
        assert_eq!(a.live(), base + 96);
        drop(part);
        assert_eq!(a.live(), base + 96);
        drop(whole);
        assert_eq!(a.live(), base);
    }

    #[test]
    fn old_tree_remove_and_drop_return_only_their_own_origin() {
        let old = Budget::new(65536, 0).unwrap();
        let old = old.bind_initial(30, 65536).unwrap();
        let base = old.live();
        let mut tree = Tree::new(&old).unwrap();
        old.tree_insert(&mut tree, old.string("old-key").unwrap(), (1i64, 2i64))
            .unwrap();
        let next = old.handoff(31, 64).unwrap();
        let full = next.reserve(64).unwrap();
        tree.remove("old-key");
        assert_eq!(next.remaining(), 0);
        assert_eq!(next.live(), base + 64);
        drop(tree);
        drop(full);
        assert_eq!(next.live(), base);
    }

    #[test]
    fn initial_original_fee_can_be_adopted_only_once() {
        let mut values = CodecVec::from_plain(vec![1u64, 2, 3, 4]);
        let cost = values.capacity() * size_of::<u64>();
        let budget = Budget::new(4096, cost).unwrap();
        let live = budget.live();
        budget.adopt_codec_vec(&mut values).unwrap();
        assert_eq!(budget.live(), live);
        let foreign = Budget::new(4096, cost).unwrap();
        let foreign_before = (foreign.live(), foreign.peak());
        assert!(foreign.adopt_codec_vec(&mut values).is_err());
        assert_eq!((foreign.live(), foreign.peak()), foreign_before);
        assert_eq!(&*values, &[1, 2, 3, 4]);
        assert!(budget.adopt(cost).is_err());
        drop(values);
        assert_eq!(budget.live(), live - cost);
    }
}

#[cfg(test)]
// #1488 v2反例锁。NOT_RUN：本轮未获准编译或执行Rust。
mod lifecycle_repair_v2_tests {
    use super::*;
    use newchan_rust::s_content::{
        self as sc, capture_allocation::audit_json::BusinessMap, CodecVec,
    };

    #[test]
    fn frozen_increment_accepts_700_old_and_20_new_with_200_allowance() {
        let first = Budget::new(900, 0).unwrap().bind_initial(100, 900).unwrap();
        let metadata = Budget::metadata_bytes();
        let old_candidate = first
            .reserve(650 - metadata)
            .unwrap()
            .hold(vec![7u8; 650 - metadata]);
        let old_raw = first.reserve(50).unwrap().hold(vec![8u8; 50]);
        assert_eq!(first.live(), 700);
        let next = first.bind_frozen(101, 1000, 100, 700).unwrap();
        assert_eq!(next.remaining(), 200);
        let new = next.reserve(20).unwrap().hold(vec![9u8; 20]);
        assert_eq!(next.live() + 100, 820);
        assert_eq!(next.remaining(), 180);
        drop(old_candidate);
        drop(old_raw);
        assert_eq!(next.remaining(), 180, "旧来源返还不能增发当前A");
        drop(new);
        assert_eq!(next.remaining(), 200);
    }

    #[test]
    fn frozen_old_pool_must_include_actual_parent_metadata_before_handoff() {
        let first = Budget::new(4096, 0)
            .unwrap()
            .bind_initial(110, 4096)
            .unwrap();
        let bytes = first.reserve(300).unwrap().hold(vec![1u8; 300]);
        let before = (first.live(), first.peak(), first.remaining());
        assert!(first.bind_frozen(111, 4096, 0, 300).is_err());
        assert_eq!((first.live(), first.peak(), first.remaining()), before);
        assert_eq!(&**bytes, &[1u8; 300]);
        let next = first.bind_frozen(111, 4096, 0, before.0).unwrap();
        let all = next.reserve(next.remaining()).unwrap();
        assert_eq!(next.live(), 4096);
        let full = (next.live(), next.peak(), next.remaining());
        assert!(next.reserve(1).is_err());
        assert_eq!((next.live(), next.peak(), next.remaining()), full);
        drop(all);
        drop(bytes);
    }

    #[test]
    fn expired_domain_cannot_adopt_plain_vec_even_without_growth() {
        let mut values = CodecVec::from_plain(vec![1u64, 2, 3]);
        let bytes = values.capacity() * size_of::<u64>();
        let first = Budget::new(4096, bytes)
            .unwrap()
            .bind_initial(120, 4096)
            .unwrap();
        let next = first.handoff(121, 4096).unwrap();
        let before = (
            next.live(),
            next.peak(),
            next.remaining(),
            next.0.domain.lock().unwrap().moved_in,
        );
        let pointer = values.as_ptr();
        assert!(first.codec_vec_grow(&mut values, 0).is_err());
        assert!(first.adopt_codec_vec(&mut values).is_err());
        assert!(!values.is_native_bound());
        assert_eq!(values.as_ptr(), pointer);
        assert_eq!(&*values, &[1, 2, 3]);
        assert_eq!(
            (
                next.live(),
                next.peak(),
                next.remaining(),
                next.0.domain.lock().unwrap().moved_in
            ),
            before
        );
        next.codec_vec_grow(&mut values, 0).unwrap();
        assert!(values.is_native_bound());
        assert_eq!(values.as_ptr(), pointer);
    }

    #[test]
    fn insufficient_scope_metadata_keeps_plain_vec_and_original_fee_unchanged() {
        let mut values = CodecVec::from_plain(vec![4u64, 5, 6]);
        let bytes = values.capacity() * size_of::<u64>();
        let limit = Budget::metadata_bytes() + bytes + CodecWork::metadata_bytes() - 1;
        let budget = Budget::new(limit, bytes).unwrap();
        let before = (
            budget.live(),
            budget.peak(),
            budget.remaining(),
            budget.0.domain.lock().unwrap().moved_in,
        );
        let pointer = values.as_ptr();
        let capacity = values.capacity();
        assert!(budget.codec_vec_grow(&mut values, 0).is_err());
        assert_eq!(
            (values.as_ptr(), values.capacity(), values.is_native_bound()),
            (pointer, capacity, false)
        );
        assert_eq!(&*values, &[4, 5, 6]);
        assert_eq!(
            (
                budget.live(),
                budget.peak(),
                budget.remaining(),
                budget.0.domain.lock().unwrap().moved_in
            ),
            before
        );
        budget.adopt_codec_vec(&mut values).unwrap();
        drop(values);
        assert_eq!(budget.live(), Budget::metadata_bytes());
    }

    #[test]
    fn expired_budget_rejects_spare_vec_hash_and_tree_replacement() {
        let first = Budget::new(65536, 0)
            .unwrap()
            .bind_initial(130, 65536)
            .unwrap();
        let mut values = first.empty::<Vec<u64>>().unwrap();
        first.vec_grow(&mut values, 4).unwrap();
        values.push(1);
        let mut map = first.empty::<HashMap<u64, u64>>().unwrap();
        first.hash_room(&mut map, &1).unwrap();
        map.insert(1, 2);
        let mut tree = Tree::new(&first).unwrap();
        first
            .tree_insert(&mut tree, first.string("same").unwrap(), 1i64)
            .unwrap();
        let key = first.string("same").unwrap();
        let next = first.handoff(131, 4096).unwrap();
        let before = (next.live(), next.peak(), next.remaining());
        assert!(first.vec_grow(&mut values, 0).is_err());
        assert!(first.hash_room(&mut map, &1).is_err());
        assert!(first.tree_insert(&mut tree, key, 2i64).is_err());
        assert_eq!(&**values, &[1]);
        assert_eq!(map.get(&1), Some(&2));
        assert_eq!(next.remaining(), before.2);
        assert_eq!(next.peak(), before.1);
    }

    #[test]
    fn business_map_rejects_foreign_parent_before_spare_capacity_or_growth() {
        let first = Budget::new(65536, 0)
            .unwrap()
            .bind_initial(140, 65536)
            .unwrap();
        let base = first.live();
        let mut map = first
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut map = BusinessMap::<u64, u64>::plain();
                map.try_reserve_native(work, 3)?;
                map.insert(1, 10)?;
                Ok::<_, sc::CodecError>(map)
            })
            .unwrap();
        let foreign = Budget::new(65536, 0).unwrap();
        for additional in [0, map.capacity() + 1] {
            let capacity = map.capacity();
            let live = first.live();
            let result = foreign
                .codec_work()
                .unwrap()
                .run(|work| map.try_reserve_native(work, additional));
            assert!(matches!(
                result,
                Err(WorkError::Resource(ScratchFailure::Ledger))
            ));
            assert_eq!(map.capacity(), capacity);
            assert_eq!(map.get(&1), Some(&10));
            assert_eq!(first.live(), live);
        }
        let next = first.handoff(141, 65536).unwrap();
        next.codec_work()
            .unwrap()
            .run(|work| {
                map.try_reserve_native(work, 0)?;
                map.insert(1, 20)?;
                map.try_reserve_native(work, map.capacity() + 1)?;
                map.insert(2, 30)?;
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(map.get(&1), Some(&20));
        assert_eq!(map.get(&2), Some(&30));
        drop(map);
        assert_eq!(next.live(), base);
    }

    #[test]
    fn temporary_native_text_and_codec_capacity_release_on_actual_owner_drop() {
        use crate::v3::audit_store::{CodecRead, NativeAdmission};
        let parent = Budget::new(65536, 0).unwrap();
        let base = parent.live();
        parent
            .codec_work()
            .unwrap()
            .run(|work| {
                let admission = NativeAdmission(Some(work));
                let start = parent.live();
                let text = admission.native_owned_text("actual temporary text")?;
                let mut values = CodecVec::default();
                admission.native_vec_room(&mut values, 4)?;
                values.try_push(17u64)?;
                assert!(parent.live() > start);
                let capacity = values.capacity();
                admission.native_vec_room(&mut values, capacity + 1)?;
                let after_growth = parent.live();
                drop(values);
                assert!(parent.live() < after_growth);
                assert_eq!(&**text, "actual temporary text");
                drop(text);
                assert_eq!(parent.live(), start);
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.live(), base);
    }
}
