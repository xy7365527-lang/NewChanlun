//! #1394 C §5.5：从完整核验的已发布切面构造七字段Q投影，正文保持值边。
//! 构造器只接当前事务的AuditedState与只读派生图；不注册服务，不冒充Q capture。
//! 六族按现有 s_query._order/_key 排序；S发布数组自身的顺序不是Q顺序。

use super::*;
use newchan_rust::s_content::{
    self as sc,
    job::Job,
    path::{lookup_path, PathStep},
    ContentRef, Kind, NodeDigest, NodeRead, NodeSink, NodeSource, Value as V,
};
use newchan_rust::session_protocol::parse_canonical_i64;
use rusqlite::Transaction;
use std::collections::{BTreeMap, BTreeSet};

const FAMILIES: [&str; 6] = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
];

#[derive(Clone, Copy, Debug)]
pub(super) enum HistoryMode {
    AsKnown,
    RecomputedWithRevision,
}
impl HistoryMode {
    fn name(self) -> &'static str {
        match self {
            Self::AsKnown => "AsKnown",
            Self::RecomputedWithRevision => "RecomputedWithRevision",
        }
    }
}

pub(super) struct Projection {
    pub root: ContentRef,
    pub digest: String,
    /// 仅这三个位置解释ContentRef边：fixed_cut.catalog_evidence、catalog、rows[*].record。
    pub wire: Value,
}
impl Projection {
    pub(super) fn anchor(&self) -> Value {
        json!({"kind":"projection","fixed_cut":self.wire["fixed_cut"],"cut_projection_digest":self.digest,"projection_ref":self.root.to_transport_json()})
    }
}

/// 派生节点只写调用者的HashReader内存层，不能经此接口写SQLite。
/// audit必须在同一事务先完成；独立Q capture/verify资源门仍由上层负责。
pub(super) fn build(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
    graph: &mut history::HashReader<'_>,
    generation: i64,
    mode: HistoryMode,
    max_metadata_bytes: u64,
    job: &mut Job,
) -> Result<Projection, String> {
    if max_metadata_bytes == 0 || max_metadata_bytes > sc::MAX_FRAME_LEN {
        return Err(err("元数据物化上界必须显式给出且在规范数域内"));
    }
    if generation < 0 {
        return Err(err("generation须为非负整数"));
    }
    let cut = if generation == 0 {
        None
    } else {
        Some(
            audited
                .cuts
                .iter()
                .find(|c| c.metadata.generation == generation)
                .ok_or_else(|| err("Unavailable：请求cut尚未发布"))?,
        )
    };
    let get = |key| meta_get_opt(tx, key)?.ok_or_else(|| err(format!("缺冻结字段{key}")));
    if get("profile_id")? != tb02a::PROFILE {
        return Err(err("非OHLC投影仍待接线，不套用s-record-order/2"));
    }
    let (_, session_generation, _) = v2::protocol_identity(tx)?;
    let mut s = Metered {
        graph,
        job,
        metadata_remaining: max_metadata_bytes,
    };
    s.job.pre_io_check().map_err(err)?;
    let empty = write(&mut s, &json!({}))?;
    let scope = strict_json(get("scope")?.as_bytes())?;
    let (evidence, run, frontier, range, index) = if let Some(cut) = cut {
        (
            field(&mut s, &cut.snapshot, "catalog_evidence")?,
            text(&mut s, &cut.snapshot, "catalog_run_status")?,
            cut.metadata.input_frontier.to_string(),
            scalar_json(&mut s, &cut.snapshot, "seq_range")?,
            cut.batch_id.clone(),
        )
    } else {
        (
            empty.clone(),
            "not_run".into(),
            "-1".into(),
            json!({"from":"0","to":"-1"}),
            String::new(),
        )
    };
    let mut fixed = json!({"session_id":get("session_id")?,"session_generation":session_generation,
        "structure_cut":format!("cut-{generation}"),"cut_generation":generation.to_string(),"catalog_revision":get("catalog_revision")?,
        "index_frontier":index,"profile_id":get("profile_id")?,"profile_hash":get("profile_hash")?,"input_frontier":frontier,
        "seq_range":range,"catalog_run_status":run,"catalog_evidence":evidence.to_transport_json(),"scope":scope,
        "rule_revision":get("rule_revision")?,"history_mode":mode.name(),
        "as_of_generation":if matches!(mode,HistoryMode::AsKnown){json!(generation.to_string())}else{Value::Null}});
    let fixed_root = edge_object(&mut s, &fixed, &[("catalog_evidence", &evidence)])?;

    // 六族的顺序与原Q完全一致；只渲染排序/公开键字段，不展开payload、来源或证据。
    let mut rows = Vec::new();
    let mut row_roots = Vec::new();
    let mut counts = serde_json::Map::new();
    let mut object_ids = BTreeSet::new();
    for family in FAMILIES {
        let mut ordered = Vec::new();
        if let Some(cut) = cut {
            let collection = field(&mut s, &cut.snapshot, family)?;
            if collection.kind != Kind::Array {
                return Err(err("快照记录族不是数组"));
            }
            for i in 0..collection.extent.ok_or_else(|| err("数组缺extent"))? {
                s.job.charge_records(1).map_err(err)?;
                let record =
                    lookup_path(&mut s, &collection, &[PathStep::Index(i)]).map_err(err)?;
                let (order, key) = record_order_key(&mut s, family, &record)?;
                ordered.push((order, key, record));
            }
        }
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
        let mut keys = BTreeSet::new();
        counts.insert(family.into(), json!(ordered.len().to_string()));
        for (_, key, record) in ordered {
            if !keys.insert(key.clone()) {
                return Err(err(format!("{family}含重复公开键")));
            }
            if matches!(family, "objects" | "withdrawn_objects") && !object_ids.insert(key.clone())
            {
                return Err(err("active/withdrawn对象身份冲突"));
            }
            let row = json!({"family":family,"key":key,"record":record.to_transport_json()});
            row_roots.push(
                edge_object(&mut s, &row, &[("record", &record)])?
                    .digest()
                    .map_err(err)?,
            );
            rows.push(row);
        }
    }
    counts.insert("total".into(), json!(rows.len().to_string()));
    let rows_root = sc::write_value(
        &mut s,
        &V::Array(row_roots.into_iter().map(V::Edge).collect()),
    )
    .map_err(err)?
    .content_ref();
    let catalog = build_catalog(tx, &mut s, &fixed, &evidence, &empty, generation)?;
    let wire = json!({"fixed_cut":fixed.take(),"catalog":catalog.to_transport_json(),"rows":rows,
        "counts":counts,"scope":scope,"omissions":[],"order_version":"s-record-order/2"});
    let root = edge_object(
        &mut s,
        &wire,
        &[
            ("fixed_cut", &fixed_root),
            ("catalog", &catalog),
            ("rows", &rows_root),
        ],
    )?;
    let identity = sc::frame(0x22, &root.digest().map_err(err)?);
    let digest = format!(
        "qcut3-{}",
        identity
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    s.job.pre_io_check().map_err(err)?;
    Ok(Projection { root, digest, wire })
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum OrderPart {
    Number(i64),
    Text(String),
}
fn record_order_key(
    s: &mut Metered<'_, '_>,
    family: &str,
    r: &ContentRef,
) -> Result<(Vec<OrderPart>, Vec<String>), String> {
    use OrderPart::{Number as N, Text as T};
    if r.kind != Kind::Object {
        return Err(err("族记录不是对象"));
    }
    Ok(match family {
        "objects" => {
            let id = text(s, r, "object_id")?;
            let revision = number(s, r, "object_revision")?;
            let order = match lookup_path(s, r, &[PathStep::Field("fact_key".into())]) {
                Ok(fact) => vec![
                    N(1),
                    T(text(s, r, "kind")?),
                    T(render(s, &fact)?
                        .as_str()
                        .ok_or_else(|| err("fact_key非文本"))?
                        .into()),
                    T(id.clone()),
                    N(revision),
                ],
                Err(e) if e.code() == sc::path::path_code::PATH_FIELD_MISSING => vec![
                    N(0),
                    N(number(s, r, "window_start")?),
                    N(number(s, r, "window_mid")?),
                    N(number(s, r, "window_end")?),
                    T(id.clone()),
                    N(revision),
                ],
                Err(e) => return Err(err(e)),
            };
            (order, vec![id])
        }
        "withdrawn_objects" => {
            let id = text(s, r, "object_id")?;
            (
                vec![
                    N(number(s, r, "withdrawn_generation")?),
                    T(id.clone()),
                    N(number(s, r, "object_revision")?),
                ],
                vec![id],
            )
        }
        "witnesses" => {
            let id = text(s, r, "witness_id")?;
            (
                vec![
                    T(text(s, r, "object_id")?),
                    N(number(s, r, "slot")?),
                    T(id.clone()),
                ],
                vec![id],
            )
        }
        "relations" => {
            let key = ["subject", "relation_type", "object"]
                .iter()
                .map(|k| text(s, r, k))
                .collect::<Result<Vec<_>, _>>()?;
            (key.iter().cloned().map(T).collect(), key)
        }
        "observations" => {
            let id = text(s, r, "observation_id")?;
            let w = field(s, r, "window_start")?;
            let n = if w.kind == Kind::Null {
                -1
            } else {
                number(s, r, "window_start")?
            };
            (vec![T(text(s, r, "kind")?), N(n), T(id.clone())], vec![id])
        }
        "raw_history" => {
            let id = text(s, r, "identity_key")?;
            let rev = text(s, r, "revision")?;
            (
                vec![
                    N(number(s, r, "seq")?),
                    T(id.clone()),
                    N(parse_canonical_i64(&rev, "revision")?),
                ],
                vec![id, rev],
            )
        }
        _ => return Err(err("未知记录族")),
    })
}

fn build_catalog(
    tx: &Transaction<'_>,
    s: &mut Metered<'_, '_>,
    fixed: &Value,
    evidence: &ContentRef,
    empty: &ContentRef,
    generation: i64,
) -> Result<ContentRef, String> {
    let axes = if generation == 0 {
        None
    } else {
        Some(field(s, evidence, "axes")?)
    };
    let mut stmt = tx
        .prepare(
            "SELECT catalog_id,kind,title,domain,branches_json FROM catalog ORDER BY catalog_id",
        )
        .map_err(err)?;
    let mut sql_rows = stmt.query([]).map_err(err)?;
    let mut items = Vec::new();
    let mut counts = BTreeMap::from([
        ("implemented", 0u64),
        ("not_implemented", 0),
        ("run", 0),
        ("not_run", 0),
    ]);
    while let Some(row) = sql_rows.next().map_err(err)? {
        s.job.pre_io_check().map_err(err)?;
        let id: String = row.get(0).map_err(err)?;
        let (implemented, proof, run, ev) =
            if catalog_evidence::AXIS_IDS.contains(&id.as_str()) && axes.is_some() {
                let axis = field(s, axes.as_ref().unwrap(), &id)?;
                (
                    text(s, &axis, "impl_status")?,
                    text(s, &axis, "proof_status")?,
                    text(s, &axis, "run_status")?,
                    field(s, &axis, "evidence")?,
                )
            } else {
                (
                    "not_implemented".into(),
                    "not_proved".into(),
                    "not_run".into(),
                    empty.clone(),
                )
            };
        if let Some(n) = counts.get_mut(implemented.as_str()) {
            *n += 1;
        }
        if let Some(n) = counts.get_mut(run.as_str()) {
            *n += 1;
        }
        let branches: String = row.get(4).map_err(err)?;
        let item = json!({"id":id,"kind":row.get::<_,String>(1).map_err(err)?,"title":row.get::<_,String>(2).map_err(err)?,"domain":row.get::<_,String>(3).map_err(err)?,
            "branches":project_wire_integers(&strict_json(branches.as_bytes())?),"implementation_status":implemented,"proof_status":proof,"run_status":run,"evidence":ev.to_transport_json()});
        items.push(
            edge_object(s, &item, &[("evidence", &ev)])?
                .digest()
                .map_err(err)?,
        );
    }
    let items = sc::write_value(s, &V::Array(items.into_iter().map(V::Edge).collect()))
        .map_err(err)?
        .content_ref();
    let mut header = serde_json::Map::new();
    for key in [
        "session_id",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "input_frontier",
        "seq_range",
        "catalog_run_status",
        "catalog_evidence",
        "scope",
    ] {
        header.insert(key.into(), fixed[key].clone());
    }
    header.insert("generation".into(), fixed["cut_generation"].clone());
    header.insert("items".into(), items.to_transport_json());
    header.insert(
        "counts".into(),
        json!(counts
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect::<BTreeMap<_, _>>()),
    );
    edge_object(
        s,
        &Value::Object(header),
        &[("catalog_evidence", evidence), ("items", &items)],
    )
}

struct Metered<'a, 'b> {
    graph: &'a mut history::HashReader<'b>,
    job: &'a mut Job,
    metadata_remaining: u64,
}
impl NodeSource for Metered<'_, '_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        self.job.pre_io_check()?;
        let value = self.graph.read(d)?;
        if let NodeRead::Found { tag, body } = &value {
            self.job.accept_node(d, *tag, body)?;
        }
        self.job.pre_io_check()?;
        Ok(value)
    }
}
impl NodeSink for Metered<'_, '_> {
    fn put(&mut self, d: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        // 此sink类型固定为内存派生层。写前节点准入失败不写；不承诺磁盘事务或空间计费。
        self.job.accept_node(&d, tag, body)?;
        self.graph.put(d, tag, body)?;
        self.job.pre_io_check()
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("Q3ProjectionInvalid：{e}")
}
fn field(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[PathStep::Field(k.into())]).map_err(err)
}
/// 这里只物化排序/固定头的元数据；字节预算累计，不能借共享DAG放大输出。
/// 这是内部显式构造上界，尚不替代Q capture/cache的独立资源检查或峰值内存测量。
fn render(s: &mut Metered<'_, '_>, r: &ContentRef) -> Result<Value, String> {
    let limit = s.metadata_remaining;
    if matches!(r.kind, Kind::String | Kind::Number) && r.extent.is_some_and(|n| n > limit) {
        return Err(err(
            "Incomplete/ProjectionMetadataLimit：元数据长度超过剩余额度",
        ));
    }
    struct Buffer {
        bytes: Vec<u8>,
        limit: u64,
        exceeded: bool,
    }
    impl std::io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if (self.bytes.len() as u64).saturating_add(bytes.len() as u64) > self.limit {
                self.exceeded = true;
                return Err(std::io::Error::other("元数据输出超限"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Buffer {
        bytes: vec![],
        limit,
        exceeded: false,
    };
    let result = sc::render_canonical(s, r, &mut out);
    s.metadata_remaining -= out.bytes.len() as u64;
    if out.exceeded {
        return Err(err(
            "Incomplete/ProjectionMetadataLimit：元数据展开或转义超过剩余额度",
        ));
    }
    result.map_err(err)?;
    s.job.pre_io_check().map_err(err)?;
    strict_json(&out.bytes)
}
fn scalar_json(s: &mut Metered<'_, '_>, r: &ContentRef, k: &str) -> Result<Value, String> {
    let child = field(s, r, k)?;
    render(s, &child)
}
fn text(s: &mut Metered<'_, '_>, r: &ContentRef, k: &str) -> Result<String, String> {
    scalar_json(s, r, k)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| err(format!("{k}不是规范文本")))
}
fn number(s: &mut Metered<'_, '_>, r: &ContentRef, k: &str) -> Result<i64, String> {
    parse_canonical_i64(&text(s, r, k)?, k)
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(b) => V::Bool(*b),
        Value::String(t) => V::String(t.clone()),
        Value::Number(n) => V::Number(sc::NumberToken::parse(&n.to_string()).map_err(err)?),
        Value::Array(xs) => V::Array(xs.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(xs) => V::Object(
            xs.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
fn write(s: &mut dyn NodeSink, v: &Value) -> Result<ContentRef, String> {
    sc::write_value(s, &cv(v)?)
        .map(|v| v.content_ref())
        .map_err(err)
}
fn edge_object(
    s: &mut dyn NodeSink,
    value: &Value,
    edges: &[(&str, &ContentRef)],
) -> Result<ContentRef, String> {
    let fields = value
        .as_object()
        .ok_or_else(|| err("投影值非对象"))?
        .iter()
        .map(|(k, v)| {
            let value = match edges.iter().find(|(name, _)| *name == k) {
                Some((_, r)) => V::Edge(r.digest().map_err(err)?),
                None => cv(v)?,
            };
            Ok((k.clone(), value))
        })
        .collect::<Result<Vec<_>, String>>()?;
    sc::write_value(s, &V::Object(fields))
        .map(|v| v.content_ref())
        .map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    const METADATA: u64 = 1_048_576;
    fn render_fixture(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Value, String> {
        let mut bytes = Vec::new();
        sc::render_canonical(s, r, &mut bytes).map_err(err)?;
        strict_json(&bytes)
    }
    use prepared::tests::Fixture;
    use sc::job::{Existing, JobLimits};
    use std::io::Write;
    use std::process::{Command, Stdio};
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(), Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn projected(f: &mut Fixture, g: i64, mode: HistoryMode) -> (Projection, Value, Value) {
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let p = build(&tx, &audited, &mut graph, g, mode, METADATA, &mut job()).unwrap();
        let catalog = render_fixture(
            &mut graph,
            &ContentRef::from_transport_json(&p.wire["catalog"]).unwrap(),
        )
        .unwrap();
        let logical = render_fixture(&mut graph, &p.root).unwrap();
        (p, catalog, logical)
    }
    /// 使用仓内原Q的排序/键函数作为独立oracle；不复写其六族判据。
    fn python_rows(snapshot: &Value) -> Value {
        let script="import sys,json;sys.path.insert(0,sys.argv[1]);import s_query as q;s=json.load(sys.stdin);json.dump([dict(family=f,key=q._key(f,r),record=r) for f in q.FAMILIES for r in sorted(s[f],key=lambda r:q._order(f,r))],sys.stdout,ensure_ascii=False,separators=(',',':'))";
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let mut child = Command::new("python3")
            .args(["-c", script])
            .arg(base)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let data = serde_json::to_vec(snapshot).unwrap();
        child.stdin.take().unwrap().write_all(&data).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        serde_json::from_slice(&output.stdout).unwrap()
    }
    #[test]
    fn real_projection_matches_original_q_order_and_hashes_complete_logical_values() {
        let mut f = Fixture::new();
        for n in 1..=3 {
            publish(&mut f, n);
        }
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let snapshot = render_fixture(&mut graph, &audited.cuts[2].snapshot).unwrap();
        let mut budget = job();
        let p = build(
            &tx,
            &audited,
            &mut graph,
            3,
            HistoryMode::AsKnown,
            METADATA,
            &mut budget,
        )
        .unwrap();
        assert_eq!(p.wire.as_object().unwrap().len(), 7);
        assert_eq!(p.wire["fixed_cut"].as_object().unwrap().len(), 16);
        assert_eq!(p.wire["counts"].as_object().unwrap().len(), 7);
        assert_eq!(p.wire["order_version"], "s-record-order/2");
        assert_eq!(p.wire["omissions"], json!([]));
        let logical = render_fixture(&mut graph, &p.root).unwrap();
        let expected = python_rows(&snapshot);
        assert_eq!(logical["rows"], expected);
        assert!(!expected.as_array().unwrap().is_empty());
        assert_eq!(
            budget.snapshot().records,
            expected.as_array().unwrap().len() as u64
        );
        assert_eq!(
            logical["fixed_cut"]["catalog_evidence"],
            snapshot["catalog_evidence"]
        );
        let mut independently_composed = p.wire.clone();
        independently_composed["rows"] = expected;
        independently_composed["catalog"] = logical["catalog"].clone();
        independently_composed["fixed_cut"]["catalog_evidence"] =
            snapshot["catalog_evidence"].clone();
        let expected_root = write(&mut sc::MemoryNodes::new(), &independently_composed).unwrap();
        assert_eq!(expected_root, p.root);
        assert_ne!(
            write(&mut sc::MemoryNodes::new(), &p.wire).unwrap(),
            p.root,
            "引用carrier不能作为普通JSON进入逻辑根"
        );
        let mut bytes = b"NewChanlun/SContent/1\0".to_vec();
        bytes.push(0x22);
        bytes.extend_from_slice(&32u64.to_be_bytes());
        bytes.extend_from_slice(&p.root.digest().unwrap());
        assert_eq!(p.digest, format!("qcut3-{}", sha256_hex(&bytes)));
        independently_composed["rows"][0]["record"]["proof_body_changed"] = json!(true);
        assert_ne!(
            write(&mut sc::MemoryNodes::new(), &independently_composed).unwrap(),
            p.root
        );
    }
    #[test]
    fn zero_and_historical_catalog_use_their_own_axes_after_later_commit() {
        let mut f = Fixture::with_retain(1);
        let (zero, cat, _) = projected(&mut f, 0, HistoryMode::AsKnown);
        assert_eq!(zero.wire["counts"]["total"], "0");
        assert_eq!(zero.wire["fixed_cut"]["input_frontier"], "-1");
        assert_eq!(
            zero.wire["fixed_cut"]["seq_range"],
            json!({"from":"0","to":"-1"})
        );
        assert_eq!(zero.wire["fixed_cut"]["index_frontier"], "");
        assert_eq!(zero.wire["fixed_cut"]["profile_id"], tb02a::PROFILE);
        for item in cat["items"].as_array().unwrap() {
            assert_eq!(item["implementation_status"], "not_implemented");
            assert_eq!(item["proof_status"], "not_proved");
            assert_eq!(item["run_status"], "not_run");
            assert_eq!(item["evidence"], json!({}));
        }
        publish(&mut f, 1);
        let (old, old_catalog, logical) = projected(&mut f, 1, HistoryMode::AsKnown);
        let axes = &logical["fixed_cut"]["catalog_evidence"]["axes"];
        let items = old_catalog["items"].as_array().unwrap();
        assert!(items
            .windows(2)
            .all(|w| w[0]["id"].as_str() < w[1]["id"].as_str()));
        for item in items {
            if let Some(axis) = axes.get(item["id"].as_str().unwrap()) {
                for (public, private) in [
                    ("implementation_status", "impl_status"),
                    ("proof_status", "proof_status"),
                    ("run_status", "run_status"),
                    ("evidence", "evidence"),
                ] {
                    assert_eq!(item[public], axis[private]);
                }
            }
        }
        for key in ["implemented", "not_implemented", "run", "not_run"] {
            let field = if key.contains("implemented") {
                "implementation_status"
            } else {
                "run_status"
            };
            assert_eq!(
                old_catalog["counts"][key],
                items.iter().filter(|i| i[field] == key).count().to_string()
            );
        }
        publish(&mut f, 2);
        let (again, again_catalog, _) = projected(&mut f, 1, HistoryMode::AsKnown);
        assert_eq!(again.root, old.root);
        assert_eq!(again.digest, old.digest);
        assert_eq!(again.wire, old.wire);
        assert_eq!(again_catalog, old_catalog);
        assert_eq!(projected(&mut f, 0, HistoryMode::AsKnown).0.root, zero.root);
        let current = projected(&mut f, 2, HistoryMode::RecomputedWithRevision).0;
        assert_eq!(current.wire["fixed_cut"]["as_of_generation"], Value::Null);
        assert_ne!(current.digest, old.digest);
    }
    #[test]
    fn projection_anchor_is_exact_and_query_only_and_old_cut_survives_delta_retention() {
        let mut f = Fixture::with_retain(1);
        publish(&mut f, 1);
        let anchor = projected(&mut f, 1, HistoryMode::AsKnown).0.anchor();
        publish(&mut f, 2);
        let mut conn = Connection::open(f.db_path()).unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        let before = conn.total_changes();
        let tx = conn.transaction().unwrap();
        let path = [PathStep::Field("rows".into())];
        let p = reader::projection_fragment(
            &tx,
            &anchor,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job(),
        )
        .unwrap();
        sc::fragment::verify(
            &ContentRef::from_transport_json(&anchor["projection_ref"]).unwrap(),
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            &p,
            262144,
            &mut job(),
        )
        .unwrap();
        for changed in [
            "session_id",
            "session_generation",
            "profile_hash",
            "rule_revision",
            "index_frontier",
            "structure_cut",
        ] {
            let mut bad = anchor.clone();
            bad["fixed_cut"][changed] = json!("other");
            assert!(
                reader::projection_fragment(
                    &tx,
                    &bad,
                    &path,
                    sc::fragment::Unit::Items,
                    0,
                    1,
                    262144,
                    METADATA,
                    &mut job()
                )
                .is_err(),
                "{changed}"
            );
        }
        let mut bad = anchor.clone();
        bad["cut_projection_digest"] = json!("qcut3-bad");
        assert!(reader::projection_fragment(
            &tx,
            &bad,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job()
        )
        .is_err());
        let mut bad = anchor.clone();
        bad["projection_ref"]["extent"] = json!("8");
        assert!(reader::projection_fragment(
            &tx,
            &bad,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job()
        )
        .is_err());
        drop(tx);
        assert_eq!(conn.total_changes(), before);
    }
    #[test]
    fn record_limit_stops_projection_without_mutating_storage() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let before = f.conn.total_changes();
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let limits = JobLimits {
            max_total_records: 1,
            ..job().snapshot().limits
        };
        let mut budget = Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(std::time::Instant::now),
        )
        .unwrap();
        assert!(build(
            &tx,
            &audited,
            &mut graph,
            1,
            HistoryMode::AsKnown,
            METADATA,
            &mut budget
        )
        .is_err());
        assert!(budget.snapshot().stopped.is_some());
        drop(tx);
        assert_eq!(f.conn.total_changes(), before);
    }

    #[test]
    fn metadata_limits_bound_expansion_escaping_and_cumulative_reads() {
        let mut nodes = sc::MemoryNodes::new();
        let small = write(&mut nodes, &json!("abcd")).unwrap();
        let escaped = write(&mut nodes, &json!("\0".repeat(30))).unwrap();
        let large = sc::ContentRef::new(Kind::String, &[7; 32], Some(1u64 << 43));
        let mut graph = history::HashReader::new(&mut nodes);
        let mut budget = job();
        let mut source = Metered {
            graph: &mut graph,
            job: &mut budget,
            metadata_remaining: 11,
        };
        assert_eq!(render(&mut source, &small).unwrap(), json!("abcd"));
        assert_eq!(source.metadata_remaining, 5);
        assert!(render(&mut source, &small)
            .unwrap_err()
            .contains("ProjectionMetadataLimit"));
        source.metadata_remaining = 100;
        assert!(render(&mut source, &escaped)
            .unwrap_err()
            .contains("ProjectionMetadataLimit"));
        assert!(source.metadata_remaining < 100, "转义失败仍扣已输出前缀");
        // 声明长度已越界时先停，不追逐假根、更不尝试8TiB展开。
        assert!(render(&mut source, &large)
            .unwrap_err()
            .contains("ProjectionMetadataLimit"));
    }

    #[test]
    fn all_six_sort_tuples_match_python_with_numeric_utf8_and_withdrawal_edges() {
        let fixtures = json!({
            "objects":[
                {"object_id":"o-z","object_revision":"2","kind":"typed","fact_key":"中"},
                {"object_id":"o-a","object_revision":"10","kind":"typed","fact_key":"é"},
                {"object_id":"shape-b","object_revision":"1","window_start":"2","window_mid":"10","window_end":"12"},
                {"object_id":"shape-a","object_revision":"1","window_start":"2","window_mid":"3","window_end":"12"}],
            "withdrawn_objects":[
                {"object_id":"a","object_revision":"1","first_known_generation":"1","withdrawn_generation":"10"},
                {"object_id":"z","object_revision":"1","first_known_generation":"2","withdrawn_generation":"3"}],
            "witnesses":[{"witness_id":"z","object_id":"a","slot":"10"},{"witness_id":"a","object_id":"a","slot":"2"}],
            "relations":[{"subject":"a","relation_type":"t","object":"中"},{"subject":"a","relation_type":"t","object":"é"}],
            "observations":[{"observation_id":"z","kind":"k","window_start":"0"},{"observation_id":"a","kind":"k","window_start":null}],
            "raw_history":[{"seq":"10","identity_key":"z","revision":"1"},{"seq":"2","identity_key":"a","revision":"10"}]
        });
        let oracle = python_rows(&fixtures);
        let mut nodes = sc::MemoryNodes::new();
        let mut actual = Vec::new();
        for family in FAMILIES {
            let mut ordered = Vec::new();
            for record in fixtures[family].as_array().unwrap() {
                let r = write(&mut nodes, record).unwrap();
                let mut graph = history::HashReader::new(&mut nodes);
                let mut budget = job();
                let mut source = Metered {
                    graph: &mut graph,
                    job: &mut budget,
                    metadata_remaining: METADATA,
                };
                let (order, key) = record_order_key(&mut source, family, &r).unwrap();
                ordered.push((order, json!({"family":family,"key":key,"record":record})));
            }
            ordered.sort_by(|a, b| a.0.cmp(&b.0));
            actual.extend(ordered.into_iter().map(|(_, record)| record));
        }
        assert_eq!(json!(actual), oracle);
    }
}
