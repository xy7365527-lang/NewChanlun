//! #1419 M8A 首片：当前不可变 capture 的私有本步读取接口。
//!
//! 调用者以完整引用和 Mode 取一份根租约；内部 Flavor 仍由原 Verifier 推导。
//! M8B 增加同事务受管 SQL 后端；仍不提供淘汰或跨请求能力。
//! capture 借用仍会 pin 全部原件；这不是有界持有已经完成的声明。
use super::*;
use capture_allocation::scratch::WorkScope;

pub(super) mod managed_sql;
#[cfg(test)]
mod production_diff;

/// 同一个不可变 owner 与同一活动工作域是本片的封闭读取域。
/// 这里只借用，不复制父账、origin 或生涯能力；不冒充尚未实现的 SQL 事务身份。
#[cfg(test)]
#[derive(Clone, Copy)]
struct CaptureStep<'step> {
    capture: &'step AuditCapture,
    work: &'step WorkScope,
}

/// R1：单一本步读取入口。驻留原件走原 OwnerCapture 帧核；未驻留且装有同域 SQL
/// 后端时走同事务受管 SQL 帧核（`read_scoped_checked`，与 Source 上下文同一函数）。
/// 两者都只交已核摘要的帧，不另立判据，不换源重读；尚未启用淘汰，生产全部命中驻留。
#[cfg(test)]
struct StepReader<'step, 'tx, 'conn> {
    backend: CaptureStep<'step>,
    sql: Option<managed_sql::ManagedSqlStep<'tx, 'conn, 'step>>,
}

/// 不提供 Clone、into_body 或跨步句柄。正文的可见借用受租约限制，
/// 租约又独占借用 reader；退出当前帧即结束借用，不销毁 capture backing。
#[cfg(test)]
struct ReadLease<'lease> {
    reference: RefView<'lease>,
    flavor: Flavor,
    mode: Mode,
    tag: u8,
    body: VerifierBody<'lease>,
}

#[cfg(test)]
impl<'step, 'tx, 'conn> StepReader<'step, 'tx, 'conn> {
    /// 装入同事务 SQL 后端。SQL 描述符必须借同一活动工作域；异域只拒本次构造，
    /// 尚未进入 owner 核，不撤销 owner 成功资格。
    fn with_sql(
        capture: &'step AuditCapture,
        work: &'step WorkScope,
        sql: managed_sql::ManagedSqlStep<'tx, 'conn, 'step>,
    ) -> Result<Self, CodecError> {
        if !sql.bound_to(work) {
            return Err(work.reject(capture_allocation::scratch::ScratchFailure::Ledger));
        }
        let mut reader = Self::new(capture, work)?;
        reader.sql = Some(sql);
        Ok(reader)
    }

    /// 单帧读：驻留命中不触发任何 SQL；未驻留才在同事务点查。缺失与篡改均按原码拒绝。
    fn read_frame(&mut self, digest: &NodeDigest) -> Result<(u8, VerifierBody<'step>), CodecError> {
        let CaptureStep { capture, work } = self.backend;
        if capture.contains(digest) {
            return CodecContext::OwnerCapture { capture, work }.read(digest);
        }
        match self.sql.as_mut() {
            Some(sql) => read_scoped_checked(sql, digest),
            None => CodecContext::OwnerCapture { capture, work }.read(digest),
        }
    }

    fn new(capture: &'step AuditCapture, work: &'step WorkScope) -> Result<Self, CodecError> {
        // 兼容首片包装锁：只核当前域与已存在的索引描述符，不再扫描全表。
        // 索引许可不是全部正文同父的证明；此借用后端仍 pin 原 capture。
        // 新 SQL 域由 managed_sql 的封闭 Transaction/父账描述符签发。
        let result = (|| {
            work.check_current()?;
            let CaptureNodes::Native(native) = &capture.nodes else {
                return Err(work.reject(capture_allocation::scratch::ScratchFailure::Ledger));
            };
            if native
                .index_permit
                .as_ref()
                .is_some_and(|p| !p.belongs_to_parent(work))
            {
                return Err(work.reject(capture_allocation::scratch::ScratchFailure::Ledger));
            }
            Ok(Self {
                backend: CaptureStep { capture, work },
                sql: None,
            })
        })();
        // 域描述符拒绝尚未进入 owner 核，不能增添撤销成功资格的语义。
        result
    }

    fn read_checked<'lease>(
        &'lease mut self,
        reference: RefView<'lease>,
        mode: Mode,
    ) -> Result<ReadLease<'lease>, CodecError> {
        // 原核负责完整四字段、首次可达、Value/JsonInner 及 Public/Template 隔离。
        // 这里不建立序列成功表，也不从单帧或摘要签发认证资格。
        let result = (|| {
            self.backend
                .capture
                .verify_owner_refview(reference, mode, self.backend.work)?;
            let digest = reference.digest()?;
            let (tag, body) = CodecContext::OwnerCapture {
                capture: self.backend.capture,
                work: self.backend.work,
            }
            .read(&digest)?;
            Ok(ReadLease {
                reference,
                flavor: Flavor::Value,
                mode,
                tag,
                body,
            })
        })();
        if result.is_err() {
            // 沿用 owner 成功能力撤销核，不清 typed origin、不换源重试。
            self.backend.capture.owner_codec.borrow_mut().revoke();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capture_allocation::scratch::{
        failure_trace, CodecWork, ScratchCharge, ScratchFailure, ScratchParent,
    };
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Reserve(usize, u8, u64),
        Release(usize, u8, u64),
        Close(u64),
    }

    struct Parent {
        live: AtomicUsize,
        peak: AtomicUsize,
        origin: AtomicU64,
        fail_native: AtomicBool,
        events: Mutex<Vec<Event>>,
    }
    fn charge(kind: ScratchCharge) -> u8 {
        match kind {
            ScratchCharge::Metadata => 0,
            ScratchCharge::Layout => 1,
            ScratchCharge::Native => 2,
        }
    }
    impl Parent {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                live: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                origin: AtomicU64::new(7),
                fail_native: AtomicBool::new(false),
                events: Mutex::new(Vec::new()),
            })
        }
    }
    impl ScratchParent for Parent {
        fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
            let origin = self.origin();
            self.events
                .lock()
                .unwrap()
                .push(Event::Reserve(bytes, charge(kind), origin));
            if matches!(kind, ScratchCharge::Native) && self.fail_native.load(Ordering::SeqCst) {
                return Err(ScratchFailure::Budget);
            }
            let live = self.live.fetch_add(bytes, Ordering::SeqCst) + bytes;
            self.peak.fetch_max(live, Ordering::SeqCst);
            Ok(origin)
        }
        fn release(
            &self,
            bytes: usize,
            kind: ScratchCharge,
            origin: u64,
        ) -> Result<(), ScratchFailure> {
            self.events
                .lock()
                .unwrap()
                .push(Event::Release(bytes, charge(kind), origin));
            assert!(self.live.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
            // 旧 origin 的真实载荷仍按原 origin 释放，不返新域额度。
            Ok(())
        }
        fn origin(&self) -> u64 {
            self.origin.load(Ordering::SeqCst)
        }
        fn close_scope(&self, origin: u64) -> Result<(), ScratchFailure> {
            self.events.lock().unwrap().push(Event::Close(origin));
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            Ok(())
        }
        fn invalidate(&self) {
            panic!("租约对拍不应破坏父账")
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Output {
        root: NodeDigest,
        kind: Kind,
        extent: Option<u64>,
        flavor: Flavor,
        mode: Mode,
        tag: u8,
        body: Vec<u8>,
    }
    fn output(lease: ReadLease<'_>) -> Output {
        Output {
            root: lease.reference.digest().unwrap(),
            kind: lease.reference.kind(),
            extent: lease.reference.extent(),
            flavor: lease.flavor,
            mode: lease.mode,
            tag: lease.tag,
            body: lease.body.to_vec(),
        }
    }
    /// 包装透明基线：与接口同核同序列，不是独立生产路径差分。
    fn wrapper_baseline(
        capture: &AuditCapture,
        work: &WorkScope,
        reference: RefView<'_>,
        mode: Mode,
    ) -> Result<Output, CodecError> {
        let result = (|| {
            capture.verify_owner_refview(reference, mode, work)?;
            let digest = reference.digest()?;
            let (tag, body) = CodecContext::OwnerCapture { capture, work }.read(&digest)?;
            Ok(Output {
                root: digest,
                kind: reference.kind(),
                extent: reference.extent(),
                flavor: Flavor::Value,
                mode,
                tag,
                body: body.to_vec(),
            })
        })();
        if result.is_err() {
            capture.owner_codec.borrow_mut().revoke();
        }
        result
    }

    #[derive(Clone, Copy)]
    enum Fault {
        None,
        NativeBudget,
        OriginChanged,
    }
    #[derive(Debug, PartialEq, Eq)]
    struct Receipt {
        outputs: Vec<Result<Output, CodecError>>,
        events: Vec<Event>,
        reads: u64,
        frames: u64,
        peak: usize,
        trace: (usize, usize, usize, usize, usize),
    }
    fn exercise(
        nodes: MemoryNodes,
        references: &[(ContentRef, Mode)],
        fault: Fault,
        via_lease: bool,
    ) -> Receipt {
        let capture = AuditCapture::new(nodes)
            .enable_native_frame_reuse()
            .unwrap();
        let parent = Parent::new();
        let before = frame_calls_on_thread();
        let ((outputs, bridges, _), trace) = failure_trace::capture(|| {
            let work = CodecWork::new(parent.clone(), 32 << 20).unwrap();
            let mut outputs = Vec::new();
            let mut bridges = Vec::new();
            let result = work.run(|scope| -> Result<(), CodecError> {
                // 两臂同一资源起点；原 owner codec 许可全部由原核取得。
                if via_lease {
                    let mut reader = StepReader::new(&capture, scope)?;
                    set_fault(&parent, fault);
                    for (reference, mode) in references {
                        let result = reader.read_checked(reference.into(), *mode).map(output);
                        if let Some(bridge) = record(&result) {
                            bridges.push(bridge);
                        }
                        let failed = result.is_err();
                        outputs.push(result);
                        if failed {
                            break;
                        }
                    }
                } else {
                    set_fault(&parent, fault);
                    for (reference, mode) in references {
                        let result = wrapper_baseline(&capture, scope, reference.into(), *mode);
                        if let Some(bridge) = record(&result) {
                            bridges.push(bridge);
                        }
                        let failed = result.is_err();
                        outputs.push(result);
                        if failed {
                            break;
                        }
                    }
                }
                match outputs.last() {
                    Some(Err(error)) => Err(error.clone()),
                    _ => Ok(()),
                }
            });
            match (&result, outputs.last()) {
                (Ok(()), Some(Ok(_))) => {}
                (
                    Err(capture_allocation::scratch::WorkError::Resource(failure)),
                    Some(Err(error)),
                ) => {
                    assert_eq!(Some(*failure), error.scratch_failure());
                }
                (
                    Err(capture_allocation::scratch::WorkError::Operation(error)),
                    Some(Err(original)),
                ) => {
                    assert_eq!(error, original);
                    assert_eq!(error.origin, original.origin);
                }
                other => panic!("首错必须原样传播：{other:?}"),
            }
            (outputs, bridges, result)
        });
        for bridge in bridges {
            assert!(trace.bridges(&bridge));
        }
        if outputs.iter().any(Result::is_err) {
            assert!(capture.owner_codec.borrow().revoked);
            assert_eq!(capture.owner_codec_retained_bytes(), 0);
        }
        // 首错即停，不在失败域重试。owner 证书仍按原生命周期保留；
        // 真正 drop capture 后全部释放。
        let reads = capture.reads.get();
        drop(capture);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        let events = parent.events.lock().unwrap().clone();
        Receipt {
            outputs,
            events,
            reads,
            frames: frame_calls_on_thread() - before,
            peak: parent.peak.load(Ordering::SeqCst),
            trace: (
                trace.budget,
                trace.allocation,
                trace.ledger,
                trace.semantic,
                trace.bridged,
            ),
        }
    }
    fn set_fault(parent: &Parent, fault: Fault) {
        match fault {
            Fault::None => {}
            Fault::NativeBudget => parent.fail_native.store(true, Ordering::SeqCst),
            Fault::OriginChanged => parent.origin.store(8, Ordering::SeqCst),
        }
    }
    fn record(result: &Result<Output, CodecError>) -> Option<ContextError> {
        if let Err(error) = result {
            // PartialEq 不比较 origin；另核 bridge 可回指当前真实资源错误。
            if let Some(failure) = error.scratch_failure() {
                assert!(error.origin.is_some());
                if matches!(failure, ScratchFailure::Budget | ScratchFailure::Allocation) {
                    return Some(ContextError::from_codec(error.clone(), ""));
                }
            }
        }
        None
    }
    fn pair(nodes: MemoryNodes, refs: &[(ContentRef, Mode)], fault: Fault) -> Receipt {
        let expected = exercise(nodes.clone(), refs, fault, false);
        let actual = exercise(nodes, refs, fault, true);
        assert_eq!(actual, expected);
        actual
    }

    fn managed(parent: &Arc<Parent>, tag: u8, bytes: &[u8]) -> (AuditCapture, NodeDigest) {
        let mut capture = AuditCapture::new(MemoryNodes::new())
            .enable_native_frame_reuse()
            .unwrap();
        // 两个独立父账使用同一随机哈希布局，排除 HashMap 随机析构顺序的夹具噪声。
        static HASHER: std::sync::OnceLock<std::collections::hash_map::RandomState> =
            std::sync::OnceLock::new();
        let CaptureNodes::Native(native) = &mut capture.nodes else {
            unreachable!()
        };
        native.nodes = std::collections::HashMap::with_hasher(
            HASHER
                .get_or_init(std::collections::hash_map::RandomState::new)
                .clone(),
        );
        let digest = append_managed(&mut capture, parent, tag, bytes);
        (capture, digest)
    }
    fn append_managed(
        capture: &mut AuditCapture,
        parent: &Arc<Parent>,
        tag: u8,
        bytes: &[u8],
    ) -> NodeDigest {
        let digest = frame(tag, bytes);
        CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                let mut body = CodecVec::default();
                body.try_reserve_native(scope, bytes.len())?;
                for byte in bytes {
                    body.try_push_native(*byte, scope)?;
                }
                capture.append_original_codec_for_owner(digest, tag, body, usize::MAX, scope)?;
                Ok::<_, CodecError>(())
            })
            .unwrap();
        digest
    }

    #[test]
    fn m8a_step_reader_managed_backing_same_parent_new_origin_and_role_charge() {
        let mut receipts = Vec::new();
        for via_lease in [false, true] {
            let parent = Parent::new();
            let (mut capture, empty) = managed(&parent, tag::SEQ_EMPTY, b"");
            let mut bytes = 0u64.to_be_bytes().to_vec();
            bytes.extend_from_slice(&empty);
            let digest = append_managed(&mut capture, &parent, tag::STRING, &bytes);
            let root = ContentRef::new(Kind::String, &digest, Some(0));
            let claims = capture.native_source_claim_totals().unwrap();
            assert_eq!(claims, (2, 40 + 2 * 97));
            let CaptureNodes::Native(native) = &capture.nodes else {
                unreachable!()
            };
            let capacity = native.nodes[&digest].body.capacity();
            assert!(capacity >= 40);
            assert_eq!(native.body_capacity, capacity);
            parent.origin.store(8, Ordering::SeqCst);
            let value = CodecWork::new(parent.clone(), 32 << 20)
                .unwrap()
                .run(|scope| {
                    if via_lease {
                        StepReader::new(&capture, scope)?
                            .read_checked((&root).into(), Mode::Public)
                            .map(output)
                    } else {
                        wrapper_baseline(&capture, scope, (&root).into(), Mode::Public)
                    }
                })
                .unwrap();
            assert_eq!(value.body, bytes);
            assert_eq!(capture.native_source_claim_totals().unwrap(), claims);
            drop(capture);
            assert_eq!(parent.live.load(Ordering::SeqCst), 0);
            let events = parent.events.lock().unwrap().clone();
            assert!(events.iter().any(|e| matches!(e, Event::Release(_, _, 7))));
            assert!(events.iter().any(|e| matches!(e, Event::Release(_, _, 8))));
            receipts.push((value, events, parent.peak.load(Ordering::SeqCst)));
        }
        assert_eq!(receipts[0], receipts[1]);
    }

    #[test]
    fn m8a_step_reader_foreign_descriptor_rejects_request_without_revoking_owner() {
        let original = Parent::new();
        let foreign = Parent::new();
        let (capture, digest) = managed(&original, tag::BOOL_TRUE, b"");
        let root = ContentRef::new(Kind::Bool, &digest, None);
        CodecWork::new(original.clone(), 32 << 20)
            .unwrap()
            .run(|scope| capture.verify_owner_refview((&root).into(), Mode::Public, scope))
            .unwrap();
        let success = capture.owner_codec_retained_bytes();
        assert!(success > 0);
        let retained = original.live.load(Ordering::SeqCst);
        let result = CodecWork::new(foreign.clone(), 32 << 20)
            .unwrap()
            .run(|scope| StepReader::new(&capture, scope).map(|_| ()));
        assert!(matches!(
            result,
            Err(capture_allocation::scratch::WorkError::Resource(
                ScratchFailure::Ledger
            ))
        ));
        assert!(!capture.owner_codec.borrow().revoked);
        assert_eq!(capture.owner_codec_retained_bytes(), success);
        assert_eq!(original.live.load(Ordering::SeqCst), retained);
        assert_eq!(foreign.live.load(Ordering::SeqCst), 0);
        CodecWork::new(original.clone(), 32 << 20)
            .unwrap()
            .run(|scope| capture.verify_owner_refview((&root).into(), Mode::Public, scope))
            .unwrap();
        drop(capture);
        assert_eq!(original.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn m8a_step_reader_duplicate_actual_original_compare_revokes_owner_success() {
        let parent = Parent::new();
        let (mut capture, digest) = managed(&parent, tag::BOOL_TRUE, b"");
        let root = ContentRef::new(Kind::Bool, &digest, None);
        CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                StepReader::new(&capture, scope)?
                    .read_checked((&root).into(), Mode::Public)
                    .map(|_| ())
            })
            .unwrap();
        assert!(capture.owner_codec_retained_bytes() > 0);
        // 只在测试破坏旧实际帧：新输入摘要正确，命中原插入核的同摘要异原件逐字比较。
        let CaptureNodes::Native(native) = &mut capture.nodes else {
            unreachable!()
        };
        native.nodes.get_mut(&digest).unwrap().tag = tag::BOOL_FALSE;
        let result = CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                let mut incoming = CodecVec::default();
                incoming.try_reserve_native(scope, 0)?;
                capture.append_original_codec_for_owner(
                    digest,
                    tag::BOOL_TRUE,
                    incoming,
                    usize::MAX,
                    scope,
                )
            });
        let Err(capture_allocation::scratch::WorkError::Operation(error)) = result else {
            panic!("应由原同摘要异原件核拒绝")
        };
        assert_eq!(error.code(), code::NODE_DIGEST);
        assert_eq!(error.detail(), "同摘要异原件");
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(capture.owner_codec_retained_bytes(), 0);
        let reads = capture.reads.get();
        let result = CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                StepReader::new(&capture, scope)?
                    .read_checked((&root).into(), Mode::Public)
                    .map(|_| ())
            });
        let Err(capture_allocation::scratch::WorkError::Operation(error)) = result else {
            panic!("新步不得恢复被撤销能力")
        };
        assert_eq!(error.code(), code::SOURCE);
        assert_eq!(capture.reads.get(), reads);
        drop(capture);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn m8a_step_reader_body_context_and_permit_events_match_capture() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::Array(vec![Value::string("正文"); 4]))
            .unwrap()
            .content_ref();
        let receipt = pair(
            nodes,
            &[(root.clone(), Mode::Public), (root, Mode::Public)],
            Fault::None,
        );
        assert!(receipt.outputs.iter().all(Result::is_ok));
        assert!(receipt.reads > 0);
        assert!(receipt.frames > 0);
    }

    #[test]
    fn m8a_step_reader_full_reference_mode_and_json_flavor_keep_first_errors() {
        let mut nodes = MemoryNodes::new();
        let placeholder = write_template(&mut nodes, &Value::BatchSelf)
            .unwrap()
            .content_ref();
        let receipt = pair(
            nodes,
            &[
                (placeholder.clone(), Mode::Template),
                (placeholder, Mode::Public),
            ],
            Fault::None,
        );
        assert_eq!(
            receipt.outputs[1].as_ref().unwrap_err().code(),
            code::TEMPLATE_PLACEHOLDER
        );

        let mut nodes = MemoryNodes::new();
        let inner = write_value(&mut nodes, &Value::JsonText(Box::new(JsonValue::Null))).unwrap();
        let outer_digest = frame(tag::JSON_TEXT, &inner.digest);
        nodes
            .nodes
            .insert(outer_digest, (tag::JSON_TEXT, inner.digest.to_vec()));
        let outer = ContentRef::new(Kind::JsonText, &outer_digest, None);
        let receipt = pair(
            nodes,
            &[(inner.content_ref(), Mode::Public), (outer, Mode::Public)],
            Fault::None,
        );
        assert_eq!(
            receipt.outputs[1].as_ref().unwrap_err().code(),
            code::JSON_TEXT_INTERNAL_KIND
        );

        for field in 0..4 {
            let mut nodes = MemoryNodes::new();
            let good = write_value(&mut nodes, &Value::string("x"))
                .unwrap()
                .content_ref();
            let mut bad = good.clone();
            let expected = match field {
                0 => {
                    bad.format.push('x');
                    code::FORMAT
                }
                1 => {
                    bad.root.push('x');
                    code::ROOT
                }
                2 => {
                    bad.kind = Kind::Null;
                    code::KIND
                }
                _ => {
                    bad.extent = Some(99);
                    code::EXTENT
                }
            };
            let receipt = pair(
                nodes,
                &[(good.clone(), Mode::Public), (bad, Mode::Public)],
                Fault::None,
            );
            assert_eq!(receipt.outputs[1].as_ref().unwrap_err().code(), expected);
            assert_eq!(receipt.outputs.len(), 2);
        }
    }

    #[test]
    fn m8a_step_reader_missing_corrupt_and_partial_success_revoke_without_retry() {
        for missing in [false, true] {
            let mut nodes = MemoryNodes::new();
            let good = write_value(&mut nodes, &Value::Null).unwrap().content_ref();
            let child = write_value(&mut nodes, &Value::string("缺失或篡改")).unwrap();
            let bad = write_value(&mut nodes, &Value::Array(vec![Value::Edge(child.digest)]))
                .unwrap()
                .content_ref();
            if missing {
                nodes.nodes.remove(&child.digest);
            } else {
                nodes.nodes.get_mut(&child.digest).unwrap().1.push(0);
            }
            let receipt = pair(
                nodes,
                &[(good.clone(), Mode::Public), (bad, Mode::Public)],
                Fault::None,
            );
            assert_eq!(
                receipt.outputs[1].as_ref().unwrap_err().code(),
                if missing {
                    code::NODE_MISSING
                } else {
                    code::NODE_DIGEST
                }
            );
            assert_eq!(receipt.outputs.len(), 2);
        }
    }

    #[test]
    fn m8a_step_reader_budget_and_origin_failures_preserve_typed_source_and_release() {
        for fault in [Fault::NativeBudget, Fault::OriginChanged] {
            let mut nodes = MemoryNodes::new();
            let root = write_value(&mut nodes, &Value::Null).unwrap().content_ref();
            let receipt = pair(nodes, &[(root, Mode::Public)], fault);
            let expected = match fault {
                Fault::NativeBudget => ScratchFailure::Budget,
                _ => ScratchFailure::Ledger,
            };
            assert_eq!(
                receipt.outputs[0].as_ref().unwrap_err().scratch_failure(),
                Some(expected)
            );
        }
    }

    fn sql_database(nodes: &MemoryNodes) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL);",
        )
        .unwrap();
        for (digest, (tag, body)) in &nodes.nodes {
            conn.execute(
                "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
                rusqlite::params![&digest[..], i64::from(*tag), body],
            )
            .unwrap();
        }
        conn
    }

    /// 驻留 A 只在 capture；B 只在 SQL；库内另有 A 的同字节行。
    fn resident_and_sql_only() -> (MemoryNodes, MemoryNodes, NodeDigest, NodeDigest) {
        let mut resident = MemoryNodes::new();
        let a = write_value(&mut resident, &Value::string("驻留正文"))
            .unwrap()
            .content_ref()
            .digest()
            .unwrap();
        let mut stored = resident.clone();
        let b = write_value(&mut stored, &Value::string("只在库内的正文"))
            .unwrap()
            .content_ref()
            .digest()
            .unwrap();
        assert!(!resident.nodes.contains_key(&b));
        (resident, stored, a, b)
    }

    #[test]
    fn r1_step_reader_resident_hit_has_no_sql_and_miss_reads_same_transaction() {
        let (resident, stored, a, b) = resident_and_sql_only();
        let expected_b = stored.nodes.get(&b).cloned().unwrap();
        let capture = AuditCapture::new(resident)
            .enable_native_frame_reuse()
            .unwrap();
        let mut conn = sql_database(&stored);
        let tx = conn.transaction().unwrap();
        let parent = Parent::new();
        CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                let sql = managed_sql::ManagedSqlStep::new(&tx, scope, &|| Ok(()))?;
                let mut reader = StepReader::with_sql(&capture, scope, sql)?;
                let (tag, body) = reader.read_frame(&a)?;
                let resident = capture.original(&a).unwrap();
                assert_eq!((tag, &*body), (resident.0, resident.1));
                assert!(matches!(body, VerifierBody::Borrowed(_)));
                assert_eq!(reader.sql.as_ref().unwrap().reads, 0);
                drop(body);
                let (tag, body) = reader.read_frame(&b)?;
                assert!(matches!(body, VerifierBody::Scoped(_)));
                assert_eq!((tag, body.to_vec()), expected_b);
                assert_eq!(reader.sql.as_ref().unwrap().reads, 1);
                // SQL 正文许可与驻留同父：读出期间 live 增长，析构后归还。
                assert!(parent.live.load(Ordering::SeqCst) >= expected_b.1.len());
                drop(body);
                let missing = reader.read_frame(&[9; 32]).err().unwrap();
                assert_eq!(missing.code(), code::NODE_MISSING);
                assert_eq!(reader.sql.as_ref().unwrap().reads, 2);
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert!(!capture.owner_codec.borrow().revoked);
        drop(capture);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn r1_step_reader_without_sql_backend_keeps_resident_missing_error() {
        let (resident, _, a, b) = resident_and_sql_only();
        let capture = AuditCapture::new(resident)
            .enable_native_frame_reuse()
            .unwrap();
        let parent = Parent::new();
        CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                let mut reader = StepReader::new(&capture, scope)?;
                assert!(reader.read_frame(&a).is_ok());
                assert_eq!(
                    reader.read_frame(&b).err().unwrap().code(),
                    code::NODE_MISSING
                );
                Ok::<_, CodecError>(())
            })
            .unwrap();
    }

    #[test]
    fn r1_step_reader_rejects_sql_descriptor_bound_to_other_work_scope() {
        let (resident, stored, _, _) = resident_and_sql_only();
        let capture = AuditCapture::new(resident)
            .enable_native_frame_reuse()
            .unwrap();
        let mut conn = sql_database(&stored);
        let tx = conn.transaction().unwrap();
        let parent = Parent::new();
        let other = Parent::new();
        CodecWork::new(parent.clone(), 32 << 20)
            .unwrap()
            .run(|scope| {
                let sql = managed_sql::ManagedSqlStep::new(&tx, scope, &|| Ok(()))?;
                let rejected = CodecWork::new(other.clone(), 32 << 20)
                    .unwrap()
                    .run(|foreign| StepReader::with_sql(&capture, foreign, sql).map(|_| ()));
                assert!(matches!(
                    rejected,
                    Err(capture_allocation::scratch::WorkError::Resource(
                        ScratchFailure::Ledger
                    ))
                ));
                Ok::<_, CodecError>(())
            })
            .unwrap();
        // 构造期拒绝不进入 owner 核，不撤销 owner 成功资格。
        assert!(!capture.owner_codec.borrow().revoked);
    }

    #[test]
    fn r1_shared_transaction_write_or_commit_rejects_next_miss_without_select() {
        for commit in [false, true] {
            let (resident, stored, a, b) = resident_and_sql_only();
            let capture = AuditCapture::new(resident)
                .enable_native_frame_reuse()
                .unwrap();
            let mut conn = sql_database(&stored);
            let tx = conn.transaction().unwrap();
            let parent = Parent::new();
            let result = CodecWork::new(parent.clone(), 32 << 20)
                .unwrap()
                .run(|scope| {
                    let sql = managed_sql::ManagedSqlStep::new(&tx, scope, &|| Ok(()))?;
                    let mut reader = StepReader::with_sql(&capture, scope, sql)?;
                    drop(reader.read_frame(&b)?);
                    assert_eq!(reader.sql.as_ref().unwrap().reads, 1);
                    // 同一步写者经同一共享事务句柄写入或提交；读者不能吸收该变化。
                    if commit {
                        tx.execute_batch("COMMIT").unwrap();
                    } else {
                        tx.execute(
                            "INSERT INTO s_content_nodes VALUES(?1,0,?2)",
                            rusqlite::params![&[7u8; 32][..], &b""[..]],
                        )
                        .unwrap();
                    }
                    let first = reader.read_frame(&b).err().unwrap();
                    assert_eq!(first.scratch_failure(), Some(ScratchFailure::Ledger));
                    let again = reader.read_frame(&b).err().unwrap();
                    assert_eq!(again, first);
                    assert_eq!(reader.sql.as_ref().unwrap().reads, 1);
                    // 首错使本工作域失败；驻留读也不再越过失败域放行。
                    assert!(reader.read_frame(&a).is_err());
                    Err::<(), _>(first)
                });
            assert!(result.is_err());
            drop(capture);
            assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        }
    }
}
