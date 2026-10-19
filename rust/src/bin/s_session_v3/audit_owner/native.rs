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
// #1488 P2-4：测试普查——布防期间逐笔记录该账每张许可的签发/归还（种类、字节），
// 独立于live差额推导，供逐许可核对封存池计费。按Ledger地址登记于账外，
// 不改Ledger布局（控制块M=Layout([AtomicUsize;2])⊕Layout(Ledger)=128属合同常量）。
#[cfg(test)]
static CENSUS: Mutex<Vec<(usize, Vec<CensusEvent>)>> = Mutex::new(Vec::new());
/// #1488 P2-4：单笔许可事件；kind=None为Permit/总账许可，其余为Native/Layout/Metadata分项。
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(crate) struct CensusEvent {
    pub(crate) reserve: bool,
    pub(crate) kind: &'static str,
    pub(crate) bytes: usize,
    // 记录时本线程累计真实释放字节（allocation_probe）；相邻两次归还之差即其间析构的对象。
    pub(crate) freed: u64,
}
/// 普查容量预留，记账不得引发分配/realloc而污染freed读数；超额即判普查失效。
#[cfg(test)]
const CENSUS_CAPACITY: usize = 1 << 20;
#[cfg(test)]
fn census_kind(kind: Option<ScratchCharge>) -> &'static str {
    match kind {
        None => "permit",
        Some(ScratchCharge::Metadata) => "metadata",
        Some(ScratchCharge::Layout) => "layout",
        Some(ScratchCharge::Native) => "native",
    }
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
    /// #1488 P2-1：遗留 fixture 构造，retained 写入 moved_in 供测试追认；生产编译无此入口。
    #[cfg(test)]
    pub(crate) fn new(limit: usize, retained: usize) -> Result<Self, BudgetExceeded> {
        Self::build(limit, retained)
    }
    /// #1488 P2-1：生产父账唯一出生入口；moved_in 恒 0，不存在可追认的未分配余额。
    pub(crate) fn fresh(limit: usize) -> Result<Self, BudgetExceeded> {
        Self::build(limit, 0)
    }
    fn build(limit: usize, retained: usize) -> Result<Self, BudgetExceeded> {
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
        self.bind_frozen_mapped(request, total, legacy, old_pool, None)
    }
    /// #1488 P1-1：carried = O + uncovered（同父L0超出同父合同额的真实分配）。
    /// frozen_live为冻结时逐项映射所读的同父L0；同锁复核，L0增长即映射过期，拒绝且不改账
    /// （L0下降只使uncovered偏保守，carried仍覆盖L）。
    pub(crate) fn bind_frozen_mapped(
        &self,
        request: u64,
        total: usize,
        legacy: usize,
        old_pool: usize,
        frozen_live: Option<usize>,
    ) -> Result<Self, BudgetExceeded> {
        let allowance = total
            .checked_sub(legacy)
            .and_then(|n| n.checked_sub(old_pool))
            .ok_or(BudgetExceeded)?;
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & BROKEN != 0 || self.1 != u64::from(d.serial) || request == 0 {
            return Err(BudgetExceeded);
        }
        if frozen_live.is_some_and(|l0| self.live() > l0) {
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
        // 换域即结束旧MoveIn：未拆余额仍属旧origin，此后的adopt_native记旧origin 0，只降累计占用。
        d.state &= !MOVING;
        d.serial = serial;
        d.total_limit = total_limit;
        d.remaining = allowance;
        d.low_water = allowance;
        Ok(Self(self.0.clone(), u64::from(serial), request, allowance))
    }
    /// #1488 P2-4：布防逐许可普查；已布防时清空重来。
    #[cfg(test)]
    pub(crate) fn census_arm(&self) {
        let key = Arc::as_ptr(&self.0) as usize;
        let mut census = CENSUS.lock().unwrap();
        census.retain(|(armed, _)| *armed != key);
        census.push((key, Vec::with_capacity(CENSUS_CAPACITY)));
    }
    /// #1488 P2-4：取出并撤防；未布防返回空。
    #[cfg(test)]
    pub(crate) fn census_take(&self) -> Vec<CensusEvent> {
        let key = Arc::as_ptr(&self.0) as usize;
        let mut census = CENSUS.lock().unwrap();
        census
            .iter()
            .position(|(armed, _)| *armed == key)
            .map(|i| census.swap_remove(i).1)
            .unwrap_or_default()
    }
    /// #1488 P1-1：容器真实驻留在本父账（同一Ledger）上；未绑定Native的容器不在任何账上。
    pub(crate) fn holds_codec_vec<T>(&self, v: &newchan_rust::s_content::CodecVec<T>) -> bool {
        let parent: Arc<dyn ScratchParent> = self.0.clone();
        v.is_native_bound() && v.native_parent_matches(&parent)
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
        // 同bind_frozen：换域结束旧MoveIn，余额返还按其旧origin只降L。
        d.state &= !MOVING;
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
    /// #1488 U2：追认已退出生产路径，仅测试 fixture 保留（编译期隔离）。
    #[cfg(test)]
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
    /// #1488 U2：追认已退出生产路径，仅测试 fixture 保留（编译期隔离）。
    #[cfg(test)]
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
    /// #1488 U2：std Vec 不追认既有容量。先从本父取同容量新块许可、再分配，载荷移入后旧块析构；
    /// 返回的许可覆盖新块且必须随容器持有。失败时容器与账均不变。
    /// #1488 P2-3：旧块无票，复制期新旧两块真实共存——旧块容量先按实占临时预留（计入峰值），
    /// 旧块析构后才返还；预留失败时容器与账均不变。
    pub(crate) fn rebirth_vec<T>(&self, v: &mut Vec<T>) -> Result<Permit, BudgetExceeded> {
        let coexist = self.reserve(vec_bytes::<T>(v.capacity()))?;
        let mut fee = self.reserve(0)?;
        let mut fresh = Vec::new();
        self.grow_vec(&mut fresh, &mut fee, v.capacity())?;
        fresh.append(v);
        drop(std::mem::replace(v, fresh));
        drop(coexist);
        Ok(fee)
    }
    /// #1488 U2：空父账出生后从父真实预留首笔费用（不经 moved_in 追认）。
    /// #1488 P2-2：生产 journal 已改为 Recording 出生前取许可，本 helper 仅测试编译。
    #[cfg(test)]
    /// 返回 Err(None) 表示父元数据不足，Err(Some(parent)) 表示首笔费用预留失败（父随之析构）。
    pub(crate) fn new_empty_with_fee(
        limit: usize,
        bytes: usize,
    ) -> Result<(Self, Permit), Option<BudgetExceeded>> {
        let budget = Self::fresh(limit).map_err(|_| None)?;
        let fee = budget.reserve(bytes).map_err(Some)?;
        Ok((budget, fee))
    }
    /// #1488 U2：生产路径只核“已由本父票出生”，不追认；未绑定/外来父/非Plain一律拒绝且不改动。
    pub(crate) fn require_codec_vec_owned<T>(
        &self,
        v: &newchan_rust::s_content::CodecVec<T>,
    ) -> Result<(), Failure> {
        self.check_current()?;
        let parent: Arc<dyn ScratchParent> = self.0.clone();
        if v.native_parent_matches(&parent) {
            Ok(())
        } else {
            Err(Failure::Invalid("原件未由本父账许可出生，不得追认".into()))
        }
    }
    /// #1488 U2：未持票Plain容器不追认既有容量。先从本父活工作域取新块许可、再分配，
    /// 载荷移入新块后旧块即析构；新许可随容器移动。已绑本父直接放行；外来父或非Plain拒绝且不改动。
    pub(crate) fn rebirth_codec_vec<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
        work: &newchan_rust::s_content::capture_allocation::scratch::WorkScope,
        additional: usize,
    ) -> Result<(), Failure> {
        self.check_current()?;
        if v.is_native_bound() {
            return self.require_codec_vec_owned(v);
        }
        if !v.can_adopt_native() {
            return Err(Failure::Invalid("Native不接其它allocator容器".into()));
        }
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        // #1488 P2-3：旧 Plain 块无票；复制期新旧共存，旧块容量先按实占临时预留（计入峰值），
        // 旧块析构后返还。预留失败时容器与账均不变。
        let old_bytes = v
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(BudgetExceeded)?;
        let coexist = work
            .reserve_native(old_bytes)
            .map_err(|e| Failure::Invalid(e.to_string()))?;
        let mut fresh = newchan_rust::s_content::CodecVec::from_plain(Vec::new());
        fresh
            .try_reserve_native(work, needed)
            .map_err(|e| Failure::Invalid(e.to_string()))?;
        let parent: Arc<dyn ScratchParent> = self.0.clone();
        if !fresh.native_parent_matches(&parent) {
            return Err(Failure::Invalid("工作域不属本父账".into()));
        }
        // 新块许可与分配均已成功；以下只做不可失败的移动（容量已足）。
        let old = std::mem::replace(v, fresh)
            .try_into_plain()
            .map_err(|e| Failure::Invalid(e.to_string()))?;
        for x in old {
            v.try_push(x).map_err(|e| Failure::Invalid(e.to_string()))?;
        }
        // 旧块已随 into_iter 析构完毕，临时共存许可此后返还。
        drop(coexist);
        Ok(())
    }
    pub(crate) fn codec_vec_grow<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
        additional: usize,
    ) -> Result<(), Failure> {
        self.preflight_codec_vec(v, additional)?;
        // 工作域元数据先准入。#1488 P2-1：生产只走不追认入口——未绑票 Plain 仅零容量可零字节
        // 绑票，非空裸块拒绝；遗留 moved_in fixture 的追认通道仅测试编译。
        self.codec_work()?
            .run(|w| {
                #[cfg(test)]
                if !v.is_native_bound() && v.capacity() != 0 && size_of::<T>() != 0 {
                    return v.try_reserve_native_adopting(w, additional);
                }
                v.try_reserve_native(w, additional)
            })
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
/// #1488 v4 冷出生许可：只由本次冻结调用签发，不可Clone，只可被`Budget::bootstrap`消费一次。
/// 根Ledger控制块M在`Arc::new`前按A与总帽检查（`new`内live=M<=limit）。
/// 生产唯一构造入口是`from_freeze`，须消费冻结入口私有铸造的`FreezeSeal`（字段与构造均为
/// request_audit模块私有）；许可绑定冻结域编号，消费方核对域后才出生根账。
pub(crate) struct BootstrapGrant {
    limit: usize,
    domain: u64,
}
impl BootstrapGrant {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn from_freeze(seal: super::super::request_audit::FreezeSeal) -> Self {
        Self {
            limit: seal.limit(),
            domain: seal.domain(),
        }
    }
    /// 纯账本算术fixture；域0不对应任何真实冻结（冻结域编号从1起）。
    #[cfg(test)]
    pub(crate) fn for_test(limit: usize) -> Self {
        Self { limit, domain: 0 }
    }
    pub(crate) fn limit(&self) -> usize {
        self.limit
    }
    pub(crate) fn domain(&self) -> u64 {
        self.domain
    }
}
/// #1488 v4 移入许可：已存在载荷在一个锁事务中按当前域双门（G+x<=A、L+x<=总帽）真实预留，
/// 再逐项拆成随字段移动的Permit/Native许可；不是无票unassigned。未拆部分在Drop时归还。
/// #1488 MoveIn已退出生产路径（ColdBuildGuard前移），仅测试 fixture 保留（编译期隔离）。
#[cfg(test)]
pub(crate) struct MoveIn {
    budget: Budget,
    origin: u64,
}
impl Budget {
    /// #1488 测试观察：未经真实预留拆分的移入余额（生产冷出生必须恒为0）。
    #[cfg(test)]
    pub(crate) fn moved_in_for_test(&self) -> usize {
        self.0.domain.lock().map_or(usize::MAX, |d| d.moved_in)
    }
    pub(crate) fn bootstrap(grant: BootstrapGrant) -> Result<Self, BudgetExceeded> {
        Self::fresh(grant.limit)
    }
    #[cfg(test)]
    pub(crate) fn move_in(&self, bytes: usize) -> Result<MoveIn, BudgetExceeded> {
        {
            let d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
            // 先于预留检查，拒绝时不触碰live/peak（含遗留fixture的moved_in）。
            if d.state & (BROKEN | MOVING) != 0 || d.moved_in != 0 || u64::from(d.serial) != self.1
            {
                return Err(BudgetExceeded);
            }
        }
        let origin = self
            .0
            .reserve_total(bytes, Some(self.1))
            .map_err(|_| BudgetExceeded)?;
        let mut d = self.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & MOVING != 0 || d.moved_in != 0 {
            drop(d);
            self.0
                .release_total(bytes, origin)
                .map_err(|_| BudgetExceeded)?;
            return Err(BudgetExceeded);
        }
        d.moved_in = bytes;
        d.state |= MOVING;
        Ok(MoveIn {
            budget: self.clone(),
            origin,
        })
    }
}
#[cfg(test)]
impl MoveIn {
    /// 从本次真实移入预留中拆出同origin许可；超额或过期域拒绝且不改账。
    pub(crate) fn take(&self, bytes: usize) -> Result<Permit, BudgetExceeded> {
        let mut d = self.budget.0.domain.lock().map_err(|_| BudgetExceeded)?;
        if d.state & BROKEN != 0 || d.state & MOVING == 0 || u64::from(d.serial) != self.origin {
            return Err(BudgetExceeded);
        }
        d.moved_in = d.moved_in.checked_sub(bytes).ok_or(BudgetExceeded)?;
        Ok(Permit {
            budget: self.budget.clone(),
            bytes,
            origin: self.origin,
        })
    }
    pub(crate) fn budget(&self) -> &Budget {
        &self.budget
    }
    pub(crate) fn remaining(&self) -> usize {
        self.budget.0.domain.lock().map_or(0, |d| d.moved_in)
    }
}
#[cfg(test)]
impl Drop for MoveIn {
    fn drop(&mut self) {
        let rest = match self.budget.0.domain.lock() {
            // 换域后已排空且新域另起MoveIn：余额属于新移入，旧句柄不得取走。
            Ok(d) if d.state & MOVING != 0 && u64::from(d.serial) != self.origin => return,
            Ok(mut d) => {
                d.state &= !MOVING;
                std::mem::take(&mut d.moved_in)
            }
            Err(_) => return,
        };
        self.budget
            .0
            .release_total(rest, self.origin)
            .expect("Native移入许可返还失配");
    }
}
/// #1488 v4 只读观察句柄：控制块分配前从真实父同当前origin预留M；只持sponsor许可，
/// 不能reserve/adopt/codec_work/handoff，且不可转成Budget。最后一个句柄释放时sponsor归还。
#[derive(Clone)]
pub(crate) struct BudgetObserver(Arc<ObserverInner>);
struct ObserverInner {
    budget: Budget,
    _sponsor: Permit,
    native_base: usize,
}
impl BudgetObserver {
    /// sponsor按根Ledger元数据M计费，覆盖观察控制块的实际布局（下方断言实测）。
    pub(crate) fn sponsor_bytes() -> usize {
        Budget::metadata_bytes()
    }
    pub(crate) fn new(budget: &Budget) -> Result<Self, BudgetExceeded> {
        let layout = Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<ObserverInner>())
            .expect("观察句柄布局")
            .0
            .pad_to_align()
            .size();
        let bytes = Self::sponsor_bytes();
        if layout > bytes {
            return Err(BudgetExceeded);
        }
        let sponsor = budget.reserve(bytes)?;
        Ok(Self(Arc::new(ObserverInner {
            budget: budget.clone(),
            native_base: budget.0.native_live.load(Ordering::Acquire),
            _sponsor: sponsor,
        })))
    }
    /// 当前冻结域增量G（含sponsor M）；旧域或过期句柄读作全额A。
    pub(crate) fn live(&self) -> usize {
        let b = &self.0.budget;
        b.3.saturating_sub(b.remaining())
    }
    pub(crate) fn peak(&self) -> usize {
        self.0.budget.request_peak()
    }
    pub(crate) fn total_live(&self) -> usize {
        self.0.budget.live()
    }
    pub(crate) fn same_parent(&self, other: &Budget) -> bool {
        self.0.budget.same_parent(other)
    }
    #[cfg(test)]
    pub(super) fn native_observation(&self) -> (usize, usize) {
        let (live, peak) = self.0.budget.native_observation();
        (
            live.saturating_sub(self.0.native_base),
            peak.saturating_sub(self.0.native_base),
        )
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
        #[cfg(test)]
        self.census_note(true, kind, bytes);
        Ok(u64::from(d.serial))
    }
    #[cfg(test)]
    fn census_note(&self, reserve: bool, kind: Option<ScratchCharge>, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let key = self as *const Self as usize;
        if let Ok(mut census) = CENSUS.lock() {
            if let Some((_, events)) = census.iter_mut().find(|(armed, _)| *armed == key) {
                assert!(events.len() < CENSUS_CAPACITY, "许可普查容量耗尽");
                events.push(CensusEvent {
                    reserve,
                    kind: census_kind(kind),
                    bytes,
                    freed: super::allocation_probe::released_bytes(),
                });
            }
        }
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
        #[cfg(test)]
        self.census_note(false, kind, bytes);
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
    /// #1488 P2-1：追认覆写仅测试 fixture 编译；生产 Ledger 落 trait 默认（恒拒），
    /// 库侧 NativeReservation::adopt / WorkScope::adopt_native / try_reserve_native_adopting
    /// 在生产父账上因此不可能把已分配字节接成许可（全仓仅本 impl 覆写 adopt_native）。
    #[cfg(test)]
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
        // 活动MoveIn的字节是当前域真实预留，返还按当前origin回补A；fixture池仍是旧origin 0。
        Ok(if d.state & MOVING != 0 {
            u64::from(d.serial)
        } else {
            0
        })
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

#[cfg(test)]
// #1488 v4 统一所有权反例锁（B/D/E的账本层与MoveIn/grant）。
mod unified_ownership_v4_tests {
    use super::*;

    fn domain(b: &Budget) -> (usize, usize, usize, usize, u8) {
        let d = b.0.domain.lock().unwrap();
        (b.live(), b.peak(), d.remaining, d.moved_in, d.state)
    }

    /// 反例B：T1000/E100/O700/A200/M128。new20成功总占948；new200拒绝且不改账；旧费返还不回补A。
    #[test]
    fn observer_sponsor_counts_in_allowance_and_old_drop_never_refills() {
        let first = Budget::new(900, 0).unwrap().bind_initial(100, 900).unwrap();
        let metadata = Budget::metadata_bytes();
        let old_candidate = first
            .reserve(650 - metadata)
            .unwrap()
            .hold(vec![7u8; 650 - metadata]);
        let old_raw = first.reserve(50).unwrap().hold(vec![8u8; 50]);
        assert_eq!(first.live(), 700);
        let next = first.bind_frozen(101, 1000, 100, 700).unwrap();
        assert_eq!(BudgetObserver::sponsor_bytes(), 128);
        let observer = BudgetObserver::new(&next).unwrap();
        assert_eq!(observer.live(), 128);
        assert_eq!(next.remaining(), 200 - 128);
        let new = next.reserve(20).unwrap().hold(vec![9u8; 20]);
        assert_eq!(next.live() + 100, 948);
        assert_eq!(observer.live(), 148);
        assert_eq!(observer.peak(), 148);
        let before = domain(&next);
        assert!(next.reserve(200).is_err());
        assert_eq!(domain(&next), before, "拒绝不得部分改账");
        drop(old_candidate);
        drop(old_raw);
        assert_eq!(next.remaining(), 200 - 148, "旧来源返还不能增发当前A");
        assert_eq!(observer.live(), 148);
        drop(new);
        assert_eq!(observer.live(), 128);
        let clone = observer.clone();
        drop(observer);
        assert_eq!(clone.live(), 128, "sponsor随最后一个句柄释放");
        drop(clone);
        assert_eq!(next.remaining(), 200);
    }

    /// 观察句柄sponsor超过当前A时拒绝，且不改账。
    #[test]
    fn observer_rejects_when_sponsor_exceeds_allowance_without_mutation() {
        let first = Budget::new(900, 0).unwrap().bind_initial(102, 900).unwrap();
        let held = first.reserve(700 - Budget::metadata_bytes()).unwrap();
        let next = first.bind_frozen(103, 900, 100, 700).unwrap();
        assert_eq!(next.remaining(), 100);
        let before = domain(&next);
        assert!(BudgetObserver::new(&next).is_err());
        assert_eq!(domain(&next), before);
        drop(held);
    }

    /// MoveIn：双门真实预留、拆出同origin许可、超额拒绝不改账、Drop归还未拆部分并解除MOVING。
    #[test]
    fn move_in_reserves_splits_and_returns_rest_without_partial_mutation() {
        let root = Budget::bootstrap(BootstrapGrant::for_test(1024))
            .unwrap()
            .bind_initial(200, 1024)
            .unwrap();
        let base = root.live();
        assert_eq!(base, Budget::metadata_bytes());
        let pristine = domain(&root);
        assert!(root.move_in(1024).is_err());
        assert_eq!(domain(&root), pristine, "超额移入不得部分改账");
        let moving = root.move_in(300).unwrap();
        assert_eq!(root.live(), base + 300);
        assert_eq!(root.remaining(), 1024 - base - 300);
        let state = domain(&root);
        assert!(root.move_in(1).is_err(), "同一时刻只有一个MoveIn");
        assert_eq!(domain(&root), state);
        let permit = moving.take(100).unwrap();
        let state = domain(&root);
        assert!(moving.take(201).is_err());
        assert_eq!(domain(&root), state);
        assert_eq!(moving.remaining(), 200);
        drop(moving);
        assert_eq!(root.live(), base + 100);
        assert_eq!(root.remaining(), 1024 - base - 100);
        assert_eq!(root.0.domain.lock().unwrap().state & MOVING, 0);
        drop(permit);
        assert_eq!(root.live(), base);
        assert_eq!(root.remaining(), 1024 - base);
        assert_eq!(root.peak(), base + 300);
        let next = root.handoff(201, 512).unwrap();
        assert_eq!(next.remaining(), 512);
    }

    /// 换域结束MoveIn：旧移入不可再拆；未拆余额与其后adopt的许可返还只降L，不回补新域A。
    #[test]
    fn handoff_ends_move_in_and_old_rest_never_refills_new_allowance() {
        let root = Budget::bootstrap(BootstrapGrant::for_test(4096))
            .unwrap()
            .bind_initial(210, 4096)
            .unwrap();
        let base = root.live();
        let moving = root.move_in(300).unwrap();
        let kept = moving.take(100).unwrap();
        let adopted = root.adopt(50).unwrap();
        let next = root.handoff(211, 1024).unwrap();
        assert_eq!(next.0.domain.lock().unwrap().state & MOVING, 0);
        assert!(moving.take(1).is_err(), "旧域MoveIn不可在新域拆分");
        assert_eq!(next.remaining(), 1024);
        let new = next.reserve(24).unwrap();
        drop(moving);
        drop(kept);
        drop(adopted);
        assert_eq!(next.live(), base + 24);
        assert_eq!(next.remaining(), 1000, "旧origin返还不回补新域A");
        drop(new);
        assert_eq!(next.remaining(), 1024);
        assert_eq!(next.live(), base);
    }

    /// 旧Budget::new(limit, retained)遗留的moved_in未清零时，MoveIn拒绝且不改账。
    #[test]
    fn move_in_rejects_legacy_moved_in_without_mutation() {
        let legacy = Budget::new(4096, 64).unwrap();
        let before = domain(&legacy);
        assert!(legacy.move_in(32).is_err());
        assert_eq!(domain(&legacy), before);
    }
}

#[cfg(test)]
// #1488 U2：五处追认点退出生产路径的正/负锁。负锁统一用“遗留 moved_in 足以追认、真实余额不足”的父：
// 若仍走 adopt 会成功；新路径必须拒绝且不改容器与账。
mod u2_exit_adopt_tests {
    use super::*;
    use newchan_rust::s_content::{capture_allocation::scratch::WorkScope, CodecVec};

    fn moved_in(b: &Budget) -> usize {
        b.0.domain.lock().unwrap().moved_in
    }
    fn ledger(b: &Budget) -> (usize, usize, usize, usize) {
        (b.live(), b.peak(), b.remaining(), moved_in(b))
    }
    fn in_work<R>(b: &Budget, f: impl FnOnce(&WorkScope) -> Result<R, Failure>) -> Result<R, Failure> {
        b.codec_work()?.run(f).map_err(|e| match e {
            newchan_rust::s_content::capture_allocation::scratch::WorkError::Resource(e) => {
                Failure::Invalid(format!("{e:?}"))
            }
            newchan_rust::s_content::capture_allocation::scratch::WorkError::Operation(e) => e,
        })
    }
    fn born<T>(b: &Budget, items: Vec<T>) -> CodecVec<T> {
        in_work(b, |scope| {
            let mut v = CodecVec::from_plain(Vec::new());
            v.try_reserve_native(scope, items.len())
                .map_err(|e| Failure::Invalid(e.to_string()))?;
            for x in items {
                v.try_push(x).map_err(|e| Failure::Invalid(e.to_string()))?;
            }
            Ok(v)
        })
        .unwrap()
    }
    fn legacy_tight(cost: usize) -> Budget {
        Budget::new(Budget::metadata_bytes() + cost, cost).unwrap()
    }

    // effects.rs material 父（原 1546）：空父出生、首笔 journal 费真实预留，moved_in 恒 0。
    #[test]
    fn u2_material_parent_born_empty_and_journal_fee_reserved() {
        let (budget, fee) = Budget::new_empty_with_fee(4096, 100).unwrap();
        assert_eq!(moved_in(&budget), 0);
        assert_eq!(budget.live(), Budget::metadata_bytes() + 100);
        drop(fee);
        assert_eq!(budget.live(), Budget::metadata_bytes());
    }
    #[test]
    fn u2_material_parent_rejects_fee_beyond_real_room_instead_of_moved_in() {
        assert!(matches!(
            Budget::new_empty_with_fee(Budget::metadata_bytes() + 99, 100),
            Err(Some(_))
        ));
        assert!(matches!(Budget::new_empty_with_fee(1, 0), Err(None)));
        // 旧通道 Budget::new(limit, bytes) 能“容纳”同一笔费，正是追认；新入口不走它。
        assert!(Budget::new(Budget::metadata_bytes() + 100, 100).is_ok());
    }

    // q.rs raw（原 546）：同容量新块先取许可再分配，许可随容器；moved_in 不动。
    #[test]
    fn u2_q_raw_rebirth_reserves_new_block_and_fee_moves_with_vec() {
        let budget = Budget::new(4096, 0).unwrap();
        let base = budget.live();
        let mut raw = Vec::with_capacity(6);
        raw.extend([1u64, 2, 3]);
        let old_ptr = raw.as_ptr();
        let fee = budget.rebirth_vec(&mut raw).unwrap();
        assert_eq!(raw, [1, 2, 3]);
        assert_ne!(raw.as_ptr(), old_ptr, "必须是许可之后新分配的块");
        assert_eq!(fee.bytes, vec_bytes::<u64>(raw.capacity()));
        assert_eq!(budget.live(), base + fee.bytes);
        assert_eq!(moved_in(&budget), 0);
        let mut fee = fee;
        let more = raw.capacity() + 1;
        budget.grow_vec(&mut raw, &mut fee, more).unwrap();
        drop(fee);
        assert_eq!(budget.live(), base);
    }
    #[test]
    fn u2_q_raw_rebirth_never_draws_moved_in() {
        let mut raw = Vec::with_capacity(4);
        raw.extend([7u64, 8]);
        let cost = vec_bytes::<u64>(raw.capacity());
        let budget = legacy_tight(cost);
        let before = ledger(&budget);
        let (ptr, cap) = (raw.as_ptr(), raw.capacity());
        assert!(budget.rebirth_vec(&mut raw).is_err());
        assert_eq!((raw.as_ptr(), raw.capacity(), raw.as_slice()), (ptr, cap, &[7u64, 8][..]));
        assert_eq!(ledger(&budget), before);
    }

    // effects.rs originals.rows（原 739）：只核本父出生身份，不追认。
    #[test]
    fn u2_effects_rows_accept_same_parent_born_without_charge() {
        let budget = Budget::new(65536, 0).unwrap();
        let rows = born(&budget, vec![1u64, 2]);
        let before = ledger(&budget);
        budget.require_codec_vec_owned(&rows).unwrap();
        assert_eq!(ledger(&budget), before);
    }
    #[test]
    fn u2_effects_rows_reject_plain_and_foreign_without_adopt() {
        let plain = CodecVec::from_plain(vec![1u64, 2, 3]);
        let budget = legacy_tight(plain.capacity() * size_of::<u64>());
        let before = ledger(&budget);
        assert!(budget.require_codec_vec_owned(&plain).is_err());
        assert!(!plain.is_native_bound());
        assert_eq!(ledger(&budget), before);
        let foreign = Budget::new(65536, 0).unwrap();
        let foreign_rows = born(&foreign, vec![4u64]);
        let other = Budget::new(65536, 0).unwrap();
        let other_before = ledger(&other);
        assert!(other.require_codec_vec_owned(&foreign_rows).is_err());
        assert_eq!(ledger(&other), other_before);
    }

    // history.rs raw 前缀（原 254）：未持票容器在本父活工作域重新出生；moved_in 不动。
    #[test]
    fn u2_history_prefix_rebirth_binds_new_block_under_parent() {
        let budget = Budget::new(65536, 0).unwrap();
        let base = budget.live();
        let mut nodes = CodecVec::from_plain(vec![[1u8; 32], [2u8; 32]]);
        in_work(&budget, |scope| budget.rebirth_codec_vec(&mut nodes, scope, 3)).unwrap();
        assert!(nodes.is_native_bound());
        budget.require_codec_vec_owned(&nodes).unwrap();
        assert_eq!(&*nodes, &[[1u8; 32], [2u8; 32]]);
        assert!(nodes.capacity() >= 5);
        assert_eq!(moved_in(&budget), 0);
        assert!(budget.live() > base);
        drop(nodes);
        assert_eq!(budget.live(), base);
    }
    #[test]
    fn u2_history_prefix_rebirth_never_draws_moved_in_or_foreign() {
        let mut nodes = CodecVec::from_plain(vec![[3u8; 32]]);
        let cost = nodes.capacity() * 32;
        let budget = Budget::new(Budget::metadata_bytes() + CodecWork::metadata_bytes() + cost, cost)
            .unwrap();
        // 工作域元数据会抬 peak；判据为 live/余额/moved_in 不变。
        let before = (budget.live(), budget.remaining(), moved_in(&budget));
        let ptr = nodes.as_ptr();
        assert!(in_work(&budget, |scope| budget.rebirth_codec_vec(&mut nodes, scope, 0)).is_err());
        assert!(!nodes.is_native_bound());
        assert_eq!((nodes.as_ptr(), &*nodes), (ptr, &[[3u8; 32]][..]));
        assert_eq!((budget.live(), budget.remaining(), moved_in(&budget)), before);
        let foreign = Budget::new(65536, 0).unwrap();
        let mut foreign_nodes = born(&foreign, vec![[4u8; 32]]);
        let other = Budget::new(65536, 0).unwrap();
        assert!(in_work(&other, |scope| other.rebirth_codec_vec(&mut foreign_nodes, scope, 1))
            .is_err());
        foreign.require_codec_vec_owned(&foreign_nodes).unwrap();
    }
    // #1488 P2-3（评审 0024）：rebirth 复制期新旧块共存，父账峰值必须同时含旧块与新块；
    // 余额只够新块、不够新旧共存时拒绝且容器与 live/余额不变。
    #[test]
    fn p2_3_rebirth_vec_peak_counts_old_and_new_blocks() {
        let budget = Budget::fresh(4096).unwrap();
        let base = budget.live();
        let mut raw = Vec::with_capacity(8);
        raw.extend([1u64, 2, 3]);
        let old = vec_bytes::<u64>(raw.capacity());
        let fee = budget.rebirth_vec(&mut raw).unwrap();
        let new = vec_bytes::<u64>(raw.capacity());
        assert_eq!(fee.bytes, new);
        assert!(budget.peak() >= base + old + new, "峰值漏计旧块");
        assert_eq!(budget.live(), base + new, "旧块析构后临时共存许可必须返还");
        drop(fee);
        assert_eq!(budget.live(), base);

        let tight = Budget::fresh(Budget::metadata_bytes() + old + old - 1).unwrap();
        let mut raw = Vec::with_capacity(8);
        raw.extend([4u64, 5]);
        let (ptr, cap) = (raw.as_ptr(), raw.capacity());
        let before = (tight.live(), tight.remaining());
        assert!(tight.rebirth_vec(&mut raw).is_err());
        assert_eq!((raw.as_ptr(), raw.capacity(), raw.as_slice()), (ptr, cap, &[4u64, 5][..]));
        assert_eq!((tight.live(), tight.remaining()), before);
    }
    #[test]
    fn p2_3_rebirth_codec_vec_peak_counts_old_and_new_blocks() {
        let budget = Budget::fresh(65536).unwrap();
        let base = budget.live();
        let mut plain = Vec::with_capacity(4);
        plain.push([1u8; 32]);
        let mut nodes = CodecVec::from_plain(plain);
        let old = nodes.capacity() * 32;
        in_work(&budget, |scope| budget.rebirth_codec_vec(&mut nodes, scope, 0)).unwrap();
        let new = nodes.capacity() * 32;
        assert!(nodes.is_native_bound());
        assert!(
            budget.peak() >= base + CodecWork::metadata_bytes() + old + new,
            "峰值漏计旧块"
        );
        assert_eq!(budget.live(), base + new, "旧块析构后临时共存许可必须返还");
        drop(nodes);
        assert_eq!(budget.live(), base);

        let room = CodecWork::metadata_bytes() + old + old - 1;
        let tight = Budget::fresh(Budget::metadata_bytes() + room).unwrap();
        let mut plain = Vec::with_capacity(4);
        plain.push([2u8; 32]);
        let mut nodes = CodecVec::from_plain(plain);
        let ptr = nodes.as_ptr();
        let before = (tight.live(), tight.remaining());
        assert!(in_work(&tight, |scope| tight.rebirth_codec_vec(&mut nodes, scope, 0)).is_err());
        assert!(!nodes.is_native_bound());
        assert_eq!((nodes.as_ptr(), &*nodes), (ptr, &[[2u8; 32]][..]));
        assert_eq!((tight.live(), tight.remaining()), before);
    }
    // #1488 P2-1（评审 0024）：生产出生入口 fresh 无 moved_in；codec_vec_grow 对已分配裸块拒绝
    // 且不改容器与账，零容量裸块只经真实预留绑票；未覆写 adopt_native 的父（生产 Ledger 形态）恒拒。
    #[test]
    fn u2_fresh_parent_codec_vec_grow_never_adopts_allocated_plain() {
        let budget = Budget::fresh(65536).unwrap();
        assert_eq!(moved_in(&budget), 0);
        let mut values = CodecVec::from_plain(vec![7u64, 8, 9]);
        let before = ledger(&budget);
        let ptr = values.as_ptr();
        assert!(budget.codec_vec_grow(&mut values, 0).is_err());
        assert!(budget.codec_vec_grow(&mut values, 8).is_err());
        assert!(!values.is_native_bound());
        assert_eq!((values.as_ptr(), &*values), (ptr, &[7u64, 8, 9][..]));
        assert_eq!(ledger(&budget), before);
        let mut empty = CodecVec::<u64>::from_plain(Vec::new());
        budget.codec_vec_grow(&mut empty, 4).unwrap();
        assert!(empty.is_native_bound());
        budget.require_codec_vec_owned(&empty).unwrap();
        assert!(budget.live() >= before.0 + 4 * size_of::<u64>());
        assert_eq!(moved_in(&budget), 0);
        drop(empty);
        assert_eq!(budget.live(), before.0);

        struct DefaultParent;
        impl ScratchParent for DefaultParent {
            fn reserve(
                &self,
                _: usize,
                _: ScratchCharge,
            ) -> Result<u64, ScratchFailure> {
                Ok(0)
            }
            fn release(
                &self,
                _: usize,
                _: ScratchCharge,
                _: u64,
            ) -> Result<(), ScratchFailure> {
                Ok(())
            }
            fn health(&self) -> Result<(), ScratchFailure> {
                Ok(())
            }
            fn invalidate(&self) {}
        }
        let parent: Arc<dyn ScratchParent> = Arc::new(DefaultParent);
        assert!(
            newchan_rust::s_content::capture_allocation::scratch::NativeReservation::adopt(
                parent, 0, 0
            )
            .is_err()
        );
    }
}
