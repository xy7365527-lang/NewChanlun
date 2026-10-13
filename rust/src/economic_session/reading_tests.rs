//! #1394 A6 第4、6节读法锁：一个冻结 finance.structure 只认一种读法，失败不换读法（#799）。
//! 伪 S 是记录 argv 的 shell 脚本，走生产同一条 `SourcePins::call` → `finance_transport::financial_reader`
//! 子进程路径：`financial-basis-ref` 回一份合法 basis 头（proof 只含头部路径节点，大字符串仍缺），
//! `financial-closure` 与 `financial-content` 都回 `{}`，让所选读法在缺节点处失败。
use super::*;
use crate::s_content::job::Existing;
use crate::s_content::MemoryNodes;
use crate::session_protocol::canonical_json;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn resources() -> Value {
    json!({"schema_revision":"s-financial-reader-resources/1",
        "max_request_bytes":"1048576","max_reply_bytes":"1048576",
        "max_stderr_bytes":"1048576","request_timeout_ms":"30000",
        "job_limits":{"max_total_wire_bytes":"100000000", "max_total_node_bytes":"100000000",
            "max_total_nodes":"1000000", "max_total_records":"1000000",
            "max_total_requests":"100", "max_elapsed_ms":"60000",
            "max_staged_bytes":"100000000", "max_store_bytes":"100000000"}})
}
fn structure(binary: &str, sha: &str, reader_session: Value) -> Value {
    json!({"binary":binary,"binary_sha256":sha,"db":"/frozen/db",
        "session_id":"s-session-testonly-001","session_generation":"1","writer_epoch":"1",
        "reader_revision":"s-financial-basis/3","content_format":"s-content/1",
        "reader_resources":resources(),"reader_session":reader_session})
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
    Instant::now() + Duration::from_secs(30)
}

/// 记读到的节点：用于把 `read_once` 在独立 MemoryNodes 上实际要的头部节点做成 proof。
struct Recorder<'a> {
    inner: &'a mut MemoryNodes,
    seen: BTreeSet<NodeDigest>,
}
impl NodeSource for Recorder<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let read = self.inner.read(digest)?;
        if matches!(read, NodeRead::Found { .. }) {
            self.seen.insert(*digest);
        }
        Ok(read)
    }
}
/// 合法 `financial-basis-ref` 回复：空选中对象；proof = `validate_headers` 与 anchor 两个标量实际读到的节点。
fn basis_reply() -> Value {
    let (mut store, root) = super::closure_tests::basis();
    let evidence = super::closure_tests::evidence(&root);
    let header_pins =
        SourcePins::from_structure(&structure("/frozen/s", &"0".repeat(64), Value::Null)).unwrap();
    let (snapshot, seen) = {
        let mut rec = Recorder {
            inner: &mut store,
            seen: BTreeSet::new(),
        };
        let (snapshot, fields) = validate_headers(&header_pins, &evidence, &mut rec).unwrap();
        for key in ["structure_cut", "index_frontier"] {
            scalar_string(&mut rec, &map_ref(&fields, key).unwrap()).unwrap();
        }
        (snapshot, rec.seen)
    };
    let proof: Vec<Value> = seen
        .iter()
        .map(|d| {
            let (tag, body) = store.get(d).unwrap();
            super::closure_tests::node_json(d, tag, body)
        })
        .collect();
    json!({"schema_revision":"s-financial-basis/3","content_format":"s-content/1",
        "kind":"FinancialBasisRef","session_id":"s-session-testonly-001","session_generation":"1",
        "writer_epoch":"1","pending_revision":false,"commit_ns":"77",
        "basis_ref":root.to_transport_json(),"snapshot_ref":snapshot.to_transport_json(),
        "selected_objects":[],"proof_nodes":proof})
}

/// 伪 S：脚本把每次 argv 追加到 `argv.log`；目录 0700，会话 socket 路径默认放在它下面。
struct FakeS {
    dir: PathBuf,
    binary: String,
    sha: String,
}
impl FakeS {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "nc1394-rd-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let reply = dir.join("basis.json");
        std::fs::write(&reply, canonical_json(&basis_reply())).unwrap();
        let script = dir.join("s.sh");
        let text = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\n/bin/cat >/dev/null\ncase \"$1\" in\n  financial-basis-ref) exec /bin/cat '{reply}' ;;\n  *) printf '{{}}' ;;\nesac\n",
            log = dir.join("argv.log").display(),
            reply = reply.display()
        );
        std::fs::write(&script, &text).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let sha = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            binary: script.display().to_string(),
            dir,
            sha,
        }
    }
    fn socket(&self) -> String {
        self.dir.join("s.sock").display().to_string()
    }
    fn pins(&self, reader_session: Value) -> SourcePins {
        SourcePins::from_structure(&structure(&self.binary, &self.sha, reader_session)).unwrap()
    }
    /// 每次 S 进程的 argv，按发出顺序。
    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("argv.log"))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }
    fn commands(&self) -> Vec<String> {
        self.calls()
            .iter()
            .map(|c| c.split(' ').next().unwrap().to_owned())
            .collect()
    }
}
impl Drop for FakeS {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Default)]
struct Stage(MemoryNodes);
impl NodeSource for Stage {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.0.read(digest)
    }
}
impl FinancialStage for Stage {
    fn put_budgeted(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), CodecError> {
        use crate::s_content::NodeSink;
        let allocated = u64::try_from(body.len()).unwrap() + 128;
        let grant = job.reserve_staged(allocated)?;
        self.0.put(digest, tag, body)?;
        job.reconcile(&grant, allocated)
    }
    fn put_batch_budgeted(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
    ) -> Result<(), CodecError> {
        super::test_put_batch(self, job, nodes, inserted)
    }
    fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError> {
        Ok(super::test_discard(&mut self.0, digests))
    }
}

/// 锁 ①④（A6 第4、6节）：每种读法恰好只认一条缺节点命令；另一条在起 S 进程、计请求之前即被拒。
/// 会话读法的 `financial-closure` 必带 `--session`；冷读法调 `financial-closure`（冷 6 参对照臂）报错。
#[test]
fn each_reading_admits_exactly_one_missing_node_command() {
    let socket = "/tmp/s1394/s.sock";
    for (session, allowed, refused) in [
        (Value::Null, "financial-content", "financial-closure"),
        (json!(socket), "financial-closure", "financial-content"),
    ] {
        let pins =
            SourcePins::from_structure(&structure("/frozen/s", &"0".repeat(64), session)).unwrap();
        let mut job = job_for(&pins);
        let error = pins
            .call(refused, &json!({}), &mut job, deadline())
            .unwrap_err();
        assert!(
            error.starts_with("UnsupportedCapability：") && error.contains(refused),
            "{error}"
        );
        assert_eq!(job.snapshot().requests, 0, "被拒命令不起 S 进程、不计请求");
        // 本读法的命令照常走到传输层（/frozen/s 不存在 → 无法启动），证明门只挡另一读法的命令。
        let error = pins
            .call(allowed, &json!({}), &mut job, deadline())
            .unwrap_err();
        assert!(error.contains("无法启动"), "{error}");
        assert_eq!(job.snapshot().requests, 1);
    }
    let warm = SourcePins::from_structure(&structure("/frozen/s", &"0".repeat(64), json!(socket)))
        .unwrap();
    let args = warm.reader_args("financial-closure");
    assert_eq!(&args[args.len() - 2..], ["--session", socket]);
}

/// 锁 ②（A6 第4节）：会话读法下闭包导入失败即返回该失败，不发出任何 `financial-content`。
#[test]
fn session_closure_failure_never_requests_financial_content() {
    let s = FakeS::new();
    let socket = s.socket();
    let pins = s.pins(json!(socket));
    let mut job = job_for(&pins);
    let mut stage = Stage::default();
    let error = read_current_pins(&pins, &[], &mut job, deadline(), &mut stage).unwrap_err();
    let expected = exact(&json!({}), &CLOSURE_REPLY).unwrap_err();
    assert!(error.starts_with(&expected), "{error}");
    assert_eq!(s.commands(), ["financial-basis-ref", "financial-closure"]);
    let tail = format!("--session {socket}");
    assert!(
        s.calls().iter().all(|c| c.ends_with(&tail)),
        "{:?}",
        s.calls()
    );
    assert_eq!(
        stage.0.iter().count(),
        0,
        "会话读法不单独暂存 proof，闭包尝试已回滚"
    );
}

/// 锁 ③（A6 第4、6节）：冷读法下逐片读取失败即返回该失败，从不发出 `financial-closure`，也不带 `--session`。
#[test]
fn cold_fragment_failure_never_requests_financial_closure() {
    let s = FakeS::new();
    let pins = s.pins(Value::Null);
    let mut job = job_for(&pins);
    let mut stage = Stage::default();
    let error = read_current_pins(&pins, &[], &mut job, deadline(), &mut stage).unwrap_err();
    assert!(
        error.starts_with("SchemaUnsupported：字段集不为 exact"),
        "{error}"
    );
    assert!(!error.contains("闭包"), "{error}");
    assert_eq!(s.commands(), ["financial-basis-ref", "financial-content"]);
    assert!(s.calls().iter().all(|c| !c.contains("--session")));
}

/// A6 2.4：会话 socket 的父目录须为本用户所有且无组/其他权限位。冻结值只核语法（离线审计不依赖
/// 会话目录），目录在读时核，且在发出任何 S 读取之前。
#[test]
fn session_directory_is_checked_before_any_s_read() {
    let s = FakeS::new();
    let open = s.dir.join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
    let absent = s.dir.join("absent");
    for parent in [open, absent] {
        let pins = s.pins(json!(parent.join("s.sock").display().to_string()));
        let mut job = job_for(&pins);
        let mut stage = Stage::default();
        let error = read_current_pins(&pins, &[], &mut job, deadline(), &mut stage).unwrap_err();
        assert!(error.contains("reader_session 父目录"), "{error}");
        assert!(s.calls().is_empty(), "{:?}", s.calls());
    }
    // 同一冻结值的离线入口不碰会话目录：from_structure 照常成立。
    assert!(SourcePins::from_structure(&structure(
        &s.binary,
        &s.sha,
        json!(s.dir.join("absent/s.sock").display().to_string())
    ))
    .is_ok());
}
