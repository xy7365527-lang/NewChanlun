//! #1394 Native 的独立上界账。这里从不写入 Owned requested Layout 账。
//! 许可先占完整新块，扩容返回后才释放旧块；临时许可由 Drop 归还。
use crate::v3::audit_retention::{hash_slots, sum, tree_slots, vec_bytes, Retained};
use std::{
    alloc::Layout,
    collections::{BTreeMap, HashMap},
    hash::Hash,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
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
pub(crate) struct Budget(Arc<Ledger>);
struct Ledger {
    limit: usize,
    live: AtomicUsize,
    peak: AtomicUsize,
    broken: AtomicBool,
    scratch_live: AtomicUsize,
    scratch_peak: AtomicUsize,
    metadata_live: AtomicUsize,
    metadata_peak: AtomicUsize,
}
pub(crate) struct Permit {
    budget: Budget,
    bytes: usize,
}
/// 字段按声明顺序析构：被保护的工作集先真正释放，许可随后归还。
/// 避免 `(value, permit)` 局部绑定逆序析构时，先放掉许可再释放 value。
pub(crate) struct Reserved<T> {
    value: T,
    _permit: Permit,
}
impl<T> std::ops::Deref for Reserved<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.budget.release(self.bytes);
    }
}
impl Permit {
    pub(crate) fn hold<T>(self, value: T) -> Reserved<T> {
        Reserved {
            value,
            _permit: self,
        }
    }
    /// 只在成功构造且真实旧载荷已释放后结算。actual 是约定的保守保留费，不是 Layout 实测。
    pub(crate) fn commit(mut self, old: usize, actual: usize) {
        assert!(actual <= self.bytes, "预留必须覆盖新保留费");
        self.budget.release(self.bytes - actual);
        self.budget.release(old);
        self.bytes = 0;
    }
}
impl Budget {
    pub(crate) fn new(limit: usize, retained: usize) -> Result<Self, BudgetExceeded> {
        // 当前 Rust 1.98 ArcInner 元数据；在 Arc::new 前预留，不声称 fallible Arc。
        let metadata = Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<Ledger>())
            .map_err(|_| BudgetExceeded)?
            .0
            .pad_to_align()
            .size();
        let live = retained.checked_add(metadata).ok_or(BudgetExceeded)?;
        if live > limit {
            return Err(BudgetExceeded);
        }
        Ok(Self(Arc::new(Ledger {
            limit,
            live: AtomicUsize::new(live),
            peak: AtomicUsize::new(live),
            broken: AtomicBool::new(false),
            scratch_live: AtomicUsize::new(0),
            scratch_peak: AtomicUsize::new(0),
            metadata_live: AtomicUsize::new(0),
            metadata_peak: AtomicUsize::new(0),
        })))
    }
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Permit, BudgetExceeded> {
        self.0.reserve_total(bytes).map_err(|_| BudgetExceeded)?;
        Ok(Permit {
            budget: self.clone(),
            bytes,
        })
    }
    pub(crate) fn live(&self) -> usize {
        self.0.live.load(Ordering::Acquire)
    }
    pub(crate) fn peak(&self) -> usize {
        self.0.peak.load(Ordering::Acquire)
    }
    pub(crate) fn release(&self, bytes: usize) {
        self.0
            .release_total(bytes)
            .expect("Native 归属账不得释放未持有费用");
    }
    pub(crate) fn codec_work(&self) -> Result<CodecWork, Failure> {
        // Arc coercion 复用已有控制块，不 new 包装适配器。
        CodecWork::new(self.0.clone(), self.0.limit).map_err(Failure::from)
    }
    pub(crate) fn scratch_observation(&self) -> (usize, usize, usize, usize) {
        (
            self.0.scratch_live.load(Ordering::Acquire),
            self.0.scratch_peak.load(Ordering::Acquire),
            self.0.metadata_live.load(Ordering::Acquire),
            self.0.metadata_peak.load(Ordering::Acquire),
        )
    }
    pub(crate) fn string(&self, value: &str) -> Result<String, BudgetExceeded> {
        let permit = self.reserve(value.len())?;
        let mut out = String::new();
        out.try_reserve_exact(value.len())
            .map_err(|_| BudgetExceeded)?;
        out.push_str(value);
        permit.commit(0, out.capacity());
        Ok(out)
    }
    pub(crate) fn vec_grow<T>(
        &self,
        v: &mut Vec<T>,
        additional: usize,
    ) -> Result<(), BudgetExceeded> {
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        if needed <= v.capacity() {
            return Ok(());
        }
        let next = needed
            .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
            .max(4);
        let permit = self.reserve(vec_bytes::<T>(next))?;
        let old = vec_bytes::<T>(v.capacity());
        v.try_reserve_exact(next - v.len())
            .map_err(|_| BudgetExceeded)?;
        permit.commit(old, vec_bytes::<T>(v.capacity()));
        Ok(())
    }
    pub(crate) fn codec_vec_grow<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
        additional: usize,
    ) -> Result<(), Failure> {
        if v.allocation_budget().is_some() {
            return Err(Failure::Invalid("Native预留不接Owned容器".into()));
        }
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        if needed <= v.capacity() {
            return Ok(());
        }
        let next = needed
            .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
            .max(4);
        let permit = self.reserve(vec_bytes::<T>(next))?;
        let old = vec_bytes::<T>(v.capacity());
        v.try_reserve_exact(next - v.len())
            .map_err(|_| BudgetExceeded)?;
        permit.commit(old, vec_bytes::<T>(v.capacity()));
        Ok(())
    }
    pub(crate) fn hash_room<K: Eq + Hash, V>(
        &self,
        m: &mut HashMap<K, V>,
        key: &K,
    ) -> Result<(), BudgetExceeded> {
        if m.contains_key(key) || m.len() < m.capacity() {
            return Ok(());
        }
        let cap = hash_capacity(m.len().checked_add(1).ok_or(BudgetExceeded)?)?;
        let permit = self.reserve(hash_slots::<K, V>(cap))?;
        let old = hash_slots::<K, V>(m.capacity());
        m.try_reserve(1).map_err(|_| BudgetExceeded)?;
        permit.commit(old, hash_slots::<K, V>(m.capacity()));
        Ok(())
    }
    pub(crate) fn tree_insert<K: Ord + Retained, V: Retained>(
        &self,
        m: &mut BTreeMap<K, V>,
        k: K,
        v: V,
    ) -> Result<(), BudgetExceeded> {
        // k/v 的独立载荷已由调用点持账；这里只管理节点与真正被覆盖的旧载荷。
        if let Some(old) = m.get_mut(&k) {
            let released = sum([k.heap_bytes(), old.heap_bytes()]);
            *old = v;
            drop(k);
            self.release(released);
            return Ok(());
        }
        let permit = self.reserve(tree_growth::<K, V>(m.len()))?;
        m.insert(k, v);
        permit.commit(0, tree_slots::<K, V>(1));
        Ok(())
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
    fn reserve_total(&self, bytes: usize) -> Result<(), ScratchFailure> {
        if self.broken.load(Ordering::Acquire) {
            return Err(ScratchFailure::Ledger);
        }
        let old = self
            .live
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes).filter(|n| *n <= self.limit)
            })
            .map_err(|_| ScratchFailure::Budget)?;
        self.peak.fetch_max(old + bytes, Ordering::Relaxed);
        Ok(())
    }
    fn release_total(&self, bytes: usize) -> Result<(), ScratchFailure> {
        if self
            .live
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_sub(bytes)
            })
            .is_err()
        {
            self.broken.store(true, Ordering::Release);
            return Err(ScratchFailure::Ledger);
        }
        Ok(())
    }
    fn component(&self, kind: ScratchCharge) -> (&AtomicUsize, &AtomicUsize) {
        match kind {
            ScratchCharge::Layout => (&self.scratch_live, &self.scratch_peak),
            ScratchCharge::Metadata => (&self.metadata_live, &self.metadata_peak),
        }
    }
}
impl ScratchParent for Ledger {
    fn health(&self) -> Result<(), ScratchFailure> {
        if self.broken.load(Ordering::Acquire) {
            Err(ScratchFailure::Ledger)
        } else {
            Ok(())
        }
    }
    fn invalidate(&self) {
        self.broken.store(true, Ordering::Release);
    }

    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure> {
        self.reserve_total(bytes)?;
        let (live, peak) = self.component(kind);
        // 分项仅观测，准入由同一 total CAS 决定。
        let n = live.fetch_add(bytes, Ordering::AcqRel) + bytes;
        peak.fetch_max(n, Ordering::Relaxed);
        Ok(())
    }
    fn release(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure> {
        let (live, _) = self.component(kind);
        if live
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_sub(bytes)
            })
            .is_err()
        {
            self.broken.store(true, Ordering::Release);
            return Err(ScratchFailure::Ledger);
        }
        self.release_total(bytes)
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
        let mut v = Vec::<u64>::new();
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
