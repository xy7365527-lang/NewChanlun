//! #1406 S3（AC5）：WE7 上 TB03C 持久写操作的真实子进程 SIGKILL 矩阵。
//! 故障缝是生产既有的 `Config::pause`：Capture/Resolve/Apply 在同一写事务内、`tx.commit()` 紧前与紧后调用。
//! 子进程由 `current_exe` 以具名 ignored 测试重入，经 `service::handle` 执行；生产语义零改动。
//! 范围是本地事务进程故障缝，不是正式 socket/S/浏览器。
use super::tb03c_book_tests::Space;
use super::{book, service, store};
use crate::session_protocol::{canonical_json, strict_json};
use rusqlite::types::ValueRef;
use rusqlite::OptionalExtension;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const RAW: &str = include_str!("../../../s_session/tests/fixtures/tb03c_allocation/RAW.json");
const MANIFEST: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/FROZEN-MANIFEST.json");
const HAND: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/HAND-ORACLE.json");
const SESSION: &str = "tb1406-s3-kill";
const CHILD: &str = "economic_session::tb03c_kill_tests::tb03c_kill_child";
const DEADLINE: Duration = Duration::from_secs(300);

/// #1622：创建 voice_binding 证据权威（TestOnly fixture），写入 bundle 与 verification 文件。
fn write_voice_authority(dir: &Path) -> Value {
    use crate::session_protocol::sha256_hex;
    let doc: Value = serde_json::from_str(RAW).unwrap();
    let event = &doc["events"][0]; // 使用开局事实
    let raw_sha = sha256_hex(RAW.as_bytes());
    let key = book::fact_key(event).unwrap();
    let fact_hash = book::hash(
        &json!({"raw_sha256":raw_sha,"json_pointer":"/events/0","event":event,"units":doc["units"]}),
    );
    let reference = |name: &str| json!({"authority_id":"kill-unit","object_id":name,"revision":"0","content_hash":book::hash(&json!(name))});
    // 为 B:chong-N1 创建 binding（operation_level = 1）
    let binding = json!({
        "evidence_id":"kill-evidence-1",
        "instrument":"TEST-N",
        "chong_id":"chong-N1",
        "operation_level":"1",
        "carrier_ref":reference("kill-carrier"),
        "gamma_ref":reference("kill-gamma"),
        "sigma":"Long",
        "n":"0",
        "parent_voice_id":null,
        "structure_view":{
            "profile_id":"kill-unit",
            "execution_level":"1",
            "confirmation_level":"2",
            "nest_depth":"1"
        },
        "opening_source":{
            "fact_key":key,
            "fact_hash":fact_hash,
            "allocation_id":event["allocation_id"],
            "raw_sha256":raw_sha,
            "json_pointer":"/events/0",
            "quantity":book::integer(&event["observed_existing_book"], "units").unwrap().to_string(),
            "units":doc["units"]
        }
    });
    let mut artifacts = vec![];
    for name in [
        "input",
        "controlled_gammas",
        "carrier_objects",
        "instrument_declaration",
        "opening_raw",
    ] {
        let text = if name == "input" {
            canonical_json(
                &json!({"schema":"testonly-voice-input/1","units":doc["units"],"events":[event]}),
            )
        } else if name == "opening_raw" {
            // CaptureExternal 的 raw_utf8 是 RAW 原文；opening_raw 必须保留同一字节序列。
            RAW.to_owned()
        } else {
            canonical_json(
                &json!({"scope":"unit trust-root fixture, not a real gamma proof","name":name}),
            )
        };
        let path = dir.join(format!("voice-{name}.json"));
        std::fs::write(&path, &text).unwrap();
        artifacts.push(json!({
            "name":name,
            "path":path,
            "sha256":sha256_hex(text.as_bytes()),
            "bytes":text.len().to_string()
        }));
    }
    let bundle = json!({
        "schema":"voice-binding-authority/1",
        "authority_id":"kill-unit",
        "source":{
            "instrument":"TEST-N",
            "input_sha256":artifacts[0]["sha256"],
            "profile_id":"kill-unit",
            "first_confirming_observation_source":"25",
            "clock_mapping":{
                "kind":"TestOnlyAffineOrdinalToNs",
                "source_origin":"0",
                "nanosecond_origin":"0",
                "nanoseconds_per_ordinal":"1"
            }
        },
        "artifacts":artifacts,
        "bindings":[binding]
    });
    let bundle_text = canonical_json(&bundle);
    let bundle_path = dir.join("voice-authority.json");
    std::fs::write(&bundle_path, &bundle_text).unwrap();
    let bundle_sha = sha256_hex(bundle_text.as_bytes());
    let review = json!({
        "schema":"voice-binding-independent-verification/1",
        "verdict":"APPROVE_CONTROLLED_VOICE_BINDING",
        "authority_id":"kill-unit",
        "bundle_sha256":bundle_sha,
        "artifacts":bundle["artifacts"],
        "verified_evidence_ids":["kill-evidence-1"],
        "scope":"unit trusted receipt fixture only; no actual gamma verification claim"
    });
    let review_text = canonical_json(&review);
    let review_path = dir.join("voice-review.json");
    std::fs::write(&review_path, &review_text).unwrap();
    let review_sha = sha256_hex(review_text.as_bytes());
    json!({
        "schema_revision":"voice-binding/1",
        "authorities":[{
            "authority_id":"kill-unit",
            "bundle_path":bundle_path,
            "bundle_sha256":bundle_sha,
            "verification_path":review_path,
            "verification_sha256":review_sha,
            "max_bytes":"1048576"
        }]
    })
}

fn voice_authority() -> &'static Value {
    static AUTHORITY: OnceLock<Value> = OnceLock::new();
    AUTHORITY.get_or_init(|| {
        let dir = root().join("voice-fixture");
        std::fs::create_dir_all(&dir).unwrap();
        write_voice_authority(&dir)
    })
}

/// 与 C4a WE7 同一时钟形状：每条消息每个 epoch 一列，恢复后按第二列提交。
fn manifest(dir: &Path, fault: Option<(&str, &str)>) -> Value {
    let mut plan = serde_json::Map::new();
    for i in 0..256 {
        let t = 1000 + 10 * i;
        let times = |offset: i64| {
            (0..16)
                .map(|r| (r * 1_000_000 + t + offset).to_string())
                .collect::<Vec<_>>()
        };
        plan.insert(
            format!("m-{i}"),
            json!({"received_ns":times(0),"first_known_ns":times(1),"commit_ns":times(2)}),
        );
    }
    let chongs = (0..3).map(|i|json!({"chong_id":format!("chong-N{}",i+1),"symbol":"TEST-N","operation_level":(i+1).to_string(),"fixed_q":(["1","3","3"][i]),"db":dir.join(format!("b{i}.db")),"socket":dir.join(format!("b{i}.sock"))})).collect::<Vec<_>>();
    // #1622：添加 voice_binding 证据配置，供 BindVoice 测试使用
    let voice_binding = voice_authority().clone();
    let mut m = json!({"schema_revision":"economic-manifest/1","session_id":SESSION,"session_generation":"1","book_revision":"tb03c-book/1",
        "e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},"chongs":chongs,"fill_manifests":[serde_json::from_str::<Value>(MANIFEST).unwrap()],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-s3","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","BindVoice","ReadView","QueryFact","QueryAllocation","QueryVoice"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":((1..=16).map(|r| (r * 1_000_000).to_string()).collect::<Vec<_>>()),"messages":plan}},
        "fill_allocation_policy":{"fill_allocation_policy_id":super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"we7-frozen-1"},
        "voice_binding":voice_binding});
    if let Some((message, phase)) = fault {
        m["test_only_fault"] =
            json!({"message_id":message,"phase":phase,"marker_path":dir.join("fault-marker.json")});
    }
    m
}
fn chong(owner: usize) -> Option<String> {
    (owner > 0).then(|| format!("chong-N{owner}"))
}
fn domain_id(owner: usize) -> String {
    chong(owner).map_or_else(|| "E".to_owned(), |c| format!("B:{c}"))
}
fn db_name(owner: usize) -> String {
    if owner == 0 {
        "e.db".into()
    } else {
        format!("b{}.db", owner - 1)
    }
}
fn load(dir: &Path) -> Vec<store::Config> {
    let path = dir.join("manifest.json");
    (0..4)
        .map(|owner| {
            store::Config::load(&path, &dir.join(db_name(owner)), chong(owner).as_deref()).unwrap()
        })
        .collect()
}
/// 目标步的起点：复制「第 target 步之前」的四域库文件副本，代替新进程从空库逐步重放前缀（#1406 S3 降时长）。
/// 副本来自进程内无故障生成（同一时钟计划、同一信封），复制发生在被测进程启动之前。
/// 唯一与路径/故障计划相关的持久值是 init 写入 meta 的 `manifest_hash`：按本目录 manifest 改写后，
/// 断言全库 = 「对本 manifest 新 init 的 meta」+「snapshots[target] 的全部业务表」，
/// 即与旧做法（新 init 后由新进程重放前缀）在被测进程启动时的持久状态逐表逐行相同。
fn setup_from(plan: &Plan, dir: &Path, fault: (&str, &str), target: usize) -> Vec<store::Config> {
    std::fs::write(
        dir.join("manifest.json"),
        canonical_json(&manifest(dir, Some(fault))),
    )
    .unwrap();
    let configs = load(dir);
    let scratch = dir.join("init-meta");
    std::fs::create_dir(&scratch).unwrap();
    for (owner, c) in configs.iter().enumerate() {
        std::fs::copy(plan.snapshot_dirs[target].join(db_name(owner)), &c.db).unwrap();
        {
            let (db, _) = store::open(&c.db, false).unwrap();
            store::set_meta(&db, "manifest_hash", &c.manifest_hash).unwrap();
            let busy: i64 = db
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
                .unwrap();
            assert_eq!(busy, 0);
        }
        let mut fresh = c.clone();
        fresh.db = scratch.join(db_name(owner));
        assert_eq!(store::init(&fresh).unwrap()["kind"], "Initialized");
        let mut expected = plan.snapshots[target][owner].clone();
        expected["meta"] = dump(&fresh.db)["meta"].clone();
        assert_eq!(dump(&c.db), expected, "起点副本域 {owner} ≠ 新 init + 前缀");
    }
    std::fs::remove_dir_all(&scratch).unwrap();
    configs
}
fn setup(dir: &Path, fault: Option<(&str, &str)>) -> Vec<store::Config> {
    std::fs::write(
        dir.join("manifest.json"),
        canonical_json(&manifest(dir, fault)),
    )
    .unwrap();
    let configs = load(dir);
    for c in &configs {
        assert_eq!(store::init(c).unwrap()["kind"], "Initialized");
    }
    configs
}
fn envelope(id: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":SESSION,"session_generation":"1","message_id":id,"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-s3","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}
fn capture_payload(index: usize) -> Value {
    let doc: Value = serde_json::from_str(RAW).unwrap();
    let ev = &doc["events"][index];
    json!({"op":"CaptureExternal","raw_utf8":RAW,"json_pointer":format!("/events/{index}"),"token":format!("token-{}",ev["source"]["external_fact_id"].as_str().unwrap()),"source_event_time":ev["source"]["source_event_time"]})
}
fn resolve_payload(r: &Value, cursor: usize) -> Value {
    json!({"op":"ResolveEvidence","evidence_id":format!("resolve-{}",r["fact_key"].as_str().unwrap()),"covered":[{"fact_key":r["fact_key"],"fact_hash":r["fact_hash"],"token":r["token"]}],"frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":cursor.to_string()},"dependency_version":"testonly-deps/1"})
}
fn apply_payload(r: &Value, id: &Value) -> Value {
    json!({"op":"ApplyAllocation","fact_key":r["fact_key"],"allocation_id":id,"expected_fact_hash":r["fact_hash"]})
}
/// #1622：创建 BindVoice payload，复用第一笔 Capture/Apply 的 fact 与 allocation。
fn bind_payload(capture_reply: &Value, allocation_id: &Value) -> Value {
    let reference = |name: &str| json!({"authority_id":"kill-unit","object_id":name,"revision":"0","content_hash":book::hash(&json!(name))});
    json!({
        "op":"BindVoice",
        "binding_id":"kill-voice-1",
        "evidence_id":"kill-evidence-1",
        "carrier_ref":reference("kill-carrier"),
        "gamma_ref":reference("kill-gamma"),
        "parent_voice_id":null,
        "sigma":"Long",
        "n":"0",
        "opening_basis":{
            "fact_key":capture_reply["fact_key"],
            "expected_fact_hash":capture_reply["fact_hash"],
            "allocation_id":allocation_id
        }
    })
}

/// 全表逻辑转储：每表按主键序、每格原字节（TEXT 原样，BLOB 十六进制）。
fn dump(db: &Path) -> Value {
    let (c, _) = store::open(db, true).unwrap();
    let names = c
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let mut tables = serde_json::Map::new();
    for name in names {
        let mut q = c
            .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY 1"))
            .unwrap();
        let columns = q
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let rows = q
            .query_map([], |r| {
                let mut row = serde_json::Map::new();
                for (i, column) in columns.iter().enumerate() {
                    let cell = match r.get_ref(i)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(n) => json!(n),
                        ValueRef::Real(f) => json!(f),
                        ValueRef::Text(t) => json!(std::str::from_utf8(t).unwrap()),
                        ValueRef::Blob(b) => json!({"blob_hex":b.iter().map(|x| format!("{x:02x}")).collect::<String>()}),
                    };
                    row.insert(column.clone(), cell);
                }
                Ok(Value::Object(row))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        tables.insert(name, Value::Array(rows));
    }
    Value::Object(tables)
}
fn dumps(configs: &[store::Config]) -> Vec<Value> {
    configs.iter().map(|c| dump(&c.db)).collect()
}
fn without(tables: &Value, names: &[&str]) -> Value {
    let mut t = tables.clone();
    for n in names {
        t.as_object_mut().unwrap().remove(*n);
    }
    t
}
/// 业务表：除 meta（含 manifest_hash，随目录与故障计划而变）外全部表。
fn business(tables: &Value) -> Value {
    without(tables, &["meta"])
}
/// TestOnly 时钟列宽：消息 m-i 在 epoch e 的时刻 = (e−1)·COLUMN + 1000 + 10·i + offset（offset ∈ {0,1,2}，
/// 见 `manifest`）；init_ns = 1，第 k 次 recover_ns = k·COLUMN（`store.rs` `Config::clock` 按 epoch−1 取列）。
const COLUMN: i64 = 1_000_000;

/// 时间值的期望列（#1406 S3 评审 M1）：时刻由处理该消息的域按其当时 epoch 取列，
/// 故商 = 该域处理 m-i 时的 epoch−1。崩溃轨里只有被杀域、且只有从 `from` 起（提交前被杀含目标消息，
/// 提交后被杀不含）的消息由 epoch 2 处理；B 记录里抄入的 E 时刻（e_first_known_ns、E 证明的 commit_ns）
/// 归属 E 处理的那条消息，同一判据。
struct Clock {
    owners: Vec<usize>,
    killed: Option<(usize, usize)>,
}
impl Clock {
    fn reference(plan: &Plan) -> Self {
        Self {
            owners: owners(plan),
            killed: None,
        }
    }
    fn crash(plan: &Plan, owner: usize, target: usize, committed: bool) -> Self {
        Self {
            owners: owners(plan),
            killed: Some((owner, if committed { target + 1 } else { target })),
        }
    }
    fn expected_quotient(&self, rem: i64) -> Result<i64, String> {
        match rem {
            1 => return Ok(0), // init_ns
            0 => return Ok(1), // 第一次 recover_ns
            r if r < 1000 => return Err(format!("未知时钟余数 {r}")),
            _ => (),
        }
        let i = ((rem - 1000) / 10) as usize;
        let owner = *self
            .owners
            .get(i)
            .ok_or_else(|| format!("余数 {rem} 不属计划消息"))?;
        Ok(match self.killed {
            Some((o, from)) if o == owner && i >= from => 1,
            _ => 0,
        })
    }
}
fn owners(plan: &Plan) -> Vec<usize> {
    plan.steps
        .iter()
        .map(|s| s["owner"].as_u64().unwrap() as usize)
        .collect()
}
fn is_root_key(k: &str) -> bool {
    k == "root" || k.ends_with("_root") || ["root_hash", "image_hash", "writer_epoch"].contains(&k)
}
/// 规范化：根链类键（root/*_root/root_hash/image_hash，及 epoch 标记 writer_epoch）剔除——其一致性由
/// `audit_all` 重放核；全部 `*_ns` 时刻替换为 `v mod COLUMN` 逐值比较，商按 [`Clock`] 逐值核对，
/// 不符记入 `bad`。字符串形式的 JSON 先解析再规范化。
fn normalize(v: &Value, clock: &Clock, path: &str, bad: &mut Vec<String>) -> Value {
    Normalizer::new(clock).value(v, path, bad)
}
/// 规范化器：同一次比较里内容相同的 JSON 字符串（库内图像逐提交重复嵌入同一原件/记录）只解析、规范化一次。
/// 记忆的是「字符串 → (规范化值, 相对路径的商不合记录)」；命中时按当前路径前缀还原不合记录，
/// 结果与逐次重算逐字相同（规范化只依赖字符串内容与本次时钟，时钟在一个 Normalizer 内不变）。
struct Normalizer<'c> {
    clock: &'c Clock,
    strings: std::collections::HashMap<String, (Value, Vec<String>)>,
}
impl<'c> Normalizer<'c> {
    fn new(clock: &'c Clock) -> Self {
        Self {
            clock,
            strings: Default::default(),
        }
    }
    fn value(&mut self, v: &Value, path: &str, bad: &mut Vec<String>) -> Value {
        match v {
            Value::Object(o) => Value::Object(
                o.iter()
                    .filter(|(k, _)| !is_root_key(k))
                    .map(|(k, x)| {
                        let here = format!("{path}/{k}");
                        let n = match x {
                            Value::String(t) if k.ends_with("_ns") => t.parse::<i64>().ok(),
                            _ => None,
                        };
                        let out = match n {
                            Some(n) => {
                                let (q, r) = (n / COLUMN, n % COLUMN);
                                match self.clock.expected_quotient(r) {
                                    Ok(e) if e == q => (),
                                    Ok(e) => bad.push(format!("{here}={n}：商 {q} ≠ 期望 {e}")),
                                    Err(e) => bad.push(format!("{here}={n}：{e}")),
                                }
                                json!(r.to_string())
                            }
                            None => self.value(x, &here, bad),
                        };
                        (k.clone(), out)
                    })
                    .collect(),
            ),
            Value::Array(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, x)| self.value(x, &format!("{path}/{i}"), bad))
                    .collect(),
            ),
            Value::String(t) if t.starts_with('{') || t.starts_with('[') => {
                if !self.strings.contains_key(t) {
                    let mut rel = vec![];
                    let out = match strict_json(t.as_bytes()) {
                        Ok(p) => self.value(&p, "", &mut rel),
                        Err(_) => v.clone(),
                    };
                    self.strings.insert(t.clone(), (out, rel));
                }
                let (out, rel) = &self.strings[t];
                bad.extend(rel.iter().map(|r| format!("{path}{r}")));
                out.clone()
            }
            _ => v.clone(),
        }
    }
}
/// 参照值按参照时钟规范化（须全部时刻合列）。参照只算一次，存进 [`Plan`]。
fn normalize_reference(v: &Value, reference: &Clock) -> Value {
    let mut bad = vec![];
    let out = normalize(v, reference, "", &mut bad);
    assert!(bad.is_empty(), "参照时刻不合列：{bad:?}");
    out
}
/// 第一处差异的路径。每个子树只走一次（原写法对两侧键各递归一遍，相同键的子树被重复比较，
/// 代价随嵌套深度指数增长——这是 S3 矩阵时长的主因，见 S3-SPEED 报告）。
fn first_diff(a: &Value, b: &Value, path: &str) -> Option<String> {
    if a == b {
        return None;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, p) in x {
                match y.get(k) {
                    Some(q) => {
                        if let Some(d) = first_diff(p, q, &format!("{path}/{k}")) {
                            return Some(d);
                        }
                    }
                    None => return Some(format!("{path}/{k} 只在一侧")),
                }
            }
            y.keys()
                .find(|k| !x.contains_key(*k))
                .map(|k| format!("{path}/{k} 只在一侧"))
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .find_map(|(i, (p, q))| first_diff(p, q, &format!("{path}/{i}"))),
        _ => Some(format!(
            "{path}: {} ≠ {}",
            canonical_json(a).chars().take(160).collect::<String>(),
            canonical_json(b).chars().take(160).collect::<String>()
        )),
    }
}
/// 崩溃轨值 `actual`（时钟 `clock`）与参照值 `expected`（参照时钟）规范化后逐值相同、且全部时刻商合规。
/// 返回 Err(差异描述)，供正向断言与负控共用（负控断言其为 Err，不用 should_panic）。
fn compare(actual: &Value, clock: &Clock, expected: &Value) -> Result<(), String> {
    let mut bad = vec![];
    let a = normalize(actual, clock, "", &mut bad);
    if !bad.is_empty() {
        return Err(format!(
            "时刻商不合：{}",
            bad[..bad.len().min(5)].join("；")
        ));
    }
    match first_diff(&a, expected, "") {
        Some(d) => Err(format!("规范化后不同：{d}")),
        None => Ok(()),
    }
}
fn parsed(cell: &Value) -> Value {
    strict_json(cell.as_str().unwrap().as_bytes()).unwrap()
}
fn audit_all(configs: &[store::Config]) {
    for c in configs {
        let (db, _) = store::open(&c.db, true).unwrap();
        store::audit(&db, c).unwrap();
    }
}
fn bound_receipt(config: &store::Config, message: &str) -> Option<Value> {
    let (c, _) = store::open(&config.db, true).unwrap();
    let k: Option<String> = c
        .query_row(
            "SELECT receipt_key FROM messages WHERE json_extract(request_json,'$.message_id')=?",
            [message],
            |r| r.get(0),
        )
        .optional()
        .unwrap();
    k.map(|k| store::row(&c, "receipts", &k).unwrap().unwrap())
}
/// 公共只读查询：各域 ReadView，B 另加每个事实 QueryFact 与每个 allocation QueryAllocation，E 加 QueryFact。
fn public(configs: &[store::Config]) -> Vec<Value> {
    configs
        .iter()
        .map(|c| {
            let (mut db, _) = store::open(&c.db, true).unwrap();
            let epoch: i64 = store::meta(&db, "writer_epoch").unwrap().parse().unwrap();
            let keys = |db: &rusqlite::Connection, table: &str| {
                db.prepare(&format!("SELECT k FROM {table} ORDER BY k"))
                    .unwrap()
                    .query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>()
            };
            let mut payloads = vec![json!({"op":"ReadView","cut":null,"as_known_ns":null})];
            payloads.extend(
                keys(&db, "facts")
                    .into_iter()
                    .map(|k| json!({"op":"QueryFact","fact_key":k})),
            );
            if c.domain != "E" {
                payloads.extend(
                    keys(&db, "applications")
                        .into_iter()
                        .map(|k| json!({"op":"QueryAllocation","allocation_id":k})),
                );
            }
            Value::Array(
                payloads
                    .into_iter()
                    .map(|p| {
                        let r = service::handle(
                            &mut db,
                            c,
                            epoch,
                            epoch,
                            &json!({}),
                            &envelope("public-query", p),
                        )["payload"]
                            .clone();
                        assert_ne!(r["kind"], "Error", "{r}");
                        r
                    })
                    .collect(),
            )
        })
        .collect()
}
/// 终态金额与 HAND-ORACLE（test_oracle.py 以 Fraction 独立复算）对拍。
fn oracle(plan: &Plan, tables: &[Value]) {
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let last = &hand["reports"][2];
    let image =
        |d: usize| parsed(&tables[d]["commits"].as_array().unwrap().last().unwrap()["image"]);
    for i in 1..4 {
        let b = &image(i)["book"];
        assert_eq!(
            b["actual_quantity"],
            last["book_quantity"][i - 1].as_i64().unwrap().to_string()
        );
        assert_eq!(
            b["cost_basis"]["K"],
            last["K"][i - 1].as_i64().unwrap().to_string()
        );
    }
    let image = image(0);
    for (n, step) in [6, 11, 16].into_iter().enumerate() {
        let fact = &plan.replies[step]["fact_key"];
        let allocations = &image["fill_allocations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| &a["fact_key"] == fact)
            .unwrap()["allocations"];
        for i in 0..3 {
            let r = &hand["reports"][n];
            assert_eq!(
                allocations[i]["units"],
                r["quantity"][i].as_i64().unwrap().to_string()
            );
            assert_eq!(
                allocations[i]["commission"],
                r["fees"][i].as_i64().unwrap().to_string()
            );
        }
    }
}

/// 子进程入口。只由下方 SIGKILL 调度以本测试建立的 job 文件调用。
#[test]
#[ignore = "仅供 tb03c_kill 子进程故障调度调用，不作独立空域通过项"]
fn tb03c_kill_child() {
    let job_path = PathBuf::from(std::env::var_os("TB03C_KILL_JOB").expect("child job required"));
    let job = strict_json(&std::fs::read(&job_path).unwrap()).unwrap();
    let configs = load(Path::new(job["work"].as_str().unwrap()));
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(job["progress"].as_str().unwrap())
        .unwrap();
    // 每步回复落盘并 fsync 后才执行下一步；被杀时文件里恰是已返回的步骤。
    let mut emit = |line: Value| {
        out.write_all(format!("{}\n", canonical_json(&line)).as_bytes())
            .unwrap();
        out.sync_data().unwrap();
    };
    if job["action"] == "recover" {
        let owner = job["owner"].as_u64().unwrap() as usize;
        emit(json!({"owner":owner,"recovered":store::recover(&configs[owner]).unwrap()}));
        return;
    }
    let epochs = job["epochs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_i64().unwrap())
        .collect::<Vec<_>>();
    let mut dbs = configs
        .iter()
        .map(|c| store::open(&c.db, false).unwrap().0)
        .collect::<Vec<_>>();
    for step in job["steps"].as_array().unwrap() {
        let t0 = Instant::now();
        let owner = step["owner"].as_u64().unwrap() as usize;
        let reply = service::handle(
            &mut dbs[owner],
            &configs[owner],
            epochs[owner],
            epochs[0],
            &json!({}),
            &step["envelope"],
        )["payload"]
            .clone();
        let seq = store::latest(&dbs[owner]).unwrap().0;
        emit(
            json!({"owner":owner,"message_id":step["envelope"]["message_id"],"reply":reply,"seq":seq,"ms":t0.elapsed().as_millis() as u64}),
        );
    }
}
fn spawn(work: &Path, label: &str, mut job: Value) -> Child {
    let path = work.join(format!("{label}-job.json"));
    job["work"] = json!(work);
    job["progress"] = json!(work.join(format!("{label}-progress.jsonl")));
    std::fs::write(&path, canonical_json(&job)).unwrap();
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--ignored", "--nocapture"])
        .env("TB03C_KILL_JOB", &path)
        .stdout(std::fs::File::create(work.join(format!("{label}.stdout"))).unwrap())
        .stderr(std::fs::File::create(work.join(format!("{label}.stderr"))).unwrap())
        .spawn()
        .unwrap()
}
fn finish(mut child: Child, work: &Path, label: &str) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "子进程 {label} 失败 {status}: {}",
                std::fs::read_to_string(work.join(format!("{label}.stdout"))).unwrap_or_default()
            );
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("子进程 {label} 超时：{}", work.display());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn progress(work: &Path, label: &str) -> Vec<Value> {
    std::fs::read_to_string(work.join(format!("{label}-progress.jsonl")))
        .unwrap_or_default()
        .lines()
        .map(|l| strict_json(l.as_bytes()).unwrap())
        .collect()
}
fn root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let p = std::env::var_os("TB03C_KILL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("nc1406-s3-kill-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    })
}
fn save(path: &Path, v: &Value) {
    std::fs::write(path, canonical_json(v)).unwrap();
}

/// 不崩溃参照：进程内逐步生成同一组信封并按 HAND 核对，再由新进程原样重放两次（同路径）。
struct Plan {
    steps: Vec<Value>,
    replies: Vec<Value>,
    /// snapshots[i]：第 i 步之前四域业务表；末项为终态。
    snapshots: Vec<Vec<Value>>,
    /// snapshot_dirs[i]：第 i 步之前四域库文件的副本（WAL 已 checkpoint(TRUNCATE) 并入主文件）。
    snapshot_dirs: Vec<PathBuf>,
    /// 逐步回复按参照时钟规范化（只算一次）。
    norm_replies: Vec<Value>,
}
/// 不崩溃参照终态（新进程两次重放）。只在比较终态时才需要，故与 [`Plan`] 分开、延迟计算，
/// 不挡各目标步先开始杀进程。
struct Reference {
    tables: Vec<Value>,
    public: Vec<Value>,
    /// 按参照时钟规范化（只算一次）：终态全表（除 meta/epochs）、公开查询。
    norm_tables: Vec<Value>,
    norm_public: Vec<Value>,
}
struct Builder {
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    steps: Vec<Value>,
    replies: Vec<Value>,
    snapshots: Vec<Vec<Value>>,
    snapshot_dirs: Vec<PathBuf>,
}
impl Builder {
    /// 把当前四域持久状态做成文件副本：先 checkpoint(TRUNCATE) 把 WAL 并入主文件（WAL 归零），再复制主文件。
    /// 副本只供之后新建的被测进程作起点；复制发生在被测进程启动之前。
    fn save_files(&mut self) {
        let dir = root()
            .join("snapshots")
            .join(format!("before-{:02}", self.snapshots.len() - 1));
        std::fs::create_dir_all(&dir).unwrap();
        for (owner, (c, config)) in self.dbs.iter().zip(&self.configs).enumerate() {
            let (busy, log, done): (i64, i64, i64) = c
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })
                .unwrap();
            assert_eq!((busy, log, done), (0, 0, 0), "checkpoint 未完全并入");
            assert_eq!(
                std::fs::metadata(format!("{}-wal", config.db.display()))
                    .map(|m| m.len())
                    .unwrap_or(0),
                0
            );
            std::fs::copy(&config.db, dir.join(db_name(owner))).unwrap();
        }
        self.snapshot_dirs.push(dir);
    }
    fn send(&mut self, owner: usize, payload: Value) -> Value {
        let e = envelope(&format!("m-{}", self.steps.len()), payload);
        let r = service::handle(
            &mut self.dbs[owner],
            &self.configs[owner],
            1,
            1,
            &json!({}),
            &e,
        )["payload"]
            .clone();
        assert!(
            matches!(
                r["kind"].as_str(),
                Some("Recorded" | "Resolved" | "Applied" | "VoiceBound")
            ),
            "{r}"
        );
        self.steps.push(json!({"owner":owner,"envelope":e}));
        self.replies.push(r.clone());
        self.snapshots
            .push(dumps(&self.configs).iter().map(business).collect());
        self.save_files();
        r
    }
}
fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        let plan_t0 = Instant::now();
        let space = Space::new("s3-kill-plan");
        let configs = setup(&space.0, None);
        let dbs = configs
            .iter()
            .map(|c| store::open(&c.db, false).unwrap().0)
            .collect();
        let snapshots = vec![dumps(&configs).iter().map(business).collect()];
        let mut b = Builder {
            configs,
            dbs,
            steps: vec![],
            replies: vec![],
            snapshots,
            snapshot_dirs: vec![],
        };
        b.save_files();
        let hand: Value = serde_json::from_str(HAND).unwrap();
        for i in 0..3 {
            let r = b.send(0, capture_payload(i));
            b.send(i + 1, apply_payload(&r, &r["allocation_id"]));
        }
        for n in 0..3 {
            let r = b.send(0, capture_payload(n + 3));
            b.send(0, resolve_payload(&r, n + 4));
            let image = store::latest(&b.dbs[0]).unwrap().3;
            let allocations = image["fill_allocations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["fact_key"] == r["fact_key"])
                .unwrap()["allocations"]
                .clone();
            for i in 0..3 {
                assert_eq!(allocations[i]["chong_id"], format!("chong-N{}", i + 1));
                b.send(i + 1, apply_payload(&r, &allocations[i]["allocation_id"]));
                let book = &store::latest(&b.dbs[i + 1]).unwrap().3["book"];
                let report = &hand["reports"][n];
                assert_eq!(
                    book["actual_quantity"],
                    report["book_quantity"][i].as_i64().unwrap().to_string()
                );
                assert_eq!(
                    book["cost_basis"]["K"],
                    report["K"][i].as_i64().unwrap().to_string()
                );
            }
        }
        // #1622：在第一笔 Apply（步骤 1）后添加 BindVoice（步骤 21 → 总 22 步，owner=1=B:chong-N1）
        let first_capture = &b.replies[0];
        let first_apply = &b.replies[1];
        assert_eq!(first_capture["kind"], "Recorded");
        assert_eq!(first_apply["kind"], "Applied");
        b.send(
            1,
            bind_payload(first_capture, &first_capture["allocation_id"]),
        );
        assert_eq!(b.steps.len(), 22);
        assert_eq!(b.snapshot_dirs.len(), 23);
        let mut plan = Plan {
            steps: b.steps,
            replies: b.replies,
            snapshots: b.snapshots,
            snapshot_dirs: b.snapshot_dirs,
            norm_replies: vec![],
        };
        let clock = Clock::reference(&plan);
        plan.norm_replies = plan
            .replies
            .iter()
            .map(|r| normalize_reference(r, &clock))
            .collect();
        println!(
            "S3-TIME {}",
            canonical_json(&json!({"plan_ms":plan_t0.elapsed().as_millis()}))
        );
        plan
    })
}
fn reference() -> &'static Reference {
    static REFERENCE: OnceLock<Reference> = OnceLock::new();
    REFERENCE.get_or_init(|| {
        let t0 = Instant::now();
        let plan = plan();
        // 无故障新进程重放：从空库起、同一信封、同一路径两次，回复与业务表须与进程内生成逐字相同。
        // 这两次全程重放同时证明了各快照所代表的前缀状态可由新进程从头逐步得到。
        let base = root().join("reference");
        let work = base.join("work");
        let mut finals: Vec<(Vec<Value>, Vec<Value>)> = vec![];
        for rep in 0..2 {
            std::fs::create_dir_all(&work).unwrap();
            let configs = setup(&work, None);
            finish(
                spawn(
                    &work,
                    "run",
                    json!({"action":"run","epochs":[1,1,1,1],"steps":plan.steps}),
                ),
                &work,
                "run",
            );
            let lines = progress(&work, "run");
            assert_eq!(lines.len(), plan.steps.len());
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(line["reply"], plan.replies[i], "参照新进程第 {i} 步");
            }
            let tables = dumps(&configs);
            for d in 0..4 {
                assert_eq!(business(&tables[d]), plan.snapshots[22][d]);
            }
            audit_all(&configs);
            let queries = public(&configs);
            std::fs::rename(&work, base.join(format!("rep-{rep}"))).unwrap();
            finals.push((tables, queries));
        }
        assert_eq!(
            canonical_json(&json!(finals[0].0)),
            canonical_json(&json!(finals[1].0)),
            "无故障同输入两次新进程完整终态逐字节相同"
        );
        let (tables, public) = finals.swap_remove(0);
        oracle(plan, &tables);
        let clock = Clock::reference(plan);
        let norm_tables = tables
            .iter()
            .map(|t| normalize_reference(&without(t, &["meta", "epochs"]), &clock))
            .collect();
        let norm_public = public
            .iter()
            .map(|q| normalize_reference(q, &clock))
            .collect();
        println!(
            "S3-TIME {}",
            canonical_json(&json!({"reference_ms":t0.elapsed().as_millis()}))
        );
        Reference {
            tables,
            public,
            norm_tables,
            norm_public,
        }
    })
}

/// 被杀并恢复后的现场（尚未重送）。
struct Killed {
    configs: Vec<store::Config>,
    owner: usize,
    message: String,
    pid: u32,
    signal: Option<i32>,
    marker: Value,
    wal_bytes: Option<u64>,
    kill_seq: i64,
    original: Option<Value>,
    recovered: Value,
}

/// 新进程 A 跑到目标消息并停在持久边界 → SIGKILL → 边界断言 → 新进程 recover。
fn kill_and_recover(plan: &Plan, target: usize, phase: &str, work: &Path) -> Killed {
    let committed = phase == "after_commit";
    let owner = plan.steps[target]["owner"].as_u64().unwrap() as usize;
    // 评审 M3(a)：目标步确实改变被杀域，「提交前=前快照 / 提交后=后快照」两判据不重合。
    assert_ne!(
        plan.snapshots[target][owner],
        plan.snapshots[target + 1][owner],
        "目标步 {target} 未改变被杀域，边界判据退化"
    );
    let message = plan.steps[target]["envelope"]["message_id"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::create_dir_all(work.parent().unwrap()).unwrap();
    std::fs::create_dir(work).unwrap();
    let configs = setup_from(plan, work, (&message, phase), target);
    let mut a = spawn(
        work,
        "a",
        json!({"action":"run","epochs":[1,1,1,1],"steps":plan.steps[target..=target]}),
    );
    let pid = a.id();
    let marker_path = work.join("fault-marker.json");
    let deadline = Instant::now() + DEADLINE;
    // marker 由 pause 以 create_new 写入并 fsync 后进入无限停驻；读到完整内容才 kill。
    let marker = loop {
        if let Some(status) = a.try_wait().unwrap() {
            panic!("子进程未到故障缝即退出 {status}: {}", work.display())
        }
        if Instant::now() >= deadline {
            let _ = a.kill();
            let _ = a.wait();
            panic!("故障 marker 超时：{}", work.display())
        }
        if let Ok(bytes) = std::fs::read(&marker_path) {
            if let Ok(v) = strict_json(&bytes) {
                break v;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        marker,
        json!({"domain_id":domain_id(owner),"message_id":message,"phase":phase,"pid":pid,"test_only":true})
    );
    a.kill().unwrap();
    let killed = a.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(killed.signal(), Some(9));
    let wal_bytes = std::fs::metadata(format!("{}-wal", configs[owner].db.display()))
        .map(|m| m.len())
        .ok();

    // 被杀进程从目标步前的副本起只执行目标消息，且未返回任何回复。
    // （前缀各步由 `reference()` 的两次新进程全程重放逐字核对。）
    assert!(progress(work, "a").is_empty(), "目标消息不得有回复");
    // 持久边界：提交前被杀 = 目标前快照（事务内已写的全部行均未落盘）；提交后被杀 = 目标后快照。
    let boundary = if committed { target + 1 } else { target };
    let at_kill = dumps(&configs);
    for d in 0..4 {
        assert_eq!(
            business(&at_kill[d]),
            plan.snapshots[boundary][d],
            "{phase} 被杀后域 {d} 业务表"
        );
    }
    audit_all(&configs);
    let kill_seq = at_kill[owner]["commits"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["seq"]
        .as_i64()
        .unwrap();
    let original = bound_receipt(&configs[owner], &message);
    assert_eq!(original.is_some(), committed);
    if let Some(r) = &original {
        assert_eq!(r, &plan.replies[target]);
    }

    finish(
        spawn(work, "recover", json!({"action":"recover","owner":owner})),
        work,
        "recover",
    );
    let recovered = progress(work, "recover")[0]["recovered"].clone();
    assert_eq!(recovered["kind"], "Recovered");
    assert_eq!(recovered["writer_epoch"], "2");
    let after_recover = dump(&configs[owner].db);
    assert_eq!(
        without(&after_recover, &["meta", "epochs"]),
        without(&at_kill[owner], &["meta", "epochs"]),
        "recover 不改业务表"
    );
    assert_eq!(
        after_recover["epochs"],
        json!([{"epoch":1,"phase":"init","at_ns":"1"},{"epoch":2,"phase":"recover","at_ns":"1000000"}])
    );
    Killed {
        configs,
        owner,
        message,
        pid,
        signal: killed.signal(),
        marker,
        wal_bytes,
        kill_seq,
        original,
        recovered,
    }
}

/// 新进程续跑 `steps`（被杀域用 epoch 2），返回逐步进度行。
fn resume(k: &Killed, work: &Path, steps: Vec<Value>) -> Vec<Value> {
    let mut epochs = vec![1, 1, 1, 1];
    epochs[k.owner] = 2;
    finish(
        spawn(
            work,
            "c",
            json!({"action":"run","epochs":epochs,"steps":steps}),
        ),
        work,
        "c",
    );
    let after = progress(work, "c");
    assert_eq!(after.len(), steps.len());
    after
}

/// 终态四域全表（除 meta/epochs）与公开查询对不崩溃参照：规范化逐值比较（返回第一处差异）。
fn final_diff(k: &Killed, clock: &Clock) -> Result<(Vec<Value>, Vec<Value>), String> {
    let reference = reference();
    let finals = dumps(&k.configs);
    for d in 0..4 {
        compare(
            &without(&finals[d], &["meta", "epochs"]),
            clock,
            &reference.norm_tables[d],
        )
        .map_err(|e| format!("终态域 {d}：{e}"))?;
    }
    let queries = public(&k.configs);
    for d in 0..4 {
        compare(&queries[d], clock, &reference.norm_public[d])
            .map_err(|e| format!("公开查询域 {d}：{e}"))?;
    }
    Ok((finals, queries))
}

/// 一次完整轨：kill → recover → 原样重送两次 → 续跑到终态。
fn run_once(plan: &Plan, target: usize, phase: &str, work: &Path) -> Value {
    let committed = phase == "after_commit";
    let t0 = Instant::now();
    let k = kill_and_recover(plan, target, phase, work);
    let t_kill = t0.elapsed().as_millis();
    let (owner, kill_seq) = (k.owner, k.kill_seq);
    let clock = Clock::crash(plan, owner, target, committed);
    let mut steps = vec![plan.steps[target].clone(), plan.steps[target].clone()];
    steps.extend_from_slice(&plan.steps[target + 1..]);
    let after = resume(&k, work, steps);
    let t_resume = t0.elapsed().as_millis();
    let a_steps: u64 = progress(work, "a")
        .iter()
        .map(|l| l["ms"].as_u64().unwrap_or(0))
        .sum();
    let c_steps: u64 = after.iter().map(|l| l["ms"].as_u64().unwrap_or(0)).sum();
    let resent = &after[0];
    if committed {
        // 提交后被杀：重送返回原持久回执，不新增提交。
        assert_eq!(&resent["reply"], k.original.as_ref().unwrap());
        assert_eq!(resent["seq"], json!(kill_seq));
    } else {
        // 提交前被杀：重送恰好新增一个提交；回执时刻取第二列（商 1），规范化后与参照回执逐字相同。
        assert_ne!(resent["reply"], plan.replies[target]);
        assert_eq!(
            compare(&resent["reply"], &clock, &plan.norm_replies[target]),
            Ok(()),
            "重送回执"
        );
        assert_eq!(resent["seq"], json!(kill_seq + 1));
    }
    assert_eq!(after[1]["reply"], resent["reply"], "再次重送同回执");
    assert_eq!(after[1]["seq"], resent["seq"], "再次重送无新提交");
    for (n, line) in after[2..].iter().enumerate() {
        let i = target + 1 + n;
        assert_eq!(
            compare(&line["reply"], &clock, &plan.norm_replies[i]),
            Ok(()),
            "续跑第 {i} 步"
        );
    }
    audit_all(&k.configs);
    let (finals, queries) = final_diff(&k, &clock).unwrap();
    let reference = reference();
    for d in 0..4 {
        if owner != 0 && d != owner {
            // B 崩溃不触及 E 与其它 B：业务表与公开查询逐字相同。
            assert_eq!(business(&finals[d]), business(&reference.tables[d]));
            assert_eq!(queries[d], reference.public[d]);
        }
    }
    oracle(plan, &finals);
    println!(
        "S3-TIME {}",
        canonical_json(
            &json!({"target":target,"phase":phase,"kill_and_recover_ms":t_kill,"resume_done_ms":t_resume,"total_ms":t0.elapsed().as_millis(),"a_step_ms":a_steps,"c_step_ms":c_steps})
        )
    );
    let mut recovered_record = k.recovered.clone();
    recovered_record.as_object_mut().unwrap().remove("runtime");
    json!({"scope":"local transaction process fault seam (Config::pause around tx.commit); not formal sockets/S/browser",
        "message_id":k.message,"op":plan.steps[target]["envelope"]["payload"]["op"],"domain_id":domain_id(owner),"phase":phase,
        "pid":k.pid,"signal":k.signal,"marker":k.marker,"wal_bytes_at_kill":k.wal_bytes,"kill_seq":kill_seq,
        "original_receipt":k.original,"recovered":recovered_record,"resend":resent,
        "continuation_replies":after.iter().map(|l| l["reply"].clone()).collect::<Vec<_>>(),
        "final_tables_sha256":book::hash(&json!(finals)),"final_tables":finals,"public":queries})
}
/// 一个（目标步, 阶段）组合：两次全新目录、全新 A/R/C 进程，终态全表与续跑回复逐字节相同。
/// 降时长后每组合一个测试（原为每步一个测试内串行两阶段），调度粒度减半、工作量不变。
fn matrix(target: usize, op: &str, owner: usize, phase: &str) {
    let plan = plan();
    assert_eq!(plan.steps[target]["envelope"]["payload"]["op"], op);
    assert_eq!(plan.steps[target]["owner"], owner);
    let label = format!("step{target:02}");
    {
        let base = root().join(&label).join(phase);
        let work = base.join("work");
        let mut finals = vec![];
        for rep in 0..2 {
            let evidence = run_once(plan, target, phase, &work);
            let archived = base.join(format!("rep-{rep}"));
            std::fs::rename(&work, &archived).unwrap();
            save(&archived.join("evidence.json"), &evidence);
            println!(
                "S3-KILL {}",
                canonical_json(
                    &json!({"label":label,"op":op,"phase":phase,"rep":rep,"message_id":evidence["message_id"],"domain_id":evidence["domain_id"],"pid":evidence["pid"],"signal":evidence["signal"],"kill_seq":evidence["kill_seq"],"resend_kind":evidence["resend"]["reply"]["kind"],"resend_seq":evidence["resend"]["seq"],"final_tables_sha256":evidence["final_tables_sha256"],"raw_dir":archived})
                )
            );
            finals.push((
                canonical_json(&evidence["final_tables"]),
                evidence["continuation_replies"].clone(),
            ));
        }
        assert_eq!(
            finals[0].0, finals[1].0,
            "{label}/{phase} 两次新进程完整终态逐字节相同"
        );
        assert_eq!(
            finals[0].1, finals[1].1,
            "{label}/{phase} 两次新进程续跑回复相同"
        );
    }
}

/// WE7 持久写步（评审 M2 + #1622 BindVoice）：22 步（步号 / 操作 / 所属域，0=E，1..3=B:chong-N1..N3）。
/// 每步生成 `<名>::before_commit` 与 `<名>::after_commit` 两个测试（共 44 个组合 × 2 次 = 88 次 SIGKILL）。
macro_rules! kill_step {
    ($($name:ident: $target:expr, $op:expr, $owner:expr;)*) => {$(
        mod $name {
            #[test]
            fn before_commit() {
                super::matrix($target, $op, $owner, "before_commit");
            }
            #[test]
            fn after_commit() {
                super::matrix($target, $op, $owner, "after_commit");
            }
        }
    )*};
}
kill_step! {
    tb03c_kill_s00_capture_open_n1: 0, "CaptureExternal", 0;
    tb03c_kill_s01_apply_open_n1: 1, "ApplyAllocation", 1;
    tb03c_kill_s02_capture_open_n2: 2, "CaptureExternal", 0;
    tb03c_kill_s03_apply_open_n2: 3, "ApplyAllocation", 2;
    tb03c_kill_s04_capture_open_n3: 4, "CaptureExternal", 0;
    tb03c_kill_s05_apply_open_n3: 5, "ApplyAllocation", 3;
    tb03c_kill_s06_capture_fill1: 6, "CaptureExternal", 0;
    tb03c_kill_s07_resolve_fill1: 7, "ResolveEvidence", 0;
    tb03c_kill_s08_apply_fill1_n1_zero_share: 8, "ApplyAllocation", 1;
    tb03c_kill_s09_apply_fill1_n2: 9, "ApplyAllocation", 2;
    tb03c_kill_s10_apply_fill1_n3: 10, "ApplyAllocation", 3;
    tb03c_kill_s11_capture_fill2: 11, "CaptureExternal", 0;
    tb03c_kill_s12_resolve_fill2: 12, "ResolveEvidence", 0;
    tb03c_kill_s13_apply_fill2_n1: 13, "ApplyAllocation", 1;
    tb03c_kill_s14_apply_fill2_n2: 14, "ApplyAllocation", 2;
    tb03c_kill_s15_apply_fill2_n3: 15, "ApplyAllocation", 3;
    tb03c_kill_s16_capture_fill3: 16, "CaptureExternal", 0;
    tb03c_kill_s17_resolve_fill3: 17, "ResolveEvidence", 0;
    tb03c_kill_s18_apply_fill3_n1: 18, "ApplyAllocation", 1;
    tb03c_kill_s19_apply_fill3_n2: 19, "ApplyAllocation", 2;
    tb03c_kill_s20_apply_fill3_n3: 20, "ApplyAllocation", 3;
    tb03c_kill_s21_bind_voice_n1: 21, "BindVoice", 1;
}

/// 评审 M3(b) 负控：提交前被杀并恢复后**跳过重送**直接续跑，终态比较函数必须报「不同」。
/// 证明 `final_diff` 能区分「恰好一次」与「丢失一次」；断言写在返回值上，不用 should_panic。
fn skip_resend(target: usize) -> String {
    let plan = plan();
    let work = root()
        .join(format!("negative-skip-{target:02}"))
        .join("work");
    let k = kill_and_recover(plan, target, "before_commit", &work);
    let clock = Clock::crash(plan, k.owner, target, false);
    let after = resume(&k, &work, plan.steps[target + 1..].to_vec());
    let errors = after
        .iter()
        .filter(|l| l["reply"]["kind"] == "Error")
        .count();
    let diff = final_diff(&k, &clock).expect_err("跳过重送后终态竟与参照相同：比较函数无区分力");
    println!(
        "S3-NEGATIVE {}",
        canonical_json(&json!({"target":target,"continuation_errors":errors,"first_diff":diff}))
    );
    diff
}

/// #1622 负控：BindVoice 使用不存在的 allocation_id，必须在应用查找处拒绝。
fn bind_voice_wrong_allocation() -> String {
    let plan = plan();
    let work = root().join("negative-wrong-allocation").join("work");
    std::fs::create_dir_all(&work).unwrap();
    let configs = setup(&work, None);
    let mut dbs: Vec<rusqlite::Connection> = configs
        .iter()
        .map(|c| store::open(&c.db, false).unwrap().0)
        .collect();
    // 执行前两步：步骤0（Capture open N1）和步骤1（Apply open N1）
    for i in 0..2 {
        let owner = plan.steps[i]["owner"].as_u64().unwrap() as usize;
        let reply = service::handle(
            &mut dbs[owner],
            &configs[owner],
            1,
            1,
            &json!({}),
            &plan.steps[i]["envelope"],
        )["payload"]
            .clone();
        assert_ne!(reply["kind"], "Error", "前置步骤失败：{reply}");
    }
    let first_capture = &plan.replies[0];
    // 使用错误的 allocation_id（应该用 "alloc-N1-open"，故意用错误值）
    let wrong_allocation = "wrong-allocation-id";
    let bad_payload = bind_payload(first_capture, &json!(wrong_allocation));
    // 复用已配置的 BindVoice 时钟项，确保负控走到 allocation 校验而非时钟拒绝。
    let bad_envelope = envelope("m-21", bad_payload);
    let error_reply = service::handle(&mut dbs[1], &configs[1], 1, 1, &json!({}), &bad_envelope)
        ["payload"]
        .clone();
    // 该 allocation 未应用，生产入口在 voice.rs 的查找处返回 MissingDependency。
    if error_reply["kind"] != "Error" {
        panic!("预期 Error，实际得到：{}", canonical_json(&error_reply));
    }
    let error_reason = error_reply["error"].as_str().unwrap_or("").to_string();
    println!(
        "S3-NEGATIVE {}",
        canonical_json(
            &json!({"test":"bind_voice_wrong_allocation","error_kind":error_reply["kind"],"error_reason":error_reason})
        )
    );
    error_reason
}
#[test]
fn tb03c_kill_negative_skip_resend_capture_fill1() {
    let diff = skip_resend(6);
    assert!(diff.starts_with("终态域 0："), "{diff}");
}
#[test]
fn tb03c_kill_negative_skip_resend_apply_fill1_n2() {
    let diff = skip_resend(9);
    assert!(diff.starts_with("终态域 2："), "{diff}");
}

/// #1622 负控：不存在的 allocation_id 不得绑定 Voice。
#[test]
fn tb03c_kill_negative_bind_voice_wrong_allocation() {
    let reason = bind_voice_wrong_allocation();
    assert_eq!(
        reason, "MissingDependency：Voice 开局 allocation 尚未应用",
        "负控必须到达 allocation 查找失败，而不是其他前置错误"
    );
}
