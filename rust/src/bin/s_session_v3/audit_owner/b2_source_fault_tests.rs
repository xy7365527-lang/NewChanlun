// #1488 B2：只由 construction_tests include；真实 TxStore 坏件与正式 Publish 拒绝。
// 所有 Rust 执行交根的合法 runner；fixture/oracle 不向 Native 入口供应成功资格。
use crate::content_producer::{
    Cc004Producer as B2FaultProducer, CC004_STEP_KIND as B2_STEP, CC005_GROUP_KIND as B2_GROUP,
    CC006_SHAPE_KIND as B2_SHAPE, CC054_KNOWLEDGE_KIND as B2_KNOWLEDGE,
};
use newchan_rust::s_content as b2sc;

#[derive(Clone, Copy, Debug)]
enum B2SourceFault {
    ObjectBeforeMissingWitness,
    ShapeBeforeBadObservations,
    StepBeforeBadRequests,
    PartitionBeforeBadWitness,
    MissingStepRequest,
    DuplicateStepRequest,
    PendingPrefix,
    WitnessRawBeforeLaterSlot,
    MissingWitnessSlot,
    DeferredGroupBeforeBadIdentity,
    ObservationKind,
    UnsettledOtherGroup,
}
impl B2SourceFault {
    fn reason(self) -> &'static str {
        match self {
            Self::ObjectBeforeMissingWitness | Self::DeferredGroupBeforeBadIdentity => {
                "typed obj3身份与来源/载荷原件不符"
            }
            Self::ShapeBeforeBadObservations => "CC006 obj3与同cut组事实身份不符",
            Self::StepBeforeBadRequests => "CC004 incoming/槽位/OHLC不是同cut原始来源",
            Self::PartitionBeforeBadWitness => "组合并序号不连续",
            Self::MissingStepRequest => "CC004缺一一绑定的知识请求",
            Self::DuplicateStepRequest => "同cut知识请求重复",
            Self::PendingPrefix => "CC004来源不是截至incoming的同cut完整前缀",
            Self::WitnessRawBeforeLaterSlot => "见证raw_bars不是同cut真实原始成员",
            Self::MissingWitnessSlot => "形态缺完整三槽见证",
            Self::ObservationKind => "kind不是文本",
            Self::UnsettledOtherGroup => "初始未定组不是唯一seed原件",
        }
    }
}
struct B2FaultObservation {
    expected: String,
    actual_raw: usize,
    semantic_nodes_added: bool,
}
thread_local! {
    static B2_SOURCE_FAULT: std::cell::Cell<Option<B2SourceFault>> = const { std::cell::Cell::new(None) };
    static B2_SOURCE_FAULT_OBSERVATION: std::cell::RefCell<Option<B2FaultObservation>> = const { std::cell::RefCell::new(None) };
}
struct B2FaultGuard;
impl B2FaultGuard {
    fn enter(fault: B2SourceFault) -> Self {
        B2_SOURCE_FAULT.with(|slot| assert!(slot.replace(Some(fault)).is_none()));
        B2_SOURCE_FAULT_OBSERVATION.with(|slot| assert!(slot.borrow().is_none()));
        Self
    }
}
impl Drop for B2FaultGuard {
    fn drop(&mut self) {
        B2_SOURCE_FAULT.with(|slot| slot.set(None));
        B2_SOURCE_FAULT_OBSERVATION.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
fn b2_fault_error(error: impl std::fmt::Display) -> String {
    format!("B2FaultTest：{error}")
}
fn b2_fault_field(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
    name: &str,
) -> Result<b2sc::ContentRef, String> {
    b2sc::path::lookup_path(store, root, &[b2sc::path::PathStep::Field(name.into())])
        .map_err(b2_fault_error)
}
fn b2_fault_json(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
) -> Result<Value, String> {
    let mut bytes = Vec::new();
    b2sc::render_canonical(store, root, &mut bytes).map_err(b2_fault_error)?;
    newchan_rust::session_protocol::strict_json(&bytes).map_err(b2_fault_error)
}
fn b2_fault_write_json(
    store: &mut crate::content_store::TxStore<'_, '_>,
    value: &Value,
) -> Result<b2sc::ContentRef, String> {
    b2sc::write_json_view(store, value.into())
        .map(|built| built.content_ref())
        .map_err(b2_fault_error)
}
fn b2_fault_edit_fields(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
    edits: Vec<(&str, Option<b2sc::Value>)>,
) -> Result<b2sc::ContentRef, String> {
    let rendered = b2_fault_json(store, root)?;
    let object = rendered
        .as_object()
        .ok_or_else(|| b2_fault_error("需对象原件"))?;
    let mut fields = std::collections::BTreeMap::new();
    for key in object.keys() {
        let field = b2_fault_field(store, root, key)?;
        fields.insert(
            key.clone(),
            b2sc::Value::Edge(field.digest().map_err(b2_fault_error)?),
        );
    }
    for (key, value) in edits {
        if let Some(value) = value {
            fields.insert(key.into(), value);
        } else {
            assert!(fields.remove(key).is_some(), "被删字段必须真实存在");
        }
    }
    b2sc::write_value(store, &b2sc::Value::Object(fields.into_iter().collect()))
        .map(|built| built.content_ref())
        .map_err(b2_fault_error)
}
fn b2_fault_json_inner(
    store: &mut crate::content_store::TxStore<'_, '_>,
    object: &b2sc::ContentRef,
    field: &str,
) -> Result<b2sc::ContentRef, String> {
    let field = b2_fault_field(store, object, field)?;
    b2sc::path::lookup_path(store, &field, &[b2sc::path::PathStep::JsonText])
        .map_err(b2_fault_error)
}
fn b2_fault_replace_json_inner(
    store: &mut crate::content_store::TxStore<'_, '_>,
    object: &b2sc::ContentRef,
    field: &str,
    value: &Value,
) -> Result<b2sc::ContentRef, String> {
    let inner = b2_fault_write_json(store, value)?;
    let body = inner.digest().map_err(b2_fault_error)?;
    let edge = b2sc::frame(b2sc::tag::JSON_TEXT, &body);
    b2sc::NodeSink::put(store, edge, b2sc::tag::JSON_TEXT, &body).map_err(b2_fault_error)?;
    b2_fault_edit_fields(store, object, vec![(field, Some(b2sc::Value::Edge(edge)))])
}
fn b2_fault_built(reference: &b2sc::ContentRef) -> Result<b2sc::Built, String> {
    Ok(b2sc::Built {
        digest: reference.digest().map_err(b2_fault_error)?,
        kind: reference.kind,
        extent: reference.extent,
    })
}
fn b2_fault_reidentify(
    store: &mut crate::content_store::TxStore<'_, '_>,
    object: &b2sc::ContentRef,
) -> Result<b2sc::ContentRef, String> {
    let record = b2_fault_json(store, object)?;
    let kind = record["kind"]
        .as_str()
        .ok_or_else(|| b2_fault_error("kind缺文本"))?;
    let key = record["fact_key"]
        .as_str()
        .ok_or_else(|| b2_fault_error("fact_key缺文本"))?;
    let payload = b2_fault_json_inner(store, object, "payload_json")?;
    let refs = b2_fault_json_inner(store, object, "input_refs_json")?;
    let id = B2FaultProducer::new(tb02a::PROFILE, crate::W_STRUCTURE_RULE_REVISION)
        .typed_object_id(
            kind,
            key,
            &b2_fault_built(&payload)?,
            &b2_fault_built(&refs)?,
        )
        .map_err(b2_fault_error)?;
    b2_fault_edit_fields(
        store,
        object,
        vec![("object_id", Some(b2sc::Value::String(id)))],
    )
}
fn b2_fault_members(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
    field: &str,
) -> Result<Vec<b2sc::ContentRef>, String> {
    let array = b2_fault_field(store, root, field)?;
    b2sc::path::lookup_array_items(store, &array).map_err(b2_fault_error)
}
fn b2_fault_collection(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
    field: &str,
    members: &[b2sc::ContentRef],
) -> Result<b2sc::ContentRef, String> {
    let values = members
        .iter()
        .map(|r| b2sc::Value::edge_of_ref(r).map_err(b2_fault_error))
        .collect::<Result<Vec<_>, _>>()?;
    b2_fault_edit_fields(store, root, vec![(field, Some(b2sc::Value::Array(values)))])
}
fn b2_fault_find_kind(
    store: &mut crate::content_store::TxStore<'_, '_>,
    objects: &[b2sc::ContentRef],
    kind: &str,
) -> Result<usize, String> {
    for (at, object) in objects.iter().enumerate() {
        if b2_fault_json(store, object)?["kind"].as_str() == Some(kind) {
            return Ok(at);
        }
    }
    Err(b2_fault_error(format!("实际生成必须含非空{kind}")))
}
fn b2_fault_bad_id(
    store: &mut crate::content_store::TxStore<'_, '_>,
    object: &b2sc::ContentRef,
) -> Result<b2sc::ContentRef, String> {
    let record = b2_fault_json(store, object)?;
    let id = format!("{}-b2-source-fault", record["object_id"].as_str().unwrap());
    b2_fault_edit_fields(
        store,
        object,
        vec![("object_id", Some(b2sc::Value::String(id)))],
    )
}
fn b2_fault_mutate(
    store: &mut crate::content_store::TxStore<'_, '_>,
    root: &b2sc::ContentRef,
    fault: B2SourceFault,
) -> Result<b2sc::ContentRef, String> {
    let mut root = root.clone();
    let mut objects = b2_fault_members(store, &root, "objects")?;
    match fault {
        B2SourceFault::ObjectBeforeMissingWitness => {
            let at = b2_fault_find_kind(store, &objects, B2_GROUP)?;
            objects[at] = b2_fault_bad_id(store, &objects[at])?;
            root = b2_fault_collection(store, &root, "objects", &objects)?;
            root = b2_fault_edit_fields(store, &root, vec![("witnesses", None)])?;
        }
        B2SourceFault::ShapeBeforeBadObservations => {
            let at = b2_fault_find_kind(store, &objects, B2_SHAPE)?;
            objects[at] = b2_fault_bad_id(store, &objects[at])?;
            root = b2_fault_collection(store, &root, "objects", &objects)?;
            root = b2_fault_edit_fields(
                store,
                &root,
                vec![("observations", Some(b2sc::Value::Null))],
            )?;
        }
        B2SourceFault::StepBeforeBadRequests => {
            let at = b2_fault_find_kind(store, &objects, B2_STEP)?;
            let inner = b2_fault_json_inner(store, &objects[at], "payload_json")?;
            let mut payload = b2_fault_json(store, &inner)?;
            payload["incoming"]["open"] = json!("999999");
            objects[at] =
                b2_fault_replace_json_inner(store, &objects[at], "payload_json", &payload)?;
            objects[at] = b2_fault_reidentify(store, &objects[at])?;
            let at = b2_fault_find_kind(store, &objects, B2_KNOWLEDGE)?;
            let inner = b2_fault_json_inner(store, &objects[at], "payload_json")?;
            let mut payload = b2_fault_json(store, &inner)?;
            payload["requests"] = Value::Null;
            objects[at] =
                b2_fault_replace_json_inner(store, &objects[at], "payload_json", &payload)?;
            objects[at] = b2_fault_reidentify(store, &objects[at])?;
            root = b2_fault_collection(store, &root, "objects", &objects)?;
        }
        B2SourceFault::PartitionBeforeBadWitness | B2SourceFault::UnsettledOtherGroup => {
            let at = b2_fault_find_kind(store, &objects, B2_GROUP)?;
            let inner = b2_fault_json_inner(store, &objects[at], "payload_json")?;
            let mut payload = b2_fault_json(store, &inner)?;
            payload["group_index"] =
                json!(if matches!(fault, B2SourceFault::UnsettledOtherGroup) {
                    "1"
                } else {
                    "1000000"
                });
            let changed =
                b2_fault_replace_json_inner(store, &objects[at], "payload_json", &payload)?;
            let changed = b2_fault_reidentify(store, &changed)?;
            if matches!(fault, B2SourceFault::UnsettledOtherGroup) {
                objects.push(changed);
            } else {
                objects[at] = changed;
            }
            root = b2_fault_collection(store, &root, "objects", &objects)?;
            if matches!(fault, B2SourceFault::PartitionBeforeBadWitness) {
                let mut witnesses = b2_fault_members(store, &root, "witnesses")?;
                assert!(!witnesses.is_empty());
                witnesses[0] = b2_fault_edit_fields(
                    store,
                    &witnesses[0],
                    vec![(
                        "slot",
                        Some(b2sc::Value::Number(
                            b2sc::NumberToken::parse("3").map_err(b2_fault_error)?,
                        )),
                    )],
                )?;
                root = b2_fault_collection(store, &root, "witnesses", &witnesses)?;
            }
        }
        B2SourceFault::MissingStepRequest | B2SourceFault::DuplicateStepRequest => {
            let at = b2_fault_find_kind(store, &objects, B2_KNOWLEDGE)?;
            let inner = b2_fault_json_inner(store, &objects[at], "payload_json")?;
            let mut payload = b2_fault_json(store, &inner)?;
            let requests = payload["requests"].as_array_mut().unwrap();
            let request = requests
                .iter()
                .position(|r| {
                    let key: Value =
                        serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                    key[0].as_str() == Some(B2_STEP)
                })
                .expect("必须有真实CC004请求");
            if matches!(fault, B2SourceFault::MissingStepRequest) {
                requests.remove(request);
            } else {
                requests.push(requests[request].clone());
            }
            objects[at] =
                b2_fault_replace_json_inner(store, &objects[at], "payload_json", &payload)?;
            objects[at] = b2_fault_reidentify(store, &objects[at])?;
            root = b2_fault_collection(store, &root, "objects", &objects)?;
        }
        B2SourceFault::PendingPrefix => {
            let mut found = false;
            for object in &mut objects {
                if b2_fault_json(store, object)?["kind"].as_str() != Some(B2_STEP) {
                    continue;
                }
                let coords = b2_fault_json_inner(store, object, "source_coords_json")?;
                let mut coords = b2_fault_json(store, &coords)?;
                if coords.as_array().unwrap().len() < 2 {
                    continue;
                }
                coords.as_array_mut().unwrap().remove(0);
                let refs = b2_fault_json_inner(store, object, "input_refs_json")?;
                let mut refs = b2_fault_json(store, &refs)?;
                refs.as_array_mut().unwrap().remove(0);
                *object =
                    b2_fault_replace_json_inner(store, object, "source_coords_json", &coords)?;
                *object = b2_fault_replace_json_inner(store, object, "input_refs_json", &refs)?;
                *object = b2_fault_reidentify(store, object)?;
                found = true;
                break;
            }
            assert!(found, "必须有长度至少2的真实step前缀");
            root = b2_fault_collection(store, &root, "objects", &objects)?;
        }
        B2SourceFault::WitnessRawBeforeLaterSlot => {
            let mut witnesses = b2_fault_members(store, &root, "witnesses")?;
            assert!(!witnesses.is_empty());
            let last_bad = b2_fault_edit_fields(
                store,
                &witnesses[0],
                vec![(
                    "slot",
                    Some(b2sc::Value::Number(
                        b2sc::NumberToken::parse("3").map_err(b2_fault_error)?,
                    )),
                )],
            )?;
            let bars = b2_fault_field(store, &witnesses[0], "raw_bars")?;
            let mut bars = b2_fault_json(store, &bars)?;
            assert!(!bars.as_array().unwrap().is_empty());
            bars[0]["payload_hash"] = json!("b2-different-original");
            let changed = b2_fault_write_json(store, &bars)?;
            witnesses[0] = b2_fault_edit_fields(
                store,
                &witnesses[0],
                vec![(
                    "raw_bars",
                    Some(b2sc::Value::edge_of_ref(&changed).map_err(b2_fault_error)?),
                )],
            )?;
            witnesses.push(last_bad);
            root = b2_fault_collection(store, &root, "witnesses", &witnesses)?;
        }
        B2SourceFault::MissingWitnessSlot => {
            let mut witnesses = b2_fault_members(store, &root, "witnesses")?;
            assert!(!witnesses.is_empty());
            witnesses.remove(0);
            root = b2_fault_collection(store, &root, "witnesses", &witnesses)?;
        }
        B2SourceFault::DeferredGroupBeforeBadIdentity => {
            let at = b2_fault_find_kind(store, &objects, B2_GROUP)?;
            let original = objects[at].clone();
            let inner = b2_fault_json_inner(store, &original, "payload_json")?;
            let mut payload = b2_fault_json(store, &inner)?;
            payload["high"] = json!("999999");
            let changed = b2_fault_replace_json_inner(store, &original, "payload_json", &payload)?;
            let changed = b2_fault_reidentify(store, &changed)?;
            let bad = b2_fault_bad_id(store, &original)?;
            objects.insert(0, bad);
            objects.insert(0, changed);
            root = b2_fault_collection(store, &root, "objects", &objects)?;
        }
        B2SourceFault::ObservationKind => {
            let mut observations = b2_fault_members(store, &root, "observations")?;
            observations.push(b2_fault_write_json(store, &json!({"kind":7}))?);
            root = b2_fault_collection(store, &root, "observations", &observations)?;
        }
    }
    Ok(root)
}
fn b2_source_fault_hook(
    tx: &Transaction<'_>,
    root: &b2sc::ContentRef,
) -> Result<b2sc::ContentRef, String> {
    let fault = B2_SOURCE_FAULT.with(|slot| slot.get().unwrap());
    let raw = crate::read_raw_events(tx).map_err(b2_fault_error)?;
    let before_nodes: i64 = tx
        .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
        .map_err(b2_fault_error)?;
    let mut store = crate::content_store::TxStore::new(tx).map_err(b2_fault_error)?;
    let base = root.clone();
    {
        let mut reader = super::super::history::HashReader::new(&mut store);
        source_audit::audit_generated_frozen_for_test(
            &mut reader,
            &base,
            &raw,
            tb02a::PROFILE,
            crate::W_STRUCTURE_RULE_REVISION,
            None,
            None,
        )
        .expect("注入前实际原件必须通过完整来源核");
    }
    let changed = b2_fault_mutate(&mut store, &base, fault)?;
    b2sc::verify(&mut store, &changed).expect("坏件仅破坏来源语义，完整Public物料必须合法");
    let expected = {
        let mut reader = super::super::history::HashReader::new(&mut store);
        source_audit::audit_generated_frozen_for_test(
            &mut reader,
            &changed,
            &raw,
            tb02a::PROFILE,
            crate::W_STRUCTURE_RULE_REVISION,
            None,
            None,
        )
        .unwrap_err()
    };
    assert!(expected.contains(fault.reason()), "{fault:?}: {expected}");
    drop(store);
    let after_nodes: i64 = tx
        .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
        .map_err(b2_fault_error)?;
    assert!(
        after_nodes > before_nodes,
        "必须实际向TxStore写新合法摘要原件"
    );
    B2_SOURCE_FAULT_OBSERVATION.with(|slot| {
        assert!(slot
            .borrow_mut()
            .replace(B2FaultObservation {
                expected,
                actual_raw: raw.len(),
                semantic_nodes_added: after_nodes > before_nodes
            })
            .is_none());
    });
    // oracle在真实核之前完成；正式热路计数只能覆盖下面的material/S/C2调用。
    source_audit::reset_source_call_stats();
    construction_set::take_owner_source_reuses();
    Ok(changed)
}
fn b2_run_source_fault(fault: B2SourceFault) {
    use super::super::prepared::tests::{runtime_root_tables, Fixture};
    let target = Fixture::new();
    let mut ordinary = Fixture::new();
    let owner = AuditConnection::open(&target.db_path()).unwrap();
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let mut c = context();
    let _guard = B2FaultGuard::enter(fault);
    let rows = if matches!(fault, B2SourceFault::UnsettledOtherGroup) {
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 6, 12, 3, 7, 1),
            ("c", 20, 6, 12, 3, 7, 1),
            ("d", 30, 6, 12, 3, 7, 1),
        ]
    } else {
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 8, 16, 6, 12, 1),
            ("c", 20, 8, 13, 4, 7, 1),
            ("b", 10, 8, 17, 6, 12, 2),
        ]
    };
    for (offset, (id, coord, open, high, low, close, revision)) in rows.into_iter().enumerate() {
        let n = offset as i64 + 1;
        let (envelope, plain_token) =
            ordinary.accept_begin(n, (id, coord, open, high, low, close), revision);
        // 先证明本轮无变异控制流可正常落库。
        ordinary.publish(&envelope, &plain_token).unwrap();
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        if n < 4 {
            let (_, ended) = owner
                .publish(&mut runtime, &token, &envelope, &mut c)
                .unwrap();
            owner.continue_with(ended).unwrap();
            continue;
        }
        let before = runtime_root_tables(&target.conn, true);
        runtime.set_before_native_source_for_test(Some(b2_source_fault_hook));
        let error = match owner.publish(&mut runtime, &token, &envelope, &mut c) {
            Err(error) => error,
            Ok(_) => panic!("{fault:?}: 真实来源坏件意外成功"),
        };
        let observation = B2_SOURCE_FAULT_OBSERVATION
            .with(|slot| slot.borrow_mut().take())
            .unwrap_or_else(|| panic!("{fault:?}: 真实TxStore注入hook必须已运行；publish 错误：{error}"));
        assert!(observation.semantic_nodes_added);
        assert_eq!(
            error, observation.expected,
            "{fault:?}: 必须命中SOURCE并保留完整首错字符串，不能用EFFECTS/预算失败代替"
        );
        assert_eq!(
            source_audit::source_call_stats(),
            source_audit::SourceCallStats::default(),
            "{fault:?}: 失败也不能调用完整来源核/fixture兜底"
        );
        assert_eq!(
            construction_set::take_owner_source_reuses(),
            0,
            "失败来源不能封source成功证明"
        );
        assert!(owner.connection.is_autocommit());
        assert!(owner.complete.borrow().is_none());
        assert!(owner.construction.borrow().is_none());
        assert!(owner.construction_lease.stale.get());
        assert!(runtime.visit_runtime_roots().unwrap().is_empty());
        assert_eq!(
            before,
            runtime_root_tables(&target.conn, true),
            "{fault:?}: 全表含注入原件必须回滚"
        );
        assert_eq!(meta_i64(&target.conn, "generation").unwrap(), 3);
        let rows: i64 = target
            .conn
            .query_row("SELECT count(*) FROM raw_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            rows as usize, observation.actual_raw,
            "已Accept的raw仍持久存在"
        );
        eprintln!(
            "B2_SOURCE_FAULT case={fault:?} class=SOURCE current_raw={} error={error}",
            observation.actual_raw
        );
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        assert!(
            active.with_bound_raw_advance(&c, |_| Ok(())).is_err(),
            "失败owner不能重签raw资格"
        );
    }
}
#[test]
fn b2_native_source_phase_errors_precede_later_bad_material() {
    for fault in [
        B2SourceFault::ObjectBeforeMissingWitness,
        B2SourceFault::ShapeBeforeBadObservations,
        B2SourceFault::StepBeforeBadRequests,
        B2SourceFault::PartitionBeforeBadWitness,
    ] {
        b2_run_source_fault(fault);
    }
}
#[test]
fn b2_native_source_global_dependencies_match_full_oracle_and_rollback() {
    for fault in [
        B2SourceFault::MissingStepRequest,
        B2SourceFault::DuplicateStepRequest,
        B2SourceFault::PendingPrefix,
        B2SourceFault::MissingWitnessSlot,
        B2SourceFault::ObservationKind,
    ] {
        b2_run_source_fault(fault);
    }
}
#[test]
fn b2_native_source_future_duplicates_preserve_earlier_actual_fault() {
    for fault in [
        B2SourceFault::DeferredGroupBeforeBadIdentity,
        B2SourceFault::WitnessRawBeforeLaterSlot,
    ] {
        b2_run_source_fault(fault);
    }
}
#[test]
fn b2_native_source_unsettled_rechecks_added_nonzero_group_before_partition() {
    b2_run_source_fault(B2SourceFault::UnsettledOtherGroup);
}
