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
    let mut m = json!({"schema_revision":"economic-manifest/1","session_id":SESSION,"session_generation":"1","book_revision":"tb03c-book/1",
        "e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},"chongs":chongs,"fill_manifests":[serde_json::from_str::<Value>(MANIFEST).unwrap()],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-s3","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":((1..=16).map(|r| (r * 1_000_000).to_string()).collect::<Vec<_>>()),"messages":plan}},
        "fill_allocation_policy":{"fill_allocation_policy_id":super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"we7-frozen-1"}});
    if let Some((message, phase)) = fault {
        m["test_only_fault"] = json!({"message_id":message,"phase":phase,"marker_path":dir.join("fault-marker.json")});
    }
    m
}
fn chong(owner: usize) -> Option<String> {
    (owner > 0).then(|| format!("chong-N{owner}"))
}
fn domain_id(owner: usize) -> String {
    chong(owner).map_or_else(|| "E".to_owned(), |c| format!("B:{c}"))
}
fn load(dir: &Path) -> Vec<store::Config> {
    let path = dir.join("manifest.json");
    (0..4)
        .map(|owner| {
            let db = if owner == 0 {
                dir.join("e.db")
            } else {
                dir.join(format!("b{}.db", owner - 1))
            };
            store::Config::load(&path, &db, chong(owner).as_deref()).unwrap()
        })
        .collect()
}
fn setup(dir: &Path, fault: Option<(&str, &str)>) -> Vec<store::Config> {
    std::fs::write(dir.join("manifest.json"), canonical_json(&manifest(dir, fault))).unwrap();
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
/// 去掉依定义随恢复变化的派生值：全部 *_ns 时刻、根链（root/*_root/root_hash）、image_hash、writer_epoch。
/// 崩溃轨在恢复域用第二时钟列提交，这些值必然不同；其余（数量、金额、K、分配、事实/回执身份、commit_seq）逐值比较。
fn project(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, _)| {
                    !(k.ends_with("_ns")
                        || k.as_str() == "root"
                        || k.ends_with("_root")
                        || ["root_hash", "image_hash", "writer_epoch"].contains(&k.as_str()))
                })
                .map(|(k, v)| (k.clone(), project(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(project).collect()),
        Value::String(s) if s.starts_with('{') || s.starts_with('[') => strict_json(s.as_bytes())
            .map(|p| project(&p))
            .unwrap_or_else(|_| v.clone()),
        _ => v.clone(),
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
    let image = |d: usize| parsed(&tables[d]["commits"].as_array().unwrap().last().unwrap()["image"]);
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
        emit(json!({"owner":owner,"message_id":step["envelope"]["message_id"],"reply":reply,"seq":seq}));
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
    reference: Vec<Value>,
    reference_public: Vec<Value>,
}
struct Builder {
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    steps: Vec<Value>,
    replies: Vec<Value>,
    snapshots: Vec<Vec<Value>>,
}
impl Builder {
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
                Some("Recorded" | "Resolved" | "Applied")
            ),
            "{r}"
        );
        self.steps.push(json!({"owner":owner,"envelope":e}));
        self.replies.push(r.clone());
        self.snapshots
            .push(dumps(&self.configs).iter().map(business).collect());
        r
    }
}
fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
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
        };
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
        assert_eq!(b.steps.len(), 21);
        // 无故障新进程重放：同一信封、同一路径两次，回复与业务表须与进程内生成逐字相同。
        let base = root().join("reference");
        let work = base.join("work");
        let mut finals: Vec<(Vec<Value>, Vec<Value>)> = vec![];
        for rep in 0..2 {
            std::fs::create_dir_all(&work).unwrap();
            let configs = setup(&work, None);
            finish(
                spawn(&work, "run", json!({"action":"run","epochs":[1,1,1,1],"steps":b.steps})),
                &work,
                "run",
            );
            let lines = progress(&work, "run");
            assert_eq!(lines.len(), b.steps.len());
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(line["reply"], b.replies[i], "参照新进程第 {i} 步");
            }
            let tables = dumps(&configs);
            for d in 0..4 {
                assert_eq!(business(&tables[d]), b.snapshots[21][d]);
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
        let (reference, reference_public) = finals.swap_remove(0);
        let plan = Plan {
            steps: b.steps,
            replies: b.replies,
            snapshots: b.snapshots,
            reference,
            reference_public,
        };
        oracle(&plan, &plan.reference);
        plan
    })
}

/// 一次：新进程 A 跑到目标消息并停在持久边界 → SIGKILL → 新进程 recover → 新进程重送原消息两次并续跑到终态。
fn run_once(plan: &Plan, target: usize, phase: &str, work: &Path) -> Value {
    let committed = phase == "after_commit";
    let owner = plan.steps[target]["owner"].as_u64().unwrap() as usize;
    let message = plan.steps[target]["envelope"]["message_id"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::create_dir_all(work.parent().unwrap()).unwrap();
    std::fs::create_dir(work).unwrap();
    let configs = setup(work, Some((&message, phase)));
    let mut a = spawn(
        work,
        "a",
        json!({"action":"run","epochs":[1,1,1,1],"steps":plan.steps[..=target]}),
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

    // 被杀进程已逐字返回目标前全部步骤，目标消息无回复。
    let before = progress(work, "a");
    assert_eq!(before.len(), target);
    for (i, line) in before.iter().enumerate() {
        assert_eq!(line["reply"], plan.replies[i], "故障前第 {i} 步");
    }
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
    let kill_seq = at_kill[owner]["commits"].as_array().unwrap().last().unwrap()["seq"]
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

    let mut epochs = vec![1, 1, 1, 1];
    epochs[owner] = 2;
    let mut steps = vec![plan.steps[target].clone(), plan.steps[target].clone()];
    steps.extend_from_slice(&plan.steps[target + 1..]);
    finish(
        spawn(work, "c", json!({"action":"run","epochs":epochs,"steps":steps})),
        work,
        "c",
    );
    let after = progress(work, "c");
    assert_eq!(after.len(), steps.len());
    let resent = &after[0];
    if committed {
        // 提交后被杀：重送返回原持久回执，不新增提交。
        assert_eq!(&resent["reply"], original.as_ref().unwrap());
        assert_eq!(resent["seq"], json!(kill_seq));
    } else {
        // 提交前被杀：重送恰好新增一个提交，回执是恢复后新提交（第二时钟列），经济内容与参照相同。
        assert_eq!(resent["reply"]["kind"], plan.replies[target]["kind"]);
        assert_ne!(resent["reply"], plan.replies[target]);
        assert_eq!(project(&resent["reply"]), project(&plan.replies[target]));
        assert_eq!(resent["seq"], json!(kill_seq + 1));
    }
    assert_eq!(after[1]["reply"], resent["reply"], "再次重送同回执");
    assert_eq!(after[1]["seq"], resent["seq"], "再次重送无新提交");
    for (k, line) in after[2..].iter().enumerate() {
        let i = target + 1 + k;
        assert_eq!(line["reply"]["kind"], plan.replies[i]["kind"], "续跑第 {i} 步");
        assert_eq!(project(&line["reply"]), project(&plan.replies[i]), "续跑第 {i} 步");
    }

    let finals = dumps(&configs);
    audit_all(&configs);
    for d in 0..4 {
        assert_eq!(
            project(&without(&finals[d], &["meta", "epochs"])),
            project(&without(&plan.reference[d], &["meta", "epochs"])),
            "终态域 {d} 与不崩溃参照逐值相同（去时钟派生）"
        );
        if owner != 0 && d != owner {
            // B 崩溃不触及 E 与其它 B：业务表与参照逐字相同。
            assert_eq!(business(&finals[d]), business(&plan.reference[d]));
        }
    }
    oracle(plan, &finals);
    let queries = public(&configs);
    for d in 0..4 {
        assert_eq!(project(&queries[d]), project(&plan.reference_public[d]));
        if owner != 0 && d != owner {
            assert_eq!(queries[d], plan.reference_public[d]);
        }
    }
    let mut recovered_record = recovered.clone();
    recovered_record.as_object_mut().unwrap().remove("runtime");
    json!({"scope":"local transaction process fault seam (Config::pause around tx.commit); not formal sockets/S/browser",
        "message_id":message,"op":plan.steps[target]["envelope"]["payload"]["op"],"domain_id":domain_id(owner),"phase":phase,
        "pid":pid,"signal":killed.signal(),"marker":marker,"wal_bytes_at_kill":wal_bytes,"kill_seq":kill_seq,
        "original_receipt":original,"recovered":recovered_record,"resend":resent,
        "continuation_replies":after.iter().map(|l| l["reply"].clone()).collect::<Vec<_>>(),
        "final_tables_sha256":book::hash(&json!(finals)),"final_tables":finals,"public":queries})
}
fn matrix(label: &str, target: usize, op: &str, owner: usize) {
    let plan = plan();
    assert_eq!(plan.steps[target]["envelope"]["payload"]["op"], op);
    assert_eq!(plan.steps[target]["owner"], owner);
    for phase in ["before_commit", "after_commit"] {
        let base = root().join(label).join(phase);
        let work = base.join("work");
        let mut finals = vec![];
        for rep in 0..2 {
            let evidence = run_once(plan, target, phase, &work);
            let archived = base.join(format!("rep-{rep}"));
            std::fs::rename(&work, &archived).unwrap();
            save(&archived.join("evidence.json"), &evidence);
            println!(
                "S3-KILL {}",
                canonical_json(&json!({"label":label,"phase":phase,"rep":rep,"message_id":evidence["message_id"],"domain_id":evidence["domain_id"],"pid":evidence["pid"],"signal":evidence["signal"],"kill_seq":evidence["kill_seq"],"resend_kind":evidence["resend"]["reply"]["kind"],"resend_seq":evidence["resend"]["seq"],"final_tables_sha256":evidence["final_tables_sha256"],"raw_dir":archived}))
            );
            finals.push((
                canonical_json(&evidence["final_tables"]),
                evidence["continuation_replies"].clone(),
            ));
        }
        assert_eq!(finals[0].0, finals[1].0, "{label}/{phase} 两次新进程完整终态逐字节相同");
        assert_eq!(finals[0].1, finals[1].1, "{label}/{phase} 两次新进程续跑回复相同");
    }
}

#[test]
fn tb03c_kill_capture_opening() {
    matrix("capture_opening", 0, "CaptureExternal", 0);
}
#[test]
fn tb03c_kill_apply_opening_n1() {
    matrix("apply_opening_n1", 1, "ApplyAllocation", 1);
}
#[test]
fn tb03c_kill_capture_netted_fill1() {
    matrix("capture_netted_fill1", 6, "CaptureExternal", 0);
}
#[test]
fn tb03c_kill_resolve_fill1() {
    matrix("resolve_fill1", 7, "ResolveEvidence", 0);
}
#[test]
fn tb03c_kill_apply_fill1_n1_zero_share() {
    matrix("apply_fill1_n1_zero_share", 8, "ApplyAllocation", 1);
}
#[test]
fn tb03c_kill_apply_fill1_n2() {
    matrix("apply_fill1_n2", 9, "ApplyAllocation", 2);
}
#[test]
fn tb03c_kill_apply_fill1_n3() {
    matrix("apply_fill1_n3", 10, "ApplyAllocation", 3);
}
#[test]
fn tb03c_kill_apply_fill3_n1() {
    matrix("apply_fill3_n1", 18, "ApplyAllocation", 1);
}
