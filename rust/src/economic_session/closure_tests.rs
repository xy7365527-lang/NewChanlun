//! #1394 第三问选项 A：financial-closure 客户端测试锁。
//! 合成一个满足 `validate_headers` 的七字段 FinancialBasis（二十字段 snapshot、六族数组、跨块大字符串），
//! 由“诚实伪 S”按协议（摘要升序、显式游标、按字节预算贪心装块）供块，再逐项篡改。每个负例都核：
//! 拒绝；本次尝试写下的暂存全部回滚（stage 恢复到尝试前）；不回落逐片路径。
use super::*;
use crate::s_content::{job::Existing, MemoryNodes, NodeSink, Value as V};
use crate::session_protocol::canonical_json;
use std::collections::BTreeMap;
use std::time::Duration;

const SID: &str = "s-session-testonly-001";

fn resources_json(wire: &str, nodes: &str, node_bytes: &str, staged: &str) -> Value {
    json!({"schema_revision":"s-financial-reader-resources/1",
        "max_request_bytes":"1048576","max_reply_bytes":"1048576",
        "max_stderr_bytes":"1048576","request_timeout_ms":"30000",
        "job_limits":{"max_total_wire_bytes":wire, "max_total_node_bytes":node_bytes,
            "max_total_nodes":nodes, "max_total_records":"1000000",
            "max_total_requests":"100000", "max_elapsed_ms":"180000",
            "max_staged_bytes":staged, "max_store_bytes":staged}})
}
fn pins_with(resources: Value) -> SourcePins {
    SourcePins::from_structure(&json!({"binary":"/frozen/s","binary_sha256":"0".repeat(64),
        "db":"/frozen/db","session_id":SID,"session_generation":"1","writer_epoch":"1",
        "reader_revision":"s-financial-basis/3","content_format":"s-content/1",
        "reader_resources":resources}))
    .unwrap()
    .with_session("/tmp/s1394/s.sock")
    .unwrap()
}
fn pins() -> SourcePins {
    pins_with(resources_json(
        "100000000",
        "1000000",
        "100000000",
        "100000000",
    ))
}
fn job_for(pins: &SourcePins) -> Job {
    Job::new(
        pins.resources.job_limits,
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(Instant::now),
    )
    .unwrap()
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

fn text_of(seed: usize, len: usize) -> V {
    V::string(
        (0..len)
            .map(|k| char::from(b'a' + u8::try_from((k * 7 + seed * 13) % 26).unwrap()))
            .collect::<String>(),
    )
}
fn row(i: usize, blob: usize) -> V {
    V::Object(vec![
        ("object_id".into(), V::string(format!("obj3-{i:04}"))),
        ("payload".into(), text_of(i, blob)),
    ])
}
/// 七字段 FinancialBasis；objects 里有跨 4096 字节块的大字符串，六族都是数组。
fn basis() -> (MemoryNodes, ContentRef) {
    let rows = |n: usize, base: usize, blob: usize| {
        V::Array((0..n).map(|i| row(base + i, blob)).collect())
    };
    let s = |x: &str| V::string(x);
    let snapshot = V::Object(vec![
        ("session_id".into(), s(SID)),
        ("generation".into(), s("1")),
        ("structure_cut".into(), s("cut-1")),
        ("catalog_revision".into(), s("rev-1")),
        ("scope".into(), s("scope-1")),
        ("index_frontier".into(), s("batch-1")),
        ("profile_id".into(), s("profile-1")),
        ("profile_hash".into(), s("hash-1")),
        ("history_mode".into(), s("RecomputedWithRevision")),
        ("as_of_generation".into(), V::Null),
        ("objects".into(), rows(6, 0, 9000)),
        ("withdrawn_objects".into(), rows(2, 100, 50)),
        ("witnesses".into(), rows(3, 200, 300)),
        ("relations".into(), rows(2, 300, 40)),
        ("observations".into(), V::Array(Vec::new())),
        ("raw_history".into(), rows(4, 400, 700)),
        ("input_frontier".into(), s("3")),
        ("seq_range".into(), V::Array(vec![s("0"), s("3")])),
        ("catalog_run_status".into(), s("ok")),
        (
            "catalog_evidence".into(),
            V::Object(vec![("run".into(), s("r"))]),
        ),
    ]);
    let basis = V::Object(vec![
        ("kind".into(), s("FinancialBasis")),
        ("session_id".into(), s(SID)),
        ("session_generation".into(), s("1")),
        ("writer_epoch".into(), s("1")),
        ("pending_revision".into(), V::Bool(false)),
        ("commit_ns".into(), s("77")),
        ("snapshot".into(), snapshot),
    ]);
    let mut store = MemoryNodes::new();
    let root = sc::write_value(&mut store, &basis).unwrap().content_ref();
    (store, root)
}
fn evidence(root: &ContentRef) -> Value {
    json!({"schema_revision":"s-financial-evidence/1","content_format":"s-content/1",
        "kind":"FinancialBasisEvidence","session_id":SID,"session_generation":"1","writer_epoch":"1",
        "pending_revision":false,"commit_ns":"77","basis_ref":root.to_transport_json()})
}
fn anchor(root: &ContentRef) -> Value {
    json!({"session_id":SID,"session_generation":"1","writer_epoch":"1","structure_cut":"cut-1",
        "index_frontier":"batch-1","commit_ns":"77","basis_ref":root.to_transport_json()})
}
fn hexed(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn node_json(digest: &NodeDigest, tag: u8, body: &[u8]) -> Value {
    json!({"root":sc::root_string(digest),"tag":format!("{tag:02x}"),"body_hex":hexed(body)})
}

/// 诚实伪 S：闭包 = 根经 `sc::verify` 实际读到的不同摘要，按摘要升序，贪心装块。
struct FakeS {
    store: MemoryNodes,
    root: ContentRef,
    anchor: Value,
    digests: Vec<NodeDigest>,
    calls: usize,
}
struct Rec<'a> {
    inner: &'a mut MemoryNodes,
    seen: BTreeSet<NodeDigest>,
}
impl NodeSource for Rec<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let read = self.inner.read(digest)?;
        if matches!(read, NodeRead::Found { .. }) {
            self.seen.insert(*digest);
        }
        Ok(read)
    }
}
impl FakeS {
    fn new() -> Self {
        let (mut store, root) = basis();
        let mut rec = Rec {
            inner: &mut store,
            seen: BTreeSet::new(),
        };
        let verified = sc::verify(&mut rec, &root).unwrap();
        let digests: Vec<NodeDigest> = rec.seen.into_iter().collect();
        assert_eq!(verified.nodes, digests.len());
        Self {
            anchor: anchor(&root),
            store,
            root,
            digests,
            calls: 0,
        }
    }
    fn node(&self, digest: &NodeDigest) -> Value {
        let (tag, body) = self.store.get(digest).unwrap();
        node_json(digest, tag, body)
    }
    /// 一块回复（不含篡改）：`after` 之后最长前缀，规范 JSON 加 LF 不超过预算。
    fn reply(&mut self, request: &Value) -> Result<Value, String> {
        self.calls += 1;
        assert_eq!(request["schema_revision"], "s-financial-closure/1");
        assert_eq!(request.as_object().unwrap().len(), 5);
        let budget: u64 = request["max_reply_bytes"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let start = match request["after"].as_str().unwrap() {
            "" => 0,
            after => {
                let cursor = sc::parse_root(after).unwrap();
                self.digests.binary_search(&cursor).unwrap() + 1
            }
        };
        let mut reply = json!({"anchor":request["anchor"],"path":request["path"],
            "after":request["after"],"target_ref":self.root.to_transport_json(),
            "total":self.digests.len().to_string(),"nodes":[],
            "next":sc::root_string(&self.digests[start]),"done":false,"ok":true});
        let mut used = canonical_json(&reply).len() as u64 + 1;
        let mut nodes = Vec::new();
        let mut end = start;
        while end < self.digests.len() {
            let node = self.node(&self.digests[end]);
            let size = canonical_json(&node).len() as u64 + u64::from(!nodes.is_empty());
            if used + size > budget {
                break;
            }
            used += size;
            nodes.push(node);
            end += 1;
        }
        assert!(!nodes.is_empty(), "测试预算须装得下一个节点");
        reply["nodes"] = Value::Array(nodes);
        reply["next"] = json!(sc::root_string(&self.digests[end - 1]));
        reply["done"] = json!(end == self.digests.len());
        Ok(reply)
    }
    /// 一个合法但不在闭包里的节点。
    fn stranger(&self) -> (NodeDigest, u8, Vec<u8>) {
        let mut other = MemoryNodes::new();
        let r = sc::write_value(&mut other, &V::string("not-in-closure-xyz"))
            .unwrap()
            .content_ref();
        let d = r.digest().unwrap();
        let (tag, body) = other.get(&d).unwrap();
        (d, tag, body.to_vec())
    }
}

/// 物理分配用每节点 body+128 的测试模型；暂存与写入批次可观察，回滚按摘要删除。
#[derive(Default, Clone, PartialEq, Debug)]
struct MemStage {
    nodes: BTreeMap<NodeDigest, (u8, Vec<u8>)>,
    batches: usize,
}
impl NodeSource for MemStage {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        Ok(match self.nodes.get(digest) {
            Some((tag, body)) => NodeRead::Found {
                tag: *tag,
                body: body.clone(),
            },
            None => NodeRead::Missing,
        })
    }
}
impl FinancialStage for MemStage {
    fn put_budgeted(
        &mut self,
        _job: &mut Job,
        _digest: NodeDigest,
        _tag: u8,
        _body: &[u8],
    ) -> Result<(), CodecError> {
        panic!("闭包模式不得走逐节点写入");
    }
    fn put_batch_budgeted(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
    ) -> Result<(), CodecError> {
        job.pre_io_check()?;
        let missing: Vec<_> = nodes
            .iter()
            .filter(|(d, _, _)| !self.nodes.contains_key(d))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let bytes: u64 = missing.iter().map(|(_, _, b)| b.len() as u64 + 128).sum();
        let grant = job.reserve_staged(bytes)?;
        self.batches += 1;
        for (d, t, b) in &missing {
            assert_eq!(sc::frame(*t, b), *d);
            self.nodes.insert(*d, (*t, b.clone()));
        }
        inserted.extend(missing.iter().map(|(d, _, _)| *d));
        job.reconcile(&grant, bytes)
    }
    fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError> {
        Ok(digests
            .iter()
            .filter(|d| self.nodes.remove(*d).is_some())
            .count() as u64)
    }
}

fn plan(reply_bytes: u64) -> ClosurePlan {
    ClosurePlan {
        reply_bytes,
        max_chunks: 10_000,
    }
}
/// 跑一次 `import_closure`；`tamper(第几块, 回复)` 可篡改诚实回复或改为返回错误。
fn run(
    s: &mut FakeS,
    stage: &mut dyn FinancialStage,
    job: &mut Job,
    plan: ClosurePlan,
    pins: &SourcePins,
    tamper: &mut dyn FnMut(usize, &mut FakeS, Value) -> Result<Value, String>,
) -> Result<VerifiedBasis, String> {
    let root = s.root.clone();
    let anchor = s.anchor.clone();
    let mut fetch = |request: &Value, _job: &mut Job| {
        let reply = s.reply(request)?;
        let n = s.calls;
        tamper(n, &mut *s, reply)
    };
    import_closure(
        &mut fetch,
        pins,
        &evidence(&root),
        &root,
        &anchor,
        plan,
        job,
        deadline(),
        stage,
    )
}
fn honest(_: usize, _: &mut FakeS, reply: Value) -> Result<Value, String> {
    Ok(reply)
}
fn closure_set(s: &FakeS) -> BTreeSet<NodeDigest> {
    s.digests.iter().copied().collect()
}
fn stage_set(stage: &MemStage) -> BTreeSet<NodeDigest> {
    stage.nodes.keys().copied().collect()
}
/// 预置：闭包里每隔一个节点已在本地（来自先前操作或 peer 导入）。
fn half_stage(s: &FakeS) -> MemStage {
    let mut stage = MemStage::default();
    for d in s.digests.iter().step_by(2) {
        let (t, b) = s.store.get(d).unwrap();
        stage.nodes.insert(*d, (t, b.to_vec()));
    }
    stage
}
fn assert_rejected_and_rolled_back(
    result: Result<VerifiedBasis, String>,
    needle: &str,
    stage: &MemStage,
    before: &MemStage,
) {
    let error = result.expect_err("必须拒绝");
    assert!(error.contains(needle), "错误 {error:?} 不含 {needle:?}");
    assert!(error.contains("闭包尝试已回滚"), "{error}");
    assert_eq!(
        stage_set(stage),
        stage_set(before),
        "本尝试写下的暂存须全部回滚"
    );
}

#[test]
fn honest_stream_imports_exactly_the_root_closure_in_chunk_transactions() {
    let mut s = FakeS::new();
    let pins = pins();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let verified = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest).unwrap();
    assert_eq!(verified.evidence, evidence(&s.root));
    assert_eq!(stage_set(&stage), closure_set(&s), "暂存恰为根闭包");
    assert!(s.calls > 3, "小预算分多块：{}", s.calls);
    assert_eq!(stage.batches, s.calls, "每块一个批量写");
    let snap = job.snapshot();
    assert_eq!(snap.nodes as usize, s.digests.len(), "节点账按不同根计一次");
    assert!(snap.records > 0, "完整核验照旧计 records");
}

#[test]
fn locally_present_nodes_are_skipped_but_still_streamed_and_checked() {
    let mut s = FakeS::new();
    let pins = pins();
    let mut job = job_for(&pins);
    let mut stage = half_stage(&s);
    let before = stage.clone();
    run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest).unwrap();
    assert_eq!(stage_set(&stage), closure_set(&s));
    assert!(stage_set(&before).is_subset(&stage_set(&stage)));
}

#[test]
fn tampered_body_or_tag_is_rejected_and_the_attempt_rolled_back() {
    for field in ["body_hex", "tag"] {
        let mut s = FakeS::new();
        let pins = pins();
        let mut job = job_for(&pins);
        let mut stage = half_stage(&s);
        let before = stage.clone();
        let result = run(
            &mut s,
            &mut stage,
            &mut job,
            plan(9000),
            &pins,
            &mut |n, _, mut r| {
                if n == 3 {
                    let at = r["nodes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .position(|v| v["body_hex"].as_str().unwrap().len() >= 2)
                        .unwrap();
                    let node = &mut r["nodes"][at];
                    let old = node[field].as_str().unwrap().to_owned();
                    let flipped = if old.starts_with('0') { "1" } else { "0" };
                    node[field] = json!(format!("{flipped}{}", &old[1..]));
                }
                Ok(r)
            },
        );
        assert_rejected_and_rolled_back(result, "MissingDependency：", &stage, &before);
    }
}

#[test]
fn cursor_overlap_and_skip_are_rejected() {
    // 重叠：第 2 块重发第 1 块末节点。
    let mut s = FakeS::new();
    let pins = pins();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let before = stage.clone();
    let result = run(
        &mut s,
        &mut stage,
        &mut job,
        plan(9000),
        &pins,
        &mut |n, s, mut r| {
            if n == 2 {
                let cursor = sc::parse_root(r["after"].as_str().unwrap()).unwrap();
                let repeated = s.node(&cursor);
                r["nodes"].as_array_mut().unwrap().insert(0, repeated);
            }
            Ok(r)
        },
    );
    assert_rejected_and_rolled_back(result, "严格升序", &stage, &before);

    // 跳过：第 2 块丢掉首节点，其余照旧（total 不变）→ 末块 done 与累计数不符。
    let mut s = FakeS::new();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let result = run(
        &mut s,
        &mut stage,
        &mut job,
        plan(9000),
        &pins,
        &mut |n, _, mut r| {
            if n == 2 {
                r["nodes"].as_array_mut().unwrap().remove(0);
            }
            Ok(r)
        },
    );
    assert_rejected_and_rolled_back(result, "done 与 total 不符", &stage, &before);
}

#[test]
fn substituted_or_extra_nodes_fail_membership() {
    // 以一个闭包外合法节点替换一个闭包节点（total 与升序都保持）：本地缺该闭包节点 → 完整核验缺节点。
    let pins = pins();
    let mut s = FakeS::new();
    let (y, ty, by) = s.stranger();
    assert!(s.digests.binary_search(&y).is_err(), "y 不在闭包里");
    let x = s.digests[s.digests.len() / 2];
    assert_ne!(x, s.root.digest().unwrap());
    let mut forged = s.digests.clone();
    forged.retain(|d| *d != x);
    forged.push(y);
    forged.sort();
    s.store.put(y, ty, &by).unwrap();
    let honest_digests = std::mem::replace(&mut s.digests, forged);
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let before = stage.clone();
    let result = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest);
    assert_rejected_and_rolled_back(result, sc::code::NODE_MISSING, &stage, &before);

    // 同样替换，但被丢的闭包节点本地已有：核验能过，成员核验发现流里缺它、多了 y。
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let (tx_, bx) = s.store.get(&x).map(|(t, b)| (t, b.to_vec())).unwrap();
    stage.nodes.insert(x, (tx_, bx));
    let before = stage.clone();
    s.calls = 0;
    let result = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest);
    assert_rejected_and_rolled_back(result, "闭包流缺少根可达节点", &stage, &before);

    // 多发一个闭包外节点（total 随之 +1）：核验能过，成员核验发现多余节点；回滚删掉它。
    s.digests = honest_digests;
    s.digests.push(y);
    s.digests.sort();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let before = stage.clone();
    let result = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest);
    assert_rejected_and_rolled_back(result, "根不可达节点", &stage, &before);
    assert!(!stage.nodes.contains_key(&y));
}

#[test]
fn reply_binding_drift_is_rejected() {
    let pins = pins();
    type Tamper = fn(usize, &mut FakeS, Value) -> Result<Value, String>;
    let cases: [(Tamper, &str); 7] = [
        (
            |n, _, mut r| {
                if n == 2 {
                    r["anchor"]["commit_ns"] = json!("78");
                }
                Ok(r)
            },
            "未逐字绑定请求",
        ),
        (
            |n, _, mut r| {
                if n == 2 {
                    r["after"] = json!("");
                }
                Ok(r)
            },
            "未逐字绑定请求",
        ),
        (
            |n, s, mut r| {
                if n == 2 {
                    let other = s.stranger().0;
                    r["target_ref"]["root"] = json!(sc::root_string(&other));
                }
                Ok(r)
            },
            "闭包目标根不是 basis_ref",
        ),
        (
            |n, _, mut r| {
                if n == 2 {
                    let t: u64 = r["total"].as_str().unwrap().parse().unwrap();
                    r["total"] = json!((t + 1).to_string());
                }
                Ok(r)
            },
            "total 改变",
        ),
        (
            |n, _, mut r| {
                if n == 2 {
                    r["next"] = r["after"].clone();
                }
                Ok(r)
            },
            "next 不是本块末节点",
        ),
        (
            |n, _, mut r| {
                if n == 2 {
                    r["nodes"] = json!([]);
                }
                Ok(r)
            },
            "闭包块为空",
        ),
        (
            |n, _, mut r| {
                if n == 2 {
                    r["extra"] = json!(1);
                }
                Ok(r)
            },
            "字段集不为 exact",
        ),
    ];
    for (tamper, needle) in cases {
        let mut s = FakeS::new();
        let mut job = job_for(&pins);
        let mut stage = half_stage(&s);
        let before = stage.clone();
        let mut t = tamper;
        let result = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut t);
        assert_rejected_and_rolled_back(result, needle, &stage, &before);
    }
}

#[test]
fn session_failure_mid_stream_keeps_the_retry_prefix_and_rolls_back() {
    let pins = pins();
    let mut s = FakeS::new();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let before = stage.clone();
    let result = run(
        &mut s,
        &mut stage,
        &mut job,
        plan(9000),
        &pins,
        &mut |n, _, r| {
            if n == 3 {
                // 会话失效或 anchor 漂移时，S 客户端子进程非零退出；类型化原因不可传。
                Err("MissingDependency：财务 reader 非成功退出".into())
            } else {
                Ok(r)
            }
        },
    );
    let error = result.as_ref().unwrap_err().clone();
    assert!(
        error.starts_with("MissingDependency：财务 reader 非成功退出"),
        "read_current 的重读判据只看前缀：{error}"
    );
    assert_rejected_and_rolled_back(result, "非成功退出", &stage, &before);
    assert_eq!(s.calls, 3, "失败后不再请求、不回落逐片");
}

#[test]
fn caps_reject_without_leaving_a_half_import() {
    // 块数帽：S 不装满（每块 1 个节点），第 4 块前停止。
    let pins = pins();
    let mut s = FakeS::new();
    let mut job = job_for(&pins);
    let mut stage = MemStage::default();
    let before = stage.clone();
    let result = run(
        &mut s,
        &mut stage,
        &mut job,
        ClosurePlan {
            reply_bytes: 9000,
            max_chunks: 3,
        },
        &pins,
        &mut |_, _, mut r| {
            r["nodes"].as_array_mut().unwrap().truncate(1);
            let last = r["nodes"][0]["root"].clone();
            r["next"] = last;
            r["done"] = json!(false);
            Ok(r)
        },
    );
    assert_rejected_and_rolled_back(result, "块数帽", &stage, &before);
    assert_eq!(s.calls, 3);

    let total = FakeS::new().digests.len();
    for (resources, needle) in [
        // total 超过 Job 节点帽：首块即停。
        (
            resources_json(
                "100000000",
                &(total - 1).to_string(),
                "100000000",
                "100000000",
            ),
            "超过 Job max_total_nodes",
        ),
        // 节点字节帽：accept_node 停止。
        (
            resources_json("100000000", "1000000", "20000", "100000000"),
            "job_limit",
        ),
        // 暂存字节帽：批量写预留停止。
        (
            resources_json("100000000", "1000000", "100000000", "30000"),
            "job_limit",
        ),
    ] {
        let pins = pins_with(resources);
        let mut s = FakeS::new();
        let mut job = job_for(&pins);
        let mut stage = MemStage::default();
        let result = run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest);
        assert_rejected_and_rolled_back(result, needle, &stage, &before);
    }
}

#[test]
fn frozen_plan_is_derived_from_reader_resources_only() {
    let resources = FinancialReaderResources::from_json(&resources_json(
        "64307200", "32768", "8388608", "32153600",
    ))
    .unwrap();
    let mut r = resources;
    r.max_reply_bytes = 827_022;
    let plan = ClosurePlan::frozen(&r);
    assert_eq!(plan.reply_bytes, 827_022);
    assert_eq!(plan.max_chunks, 2 * (64_307_200 / 827_022) + 1);
    r.job_limits.max_total_wire_bytes = 134_217_728;
    assert_eq!(
        ClosurePlan::frozen(&r).max_chunks,
        2 * (134_217_728 / 827_022) + 1
    );
}

/// 真实 `content_store::Stage`（固定 HFS+ 载体、8 倍页规则、物理对账）上的同一组语义：每块一个事务、
/// 拒绝即回滚本尝试的暂存行、成功后发布搬运恰为根闭包、发布后的多余节点尝试不留暂存。
#[test]
#[ignore = "需要已挂载的固定 HFS+ 载体根（CLOSURE_CARRIER_DB 指向其根下尚不存在的库文件）"]
fn real_stage_on_hfs_carrier_commits_chunks_and_rolls_back_attempts() {
    use super::super::content_store;
    use super::closure_replica_tests::{create_db, new_job, publish, snapshot};
    let db = std::path::PathBuf::from(std::env::var("CLOSURE_CARRIER_DB").unwrap());
    let mut c = create_db(&db);
    let pins = pins();
    let limits = pins.resources.job_limits;
    let rows = |c: &rusqlite::Connection, table: &str| -> i64 {
        c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    };
    let mut report = json!({});
    for (label, tamper_at) in [("tampered-chunk", 3usize), ("session-failure", 4usize)] {
        let mut job = new_job(&c, &db, limits);
        let result = {
            let mut stage =
                content_store::Stage::new(&mut c, &db, limits, format!("stg-{label}")).unwrap();
            let mut s = FakeS::new();
            run(
                &mut s,
                &mut stage,
                &mut job,
                plan(9000),
                &pins,
                &mut |n, _, mut r| {
                    if n == tamper_at {
                        if label == "session-failure" {
                            return Err("MissingDependency：财务 reader 非成功退出".into());
                        }
                        r["nodes"][0]["tag"] = json!("ff");
                    }
                    Ok(r)
                },
            )
        };
        let error = result.expect_err("必须拒绝");
        assert!(error.contains("闭包尝试已回滚"), "{error}");
        assert_eq!(rows(&c, "content_staging"), 0, "{label}: 回滚后无暂存行");
        assert_eq!(snapshot(&job)["open_reservations"], 0);
        report[label] = json!({"error":error,"job":snapshot(&job)});
    }
    let mut s = FakeS::new();
    let mut job = new_job(&c, &db, limits);
    {
        let mut stage =
            content_store::Stage::new(&mut c, &db, limits, "stg-honest".into()).unwrap();
        run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest).unwrap();
    }
    assert_eq!(rows(&c, "content_staging") as usize, s.digests.len());
    report["honest_chunks"] = json!(s.calls);
    report["honest_job"] = snapshot(&job);
    let root = s.root.clone();
    report["publish"] = json!(publish(&mut c, &db, std::slice::from_ref(&root), &mut job).unwrap());
    assert_eq!(rows(&c, "content_staging"), 0);
    assert_eq!(
        rows(&c, "content_nodes") as usize,
        s.digests.len(),
        "发布搬运恰为根闭包"
    );
    // 发布后再来一次带多余节点的尝试：闭包节点都已公开（跳过），多余节点被写入后又被回滚。
    let (y, ty, by) = s.stranger();
    s.store.put(y, ty, &by).unwrap();
    s.digests.push(y);
    s.digests.sort();
    s.calls = 0;
    let mut job = new_job(&c, &db, limits);
    let result = {
        let mut stage = content_store::Stage::new(&mut c, &db, limits, "stg-extra".into()).unwrap();
        run(&mut s, &mut stage, &mut job, plan(9000), &pins, &mut honest)
    };
    let error = result.expect_err("多余节点必须拒绝");
    assert!(
        error.contains("根不可达节点") && error.contains("已回滚 1 行"),
        "{error}"
    );
    assert_eq!(rows(&c, "content_staging"), 0);
    report["extra_node"] = json!({"error":error});
    if let Ok(out) = std::env::var("CLOSURE_REPLICA_OUT") {
        std::fs::write(out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

/// r2（独审 N1）故障注入：块事务提交成功后令该块的 TRUNCATE checkpoint 失败。注入用 SQLite 的真实
/// 行为，不改产品代码：第 k 块请求返回时（上一块的 checkpoint 已截空 WAL）另开一个连接并开读事务，
/// 它停在读标记 0（只读主库），第 k 块提交后的 checkpoint 无法回填而报 busy，`put_batch_budgeted`
/// 在提交之后返回 `Err`。同一 stage token 下先由 `import_peer_refs` 从对端库导入一棵子树，其节点散在
/// 闭包的若干块里。断言：暂存表逐行回到 peer 导入后的状态（第 1..=k 块本尝试插入的行全部删去，
/// peer 行一行不少、字节与 stage 不变）；回滚行数恰为第 1..=k 块的新行数（证明第 k 块已提交）；
/// 撤掉读者后同一 stage 再导入一次诚实闭包能成功。
#[test]
#[ignore = "需要已挂载的固定 HFS+ 载体根（CLOSURE_CARRIER_DB 指向其根下尚不存在的库文件）"]
fn post_commit_checkpoint_failure_rolls_back_the_committed_chunk_and_keeps_peer_rows() {
    use super::super::{content_store, store};
    use super::closure_replica_tests::{create_db, new_job, snapshot};
    type Rows = BTreeMap<Vec<u8>, (i64, Vec<u8>, String)>;
    let staging = |c: &rusqlite::Connection| -> Rows {
        let mut q = c
            .prepare("SELECT digest,tag,body,stage FROM content_staging")
            .unwrap();
        let rows = q
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?))))
            .unwrap();
        let out: Rows = rows.map(Result::unwrap).collect();
        out
    };
    let digests = |rows: &Rows| -> BTreeSet<NodeDigest> {
        rows.keys()
            .map(|d| NodeDigest::try_from(&d[..]).unwrap())
            .collect()
    };
    let db = std::path::PathBuf::from(std::env::var("CLOSURE_CARRIER_DB").unwrap());
    let pins = pins();
    let limits = pins.resources.job_limits;
    let token = "stg-n1".to_string();

    // 对端：basis 里第一行对象这棵子树，节点全部已公开。
    let mut sub_store = MemoryNodes::new();
    let sub = sc::write_value(&mut sub_store, &row(0, 9000))
        .unwrap()
        .content_ref();
    let peer_nodes = {
        let mut rec = Rec {
            inner: &mut sub_store,
            seen: BTreeSet::new(),
        };
        sc::verify(&mut rec, &sub).unwrap();
        rec.seen
    };
    let closure = closure_set(&FakeS::new());
    // 诚实分块（与伪 S 同一函数、同一预算）：挑第 k 块注入。
    let mut chunks: Vec<Vec<NodeDigest>> = Vec::new();
    {
        let mut probe = FakeS::new();
        let mut job = job_for(&pins);
        let mut mem = MemStage::default();
        run(
            &mut probe,
            &mut mem,
            &mut job,
            plan(9000),
            &pins,
            &mut |_, _, r| {
                chunks.push(
                    r["nodes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| sc::parse_root(v["root"].as_str().unwrap()).unwrap())
                        .collect(),
                );
                Ok(r)
            },
        )
        .unwrap();
    }
    let new_in = |i: usize| {
        chunks[i]
            .iter()
            .filter(|d| !peer_nodes.contains(*d))
            .count()
    };
    let peer_in = |i: usize| chunks[i].iter().filter(|d| peer_nodes.contains(*d)).count();
    // 第 k 块（从 1 起）：k ≥ 2，本块有新行且有 peer 行；没有这样的块就退到「第 1..=k 块里有 peer 行」。
    let k = (2..=chunks.len())
        .find(|&k| new_in(k - 1) > 0 && peer_in(k - 1) > 0)
        .or_else(|| {
            (2..=chunks.len()).find(|&k| new_in(k - 1) > 0 && (0..k).any(|i| peer_in(i) > 0))
        })
        .expect("须有一块既有新行、之前（含本块）又有 peer 行");
    let new_rows_through_k: usize = (0..k).map(new_in).sum();
    let overlap = peer_nodes.intersection(&closure).count();
    assert!(overlap > 0, "peer 子树须与闭包相交");

    let peer_dir = std::env::temp_dir().join(format!("closure-n1-peer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&peer_dir);
    std::fs::create_dir_all(&peer_dir).unwrap();
    let peer_db = peer_dir.join("peer.sqlite");
    {
        let peer = create_db(&peer_db);
        for d in &peer_nodes {
            let (t, b) = sub_store.get(d).unwrap();
            peer.execute(
                "INSERT INTO content_nodes VALUES(?1,?2,?3)",
                rusqlite::params![&d[..], i64::from(t), b],
            )
            .unwrap();
        }
    }
    let (peer, _) = store::open(&peer_db, true).unwrap();

    let mut c = create_db(&db);
    let mut job = new_job(&c, &db, limits);
    {
        let mut stage = content_store::Stage::new(&mut c, &db, limits, token.clone()).unwrap();
        content_store::import_peer_refs(&peer, std::slice::from_ref(&sub), &mut stage, &mut job)
            .unwrap();
    }
    let peer_rows = staging(&c);
    assert_eq!(digests(&peer_rows), peer_nodes, "peer 导入恰为子树");
    assert!(peer_rows.values().all(|(_, _, s)| *s == token));

    // 第 k 块请求返回时开读者：它在空 WAL 上起读事务，停在读标记 0。
    let mut reader: Option<rusqlite::Connection> = None;
    let mut s = FakeS::new();
    let result = {
        let mut stage = content_store::Stage::new(&mut c, &db, limits, token.clone()).unwrap();
        run(
            &mut s,
            &mut stage,
            &mut job,
            plan(9000),
            &pins,
            &mut |n, _, r| {
                if n == k {
                    let rd = rusqlite::Connection::open_with_flags(
                        &db,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                    )
                    .unwrap();
                    rd.execute_batch("BEGIN").unwrap();
                    let _: i64 = rd
                        .query_row("SELECT count(*) FROM content_staging", [], |q| q.get(0))
                        .unwrap();
                    reader = Some(rd);
                }
                Ok(r)
            },
        )
    };
    assert!(reader.is_some(), "注入点须已到达");
    drop(reader.take());
    let error = result.expect_err("第 k 块提交后的 checkpoint 失败必须拒绝");
    assert!(
        error.contains("WAL checkpoint blocked by another reader"),
        "失败须来自 checkpoint：{error}"
    );
    assert_eq!(s.calls, k, "失败后不再请求");
    let after = staging(&c);
    let residue: Vec<_> = digests(&after).difference(&peer_nodes).copied().collect();
    assert!(
        residue.is_empty(),
        "暂存残留 {} 行本尝试写下的行（第 {k} 块提交后失败）：{error}",
        residue.len()
    );
    assert_eq!(
        after, peer_rows,
        "先前 peer 导入的行逐行不变（摘要、tag、body、stage）"
    );
    let removed: usize = error
        .split("已回滚 ")
        .nth(1)
        .and_then(|t| t.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("错误尾部须写回滚行数：{error}"));
    assert_eq!(
        removed, new_rows_through_k,
        "回滚恰为第 1..={k} 块的新行（含已提交的第 {k} 块）"
    );
    let nodes_published: i64 = c
        .query_row("SELECT count(*) FROM content_nodes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(nodes_published, 0);
    assert_eq!(snapshot(&job)["open_reservations"], 0);
    let failed_job = snapshot(&job);

    // 读者撤掉后，同一 stage 再导入一次诚实闭包：下一块写前的计费 checkpoint 收回残留 WAL。
    let mut honest_s = FakeS::new();
    {
        let mut stage = content_store::Stage::new(&mut c, &db, limits, token.clone()).unwrap();
        run(
            &mut honest_s,
            &mut stage,
            &mut job,
            plan(9000),
            &pins,
            &mut honest,
        )
        .unwrap();
    }
    let healed = staging(&c);
    let want: BTreeSet<NodeDigest> = closure.union(&peer_nodes).copied().collect();
    assert_eq!(digests(&healed), want, "诚实重试后暂存＝闭包 ∪ peer 子树");
    for (d, row) in &peer_rows {
        assert_eq!(healed.get(d), Some(row), "peer 行在重试后仍逐字节不变");
    }
    if let Ok(out) = std::env::var("CLOSURE_REPLICA_OUT") {
        let report = json!({"chunks":chunks.len(),"k":k,"chunk_k_nodes":chunks[k - 1].len(),
            "chunk_k_new":new_in(k - 1),"chunk_k_peer":peer_in(k - 1),
            "new_rows_through_k":new_rows_through_k,"peer_nodes":peer_nodes.len(),
            "peer_nodes_in_closure":overlap,"closure_nodes":closure.len(),"error":error,
            "removed":removed,"staging_rows_after_failure":after.len(),
            "job_after_failure":failed_job,"retry_staging_rows":healed.len(),
            "job_after_retry":snapshot(&job)});
        std::fs::write(out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    drop(peer);
    std::fs::remove_dir_all(&peer_dir).unwrap();
}
