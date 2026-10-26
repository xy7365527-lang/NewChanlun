//! #1674（#1406 AC5）：C8 引入的 `tb03c-book/2` 持久边界的新进程与真实 SIGKILL 恢复对拍。
//! 机制同 S3（`tb03c_kill_tests.rs`）：故障缝是生产既有的 `Config::pause`（同一写事务内 `tx.commit()` 紧前与紧后），
//! 子进程由 `current_exe` 以具名 ignored 测试重入，经 `service::handle` 执行；生产语义零改动。
//! manifest 冻结 C8 资金合同（funding-record、每重 ⟨Q, policy-record⟩、margin_leverage）；三个签署 profile 各一份计划，
//! 终态与不中断基线、与 FUNDING-ORACLE.json 逐值对拍。范围是本地事务进程故障缝，不是正式 socket/S/浏览器。
use super::tb03c_book_tests::Space;
use super::tb03c_funding_tests::{chong_id, funding_record, policy_record, raw, Batch};
use super::tb03c_kill_tests::{
    audit_all, bound_receipt, business, compare, dump, dumps, finish, normalize_reference, parsed,
    progress, save, without, Clock,
};
use super::{book, service, store};
use crate::session_protocol::{canonical_json, strict_json};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ORACLE: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_funding/FUNDING-ORACLE.json");
const SESSION: &str = "tb1674-v2-kill";
const CHILD: &str = "economic_session::tb03c_v2_kill_tests::tb03c_v2_kill_child";
const DEADLINE: Duration = Duration::from_secs(300);
/// 时钟计划覆盖的消息数（计划最长为 P0-acceptance-5m：三重 × 6 步 = 18）。
const MESSAGES: usize = 32;
const PROFILES: [&str; 3] = ["P0", "P1-hl-testnet", "P0-acceptance-5m"];
const VALID_DAY_NS: i64 = 1_792_540_800_000_000_000; // 2026-10-21 00:00:00 UTC

fn profile(id: &str) -> Value {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    oracle["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["profile_id"] == id)
        .unwrap()
        .clone()
}
fn chongs(p: &Value) -> &[Value] {
    p["chongs"].as_array().unwrap()
}
fn level(c: &Value) -> &str {
    c["level"].as_str().unwrap().trim_start_matches('L')
}
fn tag(id: &str) -> &'static str {
    ["p0", "p1", "p5m"][PROFILES.iter().position(|p| *p == id).unwrap()]
}
fn db_name(owner: usize) -> String {
    if owner == 0 {
        "e.db".into()
    } else {
        format!("b{}.db", owner - 1)
    }
}

/// C8 `/2` manifest（资金三件与 `tb03c_funding_tests` 同源构造），时钟形状同 S3：每条消息每个 epoch 一列。
fn manifest(dir: &Path, id: &str, fault: Option<(&str, &str)>) -> Value {
    let p = profile(id);
    let routes = chongs(&p)
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let l = level(c);
            json!({"chong_id":chong_id(l),"symbol":"BTC","operation_level":l,"fixed_q":"1",
                "db":dir.join(db_name(i + 1)),"socket":dir.join(format!("b{i}.sock")),
                "funding":{"Q":c["Q"].as_i64().unwrap().to_string(),"policy_record":policy_record(id, l)}})
        })
        .collect::<Vec<_>>();
    let mut plan = serde_json::Map::new();
    // #1621：资金时效取 b_observation.first_known_ns；夹具须落在签署有效期内，不能用 1970 附近的纳秒值。
    for i in 0..MESSAGES as i64 {
        let t = VALID_DAY_NS + 1000 + 10 * i;
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
    let mut m = json!({"schema_revision":"economic-manifest/1","session_id":SESSION,"session_generation":"1","book_revision":"tb03c-book/2",
        "funding":{"funding_record":funding_record(id, p["q_total"].as_i64().unwrap()),
            "margin_leverage":{"value":p["margin_leverage"].as_i64().unwrap().to_string(),"source":"ADR 0040 裁定三（#842 R-10；γ = 1.0 先例）"}},
        "e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},"chongs":routes,
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1674-v2","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":(VALID_DAY_NS + 1).to_string(),"recover_ns":((1..=16).map(|r| (VALID_DAY_NS + r * 1_000_000).to_string()).collect::<Vec<_>>()),"messages":plan}}});
    if let Some((message, phase)) = fault {
        m["test_only_fault"] =
            json!({"message_id":message,"phase":phase,"marker_path":dir.join("fault-marker.json")});
    }
    m
}
/// 域配置按 manifest 的重序：0 = E，i+1 = 第 i 重。
fn load(dir: &Path) -> Vec<store::Config> {
    let path = dir.join("manifest.json");
    let m = strict_json(&std::fs::read(&path).unwrap()).unwrap();
    let mut ids = vec![None];
    ids.extend(
        m["chongs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| Some(c["chong_id"].as_str().unwrap().to_owned())),
    );
    ids.iter()
        .enumerate()
        .map(|(owner, id)| {
            store::Config::load(&path, &dir.join(db_name(owner)), id.as_deref()).unwrap()
        })
        .collect()
}
fn setup(dir: &Path, id: &str, fault: Option<(&str, &str)>) -> Vec<store::Config> {
    std::fs::write(
        dir.join("manifest.json"),
        canonical_json(&manifest(dir, id, fault)),
    )
    .unwrap();
    let configs = load(dir);
    for c in &configs {
        assert_eq!(store::init(c).unwrap()["kind"], "Initialized");
    }
    configs
}
fn envelope(id: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":SESSION,"session_generation":"1","message_id":id,"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1674-v2","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}
fn capture_payload(raw: &str, chong: &str, k: usize) -> Value {
    let doc: Value = serde_json::from_str(raw).unwrap();
    json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":format!("/events/{k}"),"token":format!("tok-{chong}-{k}"),"source_event_time":doc["events"][k]["source"]["source_event_time"]})
}
fn apply_payload(r: &Value) -> Value {
    json!({"op":"ApplyAllocation","fact_key":r["fact_key"],"allocation_id":r["allocation_id"],"expected_fact_hash":r["fact_hash"]})
}

/// 公共只读查询：各域 ReadView，加每个事实 QueryFact，B 另加每个 allocation QueryAllocation。
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

/// 终态对 C8 签署黄金值（FUNDING-ORACLE.json）逐值：各域 meta 为 `/2`；每重 B 书账的本金 = 开局批 = ⌈Q/L⌉、
/// 资金出处逐字段；E 侧未签政策只留 typed `AwaitingPolicy{funding_revision|q_rebase}`，不填任何默认值。
fn golden(id: &str, tables: &[Value]) {
    let p = profile(id);
    assert_eq!(tables.len(), chongs(&p).len() + 1);
    for t in tables {
        let rows = t["meta"].as_array().unwrap();
        let revision = rows.iter().find(|r| r["k"] == "book_revision").unwrap();
        assert_eq!(revision["v"], "tb03c-book/2");
    }
    let image =
        |d: usize| parsed(&tables[d]["commits"].as_array().unwrap().last().unwrap()["image"]);
    let mut waits = vec![];
    for (i, c) in chongs(&p).iter().enumerate() {
        let l = level(c);
        let amount = c["p0_batch"].as_i64().unwrap().to_string();
        let b = &image(i + 1)["book"];
        assert_eq!(b["book_revision"], "tb03c-book/2");
        assert_eq!(b["principal"]["P"], amount);
        assert_eq!(b["principal"]["status"], "Known");
        assert_eq!(
            b["funding"],
            json!({"authorization_ref":format!("#1557/funding-authorization/v1@{id}"),"funding_version":"v1",
                "Q":c["Q"].as_i64().unwrap().to_string(),"margin_leverage":p["margin_leverage"].as_i64().unwrap().to_string(),
                "opening_batch":amount,
                "policy_record":{"policy_id":"#1355/quantity-tier-policy/v1","version":"v1","q_ref":format!("#1557/funding-authorization/v1@{id}")},
                "capacity":{"level":format!("L{l}"),"C":c["C"].as_i64().unwrap().to_string()}}),
            "{id} L{l}"
        );
        for slot in ["funding_revision", "q_rebase"] {
            waits.push(
                json!({"kind":"AwaitingPolicy","slot":slot,"symbol":"BTC","level":format!("L{l}")}),
            );
        }
    }
    assert_eq!(awaiting(&tables[0]), waits, "{id} E 侧 typed 等待");
}
/// E 域最新提交图像里 Pending 的 typed `AwaitingPolicy` 理由（按 Pending 原序）。
fn awaiting(e: &Value) -> Vec<Value> {
    parsed(&e["commits"].as_array().unwrap().last().unwrap()["image"])["pending"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["reason"].clone())
        .filter(|r| r["kind"] == "AwaitingPolicy")
        .collect()
}

/// 子进程入口。只由下方 SIGKILL 调度以本测试建立的 job 文件调用。
/// 步可带 `epoch` 覆盖本域写者 epoch（用于以旧写者身份重送）。
#[test]
#[ignore = "仅供 tb03c_v2_kill 子进程故障调度调用，不作独立空域通过项"]
fn tb03c_v2_kill_child() {
    let job_path =
        PathBuf::from(std::env::var_os("TB03C_V2_KILL_JOB").expect("child job required"));
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
        let epoch = step
            .get("epoch")
            .map_or(epochs[owner], |e| e.as_i64().unwrap());
        let reply = service::handle(
            &mut dbs[owner],
            &configs[owner],
            epoch,
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
        .env("TB03C_V2_KILL_JOB", &path)
        .stdout(std::fs::File::create(work.join(format!("{label}.stdout"))).unwrap())
        .stderr(std::fs::File::create(work.join(format!("{label}.stderr"))).unwrap())
        .spawn()
        .unwrap()
}
/// 新进程跑 `steps`，返回逐步进度行（须全部返回）。
fn run(work: &Path, label: &str, epochs: &[i64], steps: &[Value]) -> Vec<Value> {
    finish(
        spawn(
            work,
            label,
            json!({"action":"run","epochs":epochs,"steps":steps}),
        ),
        work,
        label,
    );
    let lines = progress(work, label);
    assert_eq!(lines.len(), steps.len());
    lines
}
fn root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        // 同一外部根下重跑或 PID 复用时也必须从空现场开始；保留旧证据，不删除共享目录。
        let run_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::var_os("TB03C_KILL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("nc1674-v2-kill-{}-{run_ns}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    })
}

/// 不崩溃计划：进程内逐步生成同一组信封；每重依次 开局 Capture/Apply、追加批 Capture/Apply、q_rebase Capture/Apply。
/// 追加批与 q_rebase 的 Apply 是 typed `AwaitingPolicy` 拒绝，不改任何域，留在续跑里核「未签政策恢复后仍等待」。
struct Plan {
    id: &'static str,
    steps: Vec<Value>,
    replies: Vec<Value>,
    /// snapshots[i]：第 i 步之前各域业务表；末项为终态。
    snapshots: Vec<Vec<Value>>,
    /// 逐步回复按参照时钟规范化（只算一次）。
    norm_replies: Vec<Value>,
}
impl Plan {
    fn owners(&self) -> Vec<usize> {
        self.steps
            .iter()
            .map(|s| s["owner"].as_u64().unwrap() as usize)
            .collect()
    }
    fn domains(&self) -> usize {
        self.snapshots[0].len()
    }
}
struct Builder {
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    steps: Vec<Value>,
    replies: Vec<Value>,
    snapshots: Vec<Vec<Value>>,
}
impl Builder {
    fn send(&mut self, owner: usize, payload: Value, expect: &str) -> Value {
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
        let after: Vec<Value> = dumps(&self.configs).iter().map(business).collect();
        if expect == "AwaitingPolicy" {
            assert_eq!(r["kind"], "Error", "{r}");
            assert!(
                r["error"].as_str().unwrap().starts_with("AwaitingPolicy："),
                "{r}"
            );
            assert_eq!(
                &after,
                self.snapshots.last().unwrap(),
                "未签政策的拒绝不得改任何域"
            );
        } else {
            assert_eq!(r["kind"], expect, "{r}");
        }
        self.steps.push(json!({"owner":owner,"envelope":e}));
        self.replies.push(r.clone());
        self.snapshots.push(after);
        r
    }
}
fn build(id: &'static str) -> Plan {
    let t0 = Instant::now();
    let space = Space::new("v2-kill-plan");
    let configs = setup(&space.0, id, None);
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
    let p = profile(id);
    for (i, c) in chongs(&p).iter().enumerate() {
        let l = level(c);
        let chong = chong_id(l);
        let raw = raw(l, &Batch::ok(c["p0_batch"].as_i64().unwrap(), id));
        for k in 0..3 {
            let r = b.send(0, capture_payload(&raw, &chong, k), "Recorded");
            // 目标步口径：追加批 / q_rebase 的 Capture 恰在 E Pending 末尾新增对应 slot 的 typed 等待。
            let waits = awaiting(&b.snapshots.last().unwrap()[0]);
            assert_eq!(
                waits.len(),
                2 * i + k,
                "{id} 第 {i} 重 /events/{k} 后 E 等待数"
            );
            if k > 0 {
                let slot = ["funding_revision", "q_rebase"][k - 1];
                assert_eq!(
                    waits.last(),
                    Some(
                        &json!({"kind":"AwaitingPolicy","slot":slot,"symbol":"BTC","level":format!("L{l}")})
                    ),
                    "{id} 第 {i} 重 /events/{k}"
                );
            }
            b.send(
                i + 1,
                apply_payload(&r),
                if k == 0 { "Applied" } else { "AwaitingPolicy" },
            );
        }
    }
    assert_eq!(b.steps.len(), 6 * chongs(&p).len());
    assert!(b.steps.len() <= MESSAGES);
    golden(id, &dumps(&b.configs));
    let mut plan = Plan {
        id,
        steps: b.steps,
        replies: b.replies,
        snapshots: b.snapshots,
        norm_replies: vec![],
    };
    let clock = Clock::of_at(plan.owners(), None, VALID_DAY_NS);
    plan.norm_replies = plan
        .replies
        .iter()
        .map(|r| normalize_reference(r, &clock))
        .collect();
    println!(
        "V2-TIME {}",
        canonical_json(&json!({"profile":id,"plan_ms":t0.elapsed().as_millis()}))
    );
    plan
}
fn plan(id: &str) -> &'static Plan {
    static PLANS: [OnceLock<Plan>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = PROFILES.iter().position(|p| *p == id).unwrap();
    PLANS[i].get_or_init(|| build(PROFILES[i]))
}

/// 不崩溃参照：同输入两次新进程从空库完整重放（同路径），终态全表逐字节、哈希相同。
struct Reference {
    tables: Vec<Value>,
    public: Vec<Value>,
    /// 按参照时钟规范化（只算一次）：终态全表（除 meta/epochs）、公开查询。
    norm_tables: Vec<Value>,
    norm_public: Vec<Value>,
}
fn build_reference(plan: &Plan) -> Reference {
    let t0 = Instant::now();
    let base = root().join(tag(plan.id)).join("reference");
    let work = base.join("work");
    let mut finals: Vec<(Vec<Value>, Vec<Value>, String)> = vec![];
    for rep in 0..2 {
        std::fs::create_dir_all(&work).unwrap();
        let configs = setup(&work, plan.id, None);
        let epochs = vec![1; plan.domains()];
        let lines = run(&work, "run", &epochs, &plan.steps);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line["reply"], plan.replies[i], "参照新进程第 {i} 步");
        }
        let tables = dumps(&configs);
        for (d, t) in tables.iter().enumerate() {
            assert_eq!(business(t), plan.snapshots.last().unwrap()[d]);
        }
        audit_all(&configs);
        golden(plan.id, &tables);
        let queries = public(&configs);
        let digest = book::hash(&json!(tables));
        println!(
            "V2-REPLAY {}",
            canonical_json(
                &json!({"profile":plan.id,"rep":rep,"final_tables_sha256":digest,"raw_dir":base.join(format!("rep-{rep}"))})
            )
        );
        std::fs::rename(&work, base.join(format!("rep-{rep}"))).unwrap();
        finals.push((tables, queries, digest));
    }
    assert_eq!(
        canonical_json(&json!(finals[0].0)),
        canonical_json(&json!(finals[1].0)),
        "{} 无故障同输入两次新进程完整终态逐字节相同",
        plan.id
    );
    assert_eq!(finals[0].2, finals[1].2, "{} 两次终态哈希相同", plan.id);
    let (tables, public, _) = finals.swap_remove(0);
    let clock = Clock::of_at(plan.owners(), None, VALID_DAY_NS);
    let norm_tables = tables
        .iter()
        .map(|t| normalize_reference(&without(t, &["meta", "epochs"]), &clock))
        .collect();
    let norm_public = public
        .iter()
        .map(|q| normalize_reference(q, &clock))
        .collect();
    println!(
        "V2-TIME {}",
        canonical_json(&json!({"profile":plan.id,"reference_ms":t0.elapsed().as_millis()}))
    );
    Reference {
        tables,
        public,
        norm_tables,
        norm_public,
    }
}
fn reference(id: &str) -> &'static Reference {
    static REFERENCES: [OnceLock<Reference>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = PROFILES.iter().position(|p| *p == id).unwrap();
    REFERENCES[i].get_or_init(|| build_reference(plan(id)))
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

/// 新进程 P 从空库跑完目标前缀 → 新进程 A 只跑目标消息并停在持久边界 → SIGKILL → 边界断言 → 新进程 recover。
fn kill_and_recover(plan: &Plan, target: usize, phase: &str, work: &Path) -> Killed {
    let committed = phase == "after_commit";
    let owner = plan.steps[target]["owner"].as_u64().unwrap() as usize;
    let n = plan.domains();
    // 目标步确实改变被杀域，「提交前=前快照 / 提交后=后快照」两判据不重合。
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
    let configs = setup(work, plan.id, Some((&message, phase)));
    let epochs = vec![1; n];
    if target > 0 {
        let lines = run(work, "p", &epochs, &plan.steps[..target]);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line["reply"], plan.replies[i], "前缀新进程第 {i} 步");
        }
    }
    let before = dumps(&configs);
    for d in 0..n {
        assert_eq!(
            business(&before[d]),
            plan.snapshots[target][d],
            "前缀后域 {d}"
        );
    }
    let mut a = spawn(
        work,
        "a",
        json!({"action":"run","epochs":epochs,"steps":plan.steps[target..=target]}),
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
        json!({"domain_id":configs[owner].domain,"message_id":message,"phase":phase,"pid":pid,"test_only":true})
    );
    a.kill().unwrap();
    let killed = a.wait().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(killed.signal(), Some(9));
    let wal_bytes = std::fs::metadata(format!("{}-wal", configs[owner].db.display()))
        .map(|m| m.len())
        .ok();

    assert!(progress(work, "a").is_empty(), "目标消息不得有回复");
    // 持久边界：提交前被杀 = 目标前快照（事务内已写的全部行均未落盘）；提交后被杀 = 目标后快照。
    let boundary = if committed { target + 1 } else { target };
    let at_kill = dumps(&configs);
    for d in 0..n {
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
        json!([
            {"epoch":1,"phase":"init","at_ns":(VALID_DAY_NS + 1).to_string()},
            {"epoch":2,"phase":"recover","at_ns":(VALID_DAY_NS + 1_000_000).to_string()}
        ])
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

/// 终态各域全表（除 meta/epochs）与公开查询对不崩溃参照：规范化逐值比较（返回第一处差异）。
fn final_diff(plan: &Plan, k: &Killed, clock: &Clock) -> Result<(Vec<Value>, Vec<Value>), String> {
    let reference = reference(plan.id);
    let finals = dumps(&k.configs);
    for (d, t) in finals.iter().enumerate() {
        compare(
            &without(t, &["meta", "epochs"]),
            clock,
            &reference.norm_tables[d],
        )
        .map_err(|e| format!("终态域 {d}：{e}"))?;
    }
    let queries = public(&k.configs);
    for (d, q) in queries.iter().enumerate() {
        compare(q, clock, &reference.norm_public[d]).map_err(|e| format!("公开查询域 {d}：{e}"))?;
    }
    Ok((finals, queries))
}

/// 一次完整轨：kill → recover → 旧写者重送（StaleWriter）→ 新写者原样重送两次 → 续跑到终态。
fn run_once(plan: &Plan, target: usize, phase: &str, work: &Path) -> Value {
    let committed = phase == "after_commit";
    let t0 = Instant::now();
    let k = kill_and_recover(plan, target, phase, work);
    let t_kill = t0.elapsed().as_millis();
    let (owner, kill_seq) = (k.owner, k.kill_seq);
    let clock = Clock::of_at(
        plan.owners(),
        Some((owner, if committed { target + 1 } else { target })),
        VALID_DAY_NS,
    );
    let mut stale = plan.steps[target].clone();
    stale["epoch"] = json!(1);
    let mut steps = vec![
        stale,
        plan.steps[target].clone(),
        plan.steps[target].clone(),
    ];
    steps.extend_from_slice(&plan.steps[target + 1..]);
    // #1621：E 恢复后，后续 B 也须由新进程 recover 并升 epoch 2；
    // 否则 B 的首次获知早于 E 的重送提交，违反 E→B 时钟因果锁。
    if owner == 0 {
        for b_owner in 1..k.configs.len() {
            let label = format!("recover-b{b_owner}");
            finish(
                spawn(work, &label, json!({"action":"recover","owner":b_owner})),
                work,
                &label,
            );
            let recovered = progress(work, &label)[0]["recovered"].clone();
            assert_eq!(recovered["kind"], "Recovered");
            assert_eq!(recovered["writer_epoch"], "2");
        }
    }
    let mut epochs = vec![1; plan.domains()];
    if owner == 0 {
        epochs.fill(2);
    } else {
        epochs[owner] = 2;
    }
    let after = run(work, "c", &epochs, &steps);
    let t_resume = t0.elapsed().as_millis();
    // 旧写者（epoch 1）重送：被 recover 封住，StaleWriter，不新增提交。
    let stale = &after[0];
    assert_eq!(stale["reply"]["kind"], "Error", "{stale}");
    assert!(
        stale["reply"]["error"]
            .as_str()
            .unwrap()
            .starts_with("StaleWriter："),
        "{stale}"
    );
    assert_eq!(stale["seq"], json!(kill_seq));
    let resent = &after[1];
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
    assert_eq!(after[2]["reply"], resent["reply"], "再次重送同回执");
    assert_eq!(after[2]["seq"], resent["seq"], "再次重送无新提交");
    for (n, line) in after[3..].iter().enumerate() {
        let i = target + 1 + n;
        assert_eq!(
            compare(&line["reply"], &clock, &plan.norm_replies[i]),
            Ok(()),
            "续跑第 {i} 步"
        );
    }
    audit_all(&k.configs);
    let (finals, queries) = final_diff(plan, &k, &clock).unwrap();
    let reference = reference(plan.id);
    for d in 0..plan.domains() {
        if owner != 0 && d != owner {
            // B 崩溃不触及 E 与其它 B：业务表与公开查询逐字相同。
            assert_eq!(business(&finals[d]), business(&reference.tables[d]));
            assert_eq!(queries[d], reference.public[d]);
        }
    }
    golden(plan.id, &finals);
    println!(
        "V2-TIME {}",
        canonical_json(
            &json!({"profile":plan.id,"target":target,"phase":phase,"kill_and_recover_ms":t_kill,"resume_done_ms":t_resume,"total_ms":t0.elapsed().as_millis()})
        )
    );
    let mut recovered_record = k.recovered.clone();
    recovered_record.as_object_mut().unwrap().remove("runtime");
    json!({"scope":"local transaction process fault seam (Config::pause around tx.commit); not formal sockets/S/browser",
        "book_revision":"tb03c-book/2","profile":plan.id,
        "message_id":k.message,"op":plan.steps[target]["envelope"]["payload"]["op"],"domain_id":k.configs[owner].domain,"phase":phase,
        "pid":k.pid,"signal":k.signal,"marker":k.marker,"wal_bytes_at_kill":k.wal_bytes,"kill_seq":kill_seq,
        "original_receipt":k.original,"recovered":recovered_record,"stale_resend":stale,"resend":resent,
        "continuation_replies":after.iter().map(|l| l["reply"].clone()).collect::<Vec<_>>(),
        "final_tables_sha256":book::hash(&json!(finals)),"final_tables":finals,"public":queries})
}
/// 一个（profile, 目标步, 阶段）组合：两次全新目录、全新 P/A/R/C 进程，终态全表与续跑回复逐字节相同。
fn matrix(id: &str, target: usize, op: &str, owner: usize, phase: &str) {
    let plan = plan(id);
    assert_eq!(plan.steps[target]["envelope"]["payload"]["op"], op);
    assert_eq!(plan.steps[target]["owner"], owner);
    let label = format!("step{target:02}");
    let base = root().join(tag(id)).join(&label).join(phase);
    let work = base.join("work");
    let mut finals = vec![];
    for rep in 0..2 {
        let evidence = run_once(plan, target, phase, &work);
        let archived = base.join(format!("rep-{rep}"));
        std::fs::rename(&work, &archived).unwrap();
        save(&archived.join("evidence.json"), &evidence);
        println!(
            "V2-KILL {}",
            canonical_json(
                &json!({"profile":id,"label":label,"op":op,"phase":phase,"rep":rep,"message_id":evidence["message_id"],"domain_id":evidence["domain_id"],"pid":evidence["pid"],"signal":evidence["signal"],"kill_seq":evidence["kill_seq"],"stale_kind":evidence["stale_resend"]["reply"]["kind"],"resend_kind":evidence["resend"]["reply"]["kind"],"resend_seq":evidence["resend"]["seq"],"final_tables_sha256":evidence["final_tables_sha256"],"raw_dir":archived})
            )
        );
        finals.push((
            canonical_json(&evidence["final_tables"]),
            evidence["continuation_replies"].clone(),
        ));
    }
    assert_eq!(
        finals[0].0, finals[1].0,
        "{id} {label}/{phase} 两次新进程完整终态逐字节相同"
    );
    assert_eq!(
        finals[0].1, finals[1].1,
        "{id} {label}/{phase} 两次新进程续跑回复相同"
    );
}

/// 无故障：同输入两次新进程完整重放，终态逐字节与哈希相同，meta 为 `/2`、E/B audit 通过、对黄金值。
#[test]
fn tb03c_v2_kill_reference_replay_p0() {
    reference("P0");
}
#[test]
fn tb03c_v2_kill_reference_replay_p1() {
    reference("P1-hl-testnet");
}
#[test]
fn tb03c_v2_kill_reference_replay_p5m() {
    reference("P0-acceptance-5m");
}

/// 每个 profile 的目标步：开局 Capture、开局 Apply（`book.funding` 写入）、E Pending 为
/// `AwaitingPolicy{funding_revision}` 与 `{q_rebase}` 的两次 Capture；P0-acceptance-5m 另加末重（L2）开局 Apply。
/// 每步生成 `<名>::before_commit` 与 `<名>::after_commit`（13 步 × 2 阶段 × 2 次 = 52 次 SIGKILL）。
macro_rules! kill_step {
    ($($name:ident: $profile:expr, $target:expr, $op:expr, $owner:expr;)*) => {$(
        mod $name {
            #[test]
            fn before_commit() {
                super::matrix($profile, $target, $op, $owner, "before_commit");
            }
            #[test]
            fn after_commit() {
                super::matrix($profile, $target, $op, $owner, "after_commit");
            }
        }
    )*};
}
kill_step! {
    tb03c_v2_kill_p0_s00_capture_open: "P0", 0, "CaptureExternal", 0;
    tb03c_v2_kill_p0_s01_apply_open: "P0", 1, "ApplyAllocation", 1;
    tb03c_v2_kill_p0_s02_capture_funding_revision: "P0", 2, "CaptureExternal", 0;
    tb03c_v2_kill_p0_s04_capture_q_rebase: "P0", 4, "CaptureExternal", 0;
    tb03c_v2_kill_p1_s00_capture_open: "P1-hl-testnet", 0, "CaptureExternal", 0;
    tb03c_v2_kill_p1_s01_apply_open: "P1-hl-testnet", 1, "ApplyAllocation", 1;
    tb03c_v2_kill_p1_s02_capture_funding_revision: "P1-hl-testnet", 2, "CaptureExternal", 0;
    tb03c_v2_kill_p1_s04_capture_q_rebase: "P1-hl-testnet", 4, "CaptureExternal", 0;
    tb03c_v2_kill_p5m_s00_capture_open_l0: "P0-acceptance-5m", 0, "CaptureExternal", 0;
    tb03c_v2_kill_p5m_s01_apply_open_l0: "P0-acceptance-5m", 1, "ApplyAllocation", 1;
    tb03c_v2_kill_p5m_s02_capture_funding_revision_l0: "P0-acceptance-5m", 2, "CaptureExternal", 0;
    tb03c_v2_kill_p5m_s04_capture_q_rebase_l0: "P0-acceptance-5m", 4, "CaptureExternal", 0;
    tb03c_v2_kill_p5m_s13_apply_open_l2: "P0-acceptance-5m", 13, "ApplyAllocation", 3;
}
