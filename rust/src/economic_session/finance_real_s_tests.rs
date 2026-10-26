//! #1716 I1-C：`tb03b-finance/4` 内容域上签署 cap 激活被真实 S 读取链实际消费的局部证据。S 由 release
//! `s_structure_session` 的只读 `financial-serve` 会话回答，会话与读取 helper 沿用 #1717 PR #1736
//! （ca251c98）已过独审的单 bar `SessionClosure` 正控设计。E/B 仍是 #1394 TestOnly 信任根与 TestOnly
//! 合成签署 cap 原件；本测试不替代 #1552 统一 source、Q 与浏览器整链，也不启用生产金额。
use super::*;
use crate::economic_session::content_store::{allocated, carrier_capacity, image_refs};
use crate::economic_session::phase_trace;
use std::time::Instant;

fn env_path(key: &str) -> PathBuf {
    PathBuf::from(std::env::var(key).unwrap_or_else(|_| panic!("{key} 未设置")))
}

/// S 侧单 bar 真实 S3 夹具导出的库上由测试起停的 `financial-serve` 只读会话（A6 修订一）。会话目录 0700，
/// stdout/stderr 落在导出目录，失败时保留原件。
struct SSession {
    child: std::process::Child,
    socket: PathBuf,
    receipts: PathBuf,
    ready: Value,
}
impl SSession {
    fn start(binary: &Path, export: &Value, work: &Path) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let dir = PathBuf::from(format!("/tmp/tb1716-s-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("s.sock");
        let receipts = work.join("s-session-receipts");
        std::fs::create_dir(&receipts).unwrap();
        let stdout = work.join("s-session.stdout");
        let child = std::process::Command::new(binary)
            .args([
                "financial-serve",
                "--protocol-revision",
                "s-session/3",
                "--db",
            ])
            .arg(required(export, "db").unwrap())
            .arg("--socket")
            .arg(&socket)
            .arg("--resources")
            .arg(required(export, "session_resources").unwrap())
            .arg("--custody")
            .arg(required(export, "writer_custody").unwrap())
            .arg("--receipts")
            .arg(&receipts)
            .env_clear()
            .stdout(std::fs::File::create(&stdout).unwrap())
            .stderr(std::fs::File::create(work.join("s-session.stderr")).unwrap())
            .spawn()
            .unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(300);
        let ready = loop {
            let text = std::fs::read_to_string(&stdout).unwrap_or_default();
            if let Some(line) = text
                .strip_prefix("READY ")
                .and_then(|t| t.strip_suffix('\n'))
            {
                break strict_json(line.as_bytes()).unwrap();
            }
            assert!(
                Instant::now() < until,
                "financial-serve 未在启动窗内 READY：{}",
                work.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        Self {
            child,
            socket,
            receipts,
            ready,
        }
    }
    /// 优雅停机；FINAL=PASS 表示会话期间 R1–R8 无失效，全部回复出自同一已审状态。
    fn stop(mut self, binary: &Path) -> Value {
        let out = std::process::Command::new(binary)
            .args([
                "financial-serve-stop",
                "--protocol-revision",
                "s-session/3",
                "--session",
            ])
            .arg(&self.socket)
            .env_clear()
            .output()
            .unwrap();
        let status = self.child.wait().unwrap();
        let fin = strict_json(&std::fs::read(self.receipts.join("FINAL.json")).unwrap()).unwrap();
        assert_eq!(
            strict_json(&out.stdout).unwrap(),
            fin,
            "停机回复等于 FINAL.json"
        );
        assert!(status.success(), "{fin}");
        assert_eq!(fin["status"], "PASS", "{fin}");
        fin
    }
}

/// 真实 S 来源：S 侧单 bar 真实 S3 夹具（init_v3 + 真实接纳/Begin/提交）导出的库，经运行中的会话以生产
/// `financial-basis-ref` 取一个当前对象的全 wire 摘要。生产 `advance` 写者在未签 venue/cost 政策下拒绝打开
/// （`driver.rs` 的 `production_open_rejects_unsigned_profile_before_pending_begin`），故会话由夹具建。
/// 返回 `content_with_real_s` 的 `{structure, s_ref}`，`structure.reader_session` 为会话 socket。
fn real_s_from_export(binary: &Path, export: &Value, socket: &Path) -> Value {
    let db = required(export, "db").unwrap().to_owned();
    for id in export["object_ids"].as_array().unwrap() {
        let Some(reply) = basis_ref_reply(binary, export, socket, id) else {
            continue;
        };
        let selected = &reply["selected_objects"][0];
        let sha = sha256_hex(&std::fs::read(binary).unwrap());
        return json!({"structure":{"binary":binary,"binary_sha256":sha,"db":db,"session_id":export["session_id"],"session_generation":export["session_generation"],"writer_epoch":export["writer_epoch"],"reader_session":socket},
            "s_ref":{"schema_revision":"s-object-ref/2","object_id":id,"object_hash":selected["object_hash"],"profile_id":export["profile_id"]},
            "export":export,"basis_ref":reply["basis_ref"],"s_commit_ns":reply["commit_ns"]});
    }
    panic!("生产 S 会话读不到导出库的任何当前对象：{export}");
}

/// 经运行中的会话以生产 `financial-basis-ref` 读一个对象；读不到返回 None。
fn basis_ref_reply(binary: &Path, export: &Value, socket: &Path, id: &Value) -> Option<Value> {
    let db = required(export, "db").unwrap().to_owned();
    let socket = socket.to_str().unwrap();
    let request = json!({"schema_revision":"s-financial-query/1","session_id":export["session_id"],"session_generation":export["session_generation"],"writer_epoch":export["writer_epoch"],"object_ids":[id]});
    let mut child = std::process::Command::new(binary)
        .args([
            "financial-basis-ref",
            "--protocol-revision",
            "s-session/3",
            "--db",
            db.as_str(),
            "--session",
            socket,
        ])
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(canonical_json(&request).as_bytes())
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    if !out.status.success() {
        return None;
    }
    let reply = strict_json(&out.stdout).unwrap();
    assert_eq!(&reply["selected_objects"][0]["object_id"], id);
    Some(reply)
}

/// 本 owner 内容存储的不同根节点数与 §5.6.1 分帧字节（len(D)+1+8+len(body)，D 为 32 字节摘要）。
fn content_footprint(c: &rusqlite::Connection) -> (i64, i64) {
    c.query_row(
        "SELECT count(*), COALESCE(sum(41+length(body)),0) FROM content_nodes",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

/// 一个 /4 夹具：两 B 经正式入口 Prepare（经会话读真实 S 并发布闭包），E 在 Prepare 与 Reserve 之间激活
/// 版本 2 的 cap，随后 E Reserve 读 S、导入两份 peer 证明闭包并按已激活版本判定。创世 cap 为 100（够全部
/// 责任 90），所以结果只取决于版本 2：等于 90 时全量授信，89 时按原合同整槽等待、零授信、责任不增。
fn reserve_under_activated_cap(carriers: &[PathBuf], real_s: &Value, cap: &str) -> Value {
    let mut f = Fixture::content_with_real_s(carriers, "100", "20", real_s, None);
    let config = f.configs[0].clone();
    assert_eq!(config.revision().unwrap(), SCHEMA_V2);
    assert_eq!(revision(&config.manifest).unwrap(), REV_V4);
    let structure = &config.manifest["finance"]["structure"];
    assert_eq!(
        structure["binary_sha256"],
        real_s["structure"]["binary_sha256"]
    );
    assert_eq!(
        structure["reader_session"],
        real_s["structure"]["reader_session"]
    );
    assert_eq!(config.manifest["resources"]["response_timeout_ms"], "3000");
    let limits = &structure["reader_resources"]["job_limits"];
    assert_eq!(limits["max_total_nodes"], "2614");
    assert_eq!(limits["max_total_node_bytes"], "192172");
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 20);

    let mut elapsed_ms = serde_json::Map::new();
    let mut phases = serde_json::Map::new();
    let mut prepared = vec![];
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        let p = f.request("PrepareDecision", name)["payload"].clone();
        let t = Instant::now();
        let r = f.handle(n, &format!("PrepareDecision-{name}"), p);
        elapsed_ms.insert(
            format!("PrepareDecision-{name}"),
            json!(t.elapsed().as_millis().to_string()),
        );
        phases.insert(format!("PrepareDecision-{name}"), phase_trace::take());
        assert_eq!(r["kind"], "PreparedReceipt", "{r}");
        prepared.push(r);
    }
    let b_before = [latest(&f.dbs[1]).unwrap(), latest(&f.dbs[2]).unwrap()];

    let payload = cap_payload(&f, "1", "2", cap, "100000");
    let activated = f.handle(0, "ActivateCapPolicy-pre", payload);
    phases.insert("ActivateCapPolicy-pre".into(), phase_trace::take());
    assert_eq!(
        activated["kind"], "CapPolicyActivatedReceipt",
        "{activated}"
    );
    let to = activated["record"]["to"].clone();
    assert_eq!(to["policy_version"], "2");
    assert_eq!(to["cap"], cap);
    assert_eq!(to["profile_id"], TEST_PROFILE);
    assert_eq!(activated["record"]["from"]["cap"], "100");
    assert_eq!(activated["record"]["liability_at_activation"], "20");

    let t = Instant::now();
    let reserved = reserve_now(&mut f, "Reserve-slot");
    elapsed_ms.insert(
        "Reserve-slot".into(),
        json!(t.elapsed().as_millis().to_string()),
    );
    phases.insert("Reserve-slot".into(), phase_trace::take());
    let timing =
        json!({"activated_cap":cap,"elapsed_ms":elapsed_ms,"phases":phases,"reserve":reserved});
    save(
        &f.dir,
        &format!("issue1716-reserve-phases-cap{cap}.json"),
        &timing,
    );
    println!("ISSUE1716_RESERVE_PHASES {}", canonical_json(&timing));
    assert_eq!(reserved["kind"], "ReserveReceipt", "{reserved}");
    let record = &reserved["record"];
    assert_eq!(record["cap_policy"], to, "Reserve 记录的是已激活的版本 2");
    assert_eq!(record["required_total"], "90");
    let image = latest(&f.dbs[0]).unwrap().3;
    assert_eq!(image["finance"]["schema_revision"], REV_V4);
    assert_eq!(image["finance"]["cap_policy"], to);
    if cap == "90" {
        assert_eq!(record["state"], "Reserved", "{reserved}");
        assert_eq!(record["retained_total"], "90");
        assert_eq!(record["grants"].as_array().unwrap().len(), 2);
        assert_eq!(responsibility_total(&image), 90);
    } else {
        assert_eq!(record["state"], "AwaitingAllocationPolicy", "{reserved}");
        assert_eq!(
            record["waiting_for"],
            f.data["policy"]["allocation_policy_id"]
        );
        assert_eq!(record["retained_total"], "20");
        assert_eq!(record["grants"], json!([]));
        assert_eq!(responsibility_total(&image), 20, "不足时不留部分责任");
    }
    assert_eq!(
        [latest(&f.dbs[1]).unwrap(), latest(&f.dbs[2]).unwrap()],
        b_before,
        "Reserve 只写 E，B 侧无任何授信或部分提交"
    );
    let refs = image_refs(&image).unwrap();
    assert!(!refs.is_empty(), "E 历史须持有 FinancialBasisEvidence 引用");
    let footprints: Vec<(i64, i64)> = f.dbs.iter().map(content_footprint).collect();
    assert!(
        footprints.iter().all(|(nodes, _)| *nodes > 0),
        "{footprints:?}"
    );
    for (nodes, bytes) in &footprints {
        assert!(*nodes <= 2614 && *bytes <= 192_172, "{footprints:?}");
    }
    assert_eq!(
        reserve_now(&mut f, "Reserve-slot"),
        reserved,
        "原请求重送只取原回执"
    );
    let mut dedup_controls = evidence_copy_controls(&f);
    dedup_controls["s_ref"] = s_ref_controls(&f, real_s);

    let db = &config.db;
    let (physical, capacity) = (allocated(db).unwrap(), carrier_capacity(db).unwrap());
    assert!(physical <= capacity, "{physical} > {capacity}");
    let reopened = Config::load(&f.dir.join("manifest.json"), db, None).unwrap();
    audit(&open(&reopened.db, false).unwrap().0, &reopened).unwrap();
    f.audit();
    let owners: Vec<Value> = ["E", "B:alpha", "B:beta"]
        .iter()
        .zip(&footprints)
        .map(|(o, (n, b))| json!({"owner":o,"content_nodes":n.to_string(),"content_node_bytes":b.to_string()}))
        .collect();
    let evidence = json!({"activated_cap":cap,"prepared":prepared,"activation":activated,"reserve":reserved,
        "elapsed_ms":elapsed_ms,"phases":phases,"dedup_controls":dedup_controls,"content":owners,"e_content_refs":refs.len().to_string(),
        "e_allocated_bytes":physical.to_string(),"e_carrier_capacity":capacity.to_string(),"e_image":image});
    save(&f.dir, "issue1716-content-domain-real-s.json", &evidence);
    evidence
}

/// 图像内容审计按值去重的负控：E（Reserve 后含多份同一证据）与 B:alpha 的已提交图像原样通过；只改其中
/// 一份的 basis_ref 或 session_generation，审计必须拒绝，即值不同的那一份照样被核。
fn evidence_copy_controls(f: &Fixture) -> Value {
    let audit = |n: usize, image: &Value| -> Result<(), String> {
        let mut job =
            crate::economic_session::store::new_content_job(&f.dbs[n], &f.configs[n]).unwrap();
        crate::economic_session::content_store::verify_image(
            &f.dbs[n],
            image,
            &f.configs[n].manifest["finance"]["structure"],
            &mut job,
        )
        .map_err(|e| e.to_string())
    };
    let e_image = latest(&f.dbs[0]).unwrap().3;
    let b_image = latest(&f.dbs[1]).unwrap().3;
    let last = e_image["history"].as_array().unwrap().len() - 1;
    assert_eq!(e_image["history"][last]["kind"], "Reserve");
    let e_copy = format!(
        "/history/{last}/witness/prepared_proofs/1/image/finance/decisions/0/structure_evidence"
    );
    let mut out = serde_json::Map::new();
    for (label, n, image, copy) in [
        ("E", 0, &e_image, e_copy.as_str()),
        (
            "B:alpha",
            1,
            &b_image,
            "/finance/decisions/0/structure_evidence",
        ),
    ] {
        audit(n, image).unwrap_or_else(|e| panic!("{label} 原图像审计失败：{e}"));
        for field in ["basis_ref", "session_generation"] {
            let mut tampered = image.clone();
            let target = tampered.pointer_mut(copy).unwrap();
            assert_eq!(target["kind"], "FinancialBasisEvidence");
            if field == "basis_ref" {
                let root = target["basis_ref"]["root"].as_str().unwrap().to_owned();
                let flipped = if root.ends_with('0') { '1' } else { '0' };
                target["basis_ref"]["root"] =
                    json!(format!("{}{flipped}", &root[..root.len() - 1]));
            } else {
                target["session_generation"] = json!("2");
            }
            let error = audit(n, &tampered).expect_err("只改一份证据也必须拒绝");
            out.insert(format!("{label}:{field}"), json!(error));
        }
    }
    Value::Object(out)
}

/// Reserve 结构校验按 s_ref 值去重的负控，在 E 已发布的同一 witness 上直接走 `verify_structure_once`：
/// 同值两次只核一次；两个不同的合法 s_ref 各核一次都通过；第二个改坏 object_hash 的 s_ref 仍被拒绝。
fn s_ref_controls(f: &Fixture, real_s: &Value) -> Value {
    let e_image = latest(&f.dbs[0]).unwrap().3;
    let witness = &e_image["history"].as_array().unwrap().last().unwrap()["witness"]["structure"];
    let first = real_s["s_ref"].clone();
    let export = &real_s["export"];
    let structure = &real_s["structure"];
    let binary = PathBuf::from(structure["binary"].as_str().unwrap());
    let socket = PathBuf::from(structure["reader_session"].as_str().unwrap());
    let second = export["object_ids"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| **id != first["object_id"])
        .find_map(|id| {
            let reply = basis_ref_reply(&binary, export, &socket, id)?;
            Some(json!({"schema_revision":"s-object-ref/2","object_id":id,
                "object_hash":reply["selected_objects"][0]["object_hash"],"profile_id":export["profile_id"]}))
        })
        .expect("导出快照须有第二个可读对象");
    let mut bad = second.clone();
    let hash = bad["object_hash"].as_str().unwrap().to_owned();
    let flipped = if hash.ends_with('0') { '1' } else { '0' };
    bad["object_hash"] = json!(format!("{}{flipped}", &hash[..hash.len() - 1]));
    let run = |refs: &[&Value]| -> (Result<(), String>, usize) {
        let mut job =
            crate::economic_session::store::new_content_job(&f.dbs[0], &f.configs[0]).unwrap();
        let mut budget = crate::economic_session::store::ReadBudget::new(Some(&mut job), None);
        let mut verified = BTreeSet::new();
        phase_trace::begin();
        let result = refs.iter().try_for_each(|s_ref| {
            verify_structure_once(
                &mut verified,
                &f.dbs[0],
                &f.configs[0],
                &mut budget,
                witness,
                &json!({"s_ref":s_ref}),
            )
        });
        let checks = phase_trace::take()["phases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| {
                p["phase"]
                    .as_str()
                    .unwrap()
                    .starts_with("structure_verify@")
            })
            .count();
        (result, checks)
    };
    let (same, same_checks) = run(&[&first, &first]);
    assert_eq!(
        (same.clone(), same_checks),
        (Ok(()), 1),
        "同值 s_ref 只核一次"
    );
    let (distinct, distinct_checks) = run(&[&first, &second]);
    assert_eq!(
        (distinct.clone(), distinct_checks),
        (Ok(()), 2),
        "不同合法 s_ref 各核一次"
    );
    let (rejected, rejected_checks) = run(&[&first, &bad]);
    let error = rejected.expect_err("第二个坏 s_ref 必须拒绝");
    assert_eq!(rejected_checks, 1, "坏 s_ref 在校验内被拒，未记完成段");
    json!({"first":first,"second":second,"bad":bad,"same_checks":same_checks.to_string(),
        "distinct_checks":distinct_checks.to_string(),"bad_error":error})
}

/// #1716：真实 S 单 bar 闭包上，/4 正式 cap 激活的版本被两 B Prepare 之后的 E Reserve 实际消费。等值例
/// cap=90 全量授信；不足例 cap=89 整槽等待、零授信、无部分责任。manifest 冻结 `SessionClosure` 读法，
/// reader_resources 八项与 E/B 3000 ms 响应期限保持原值；停机 FINAL=PASS 证明全部 S 回复出自同一已审状态。
/// 运行方式：S 侧 `TB1717_S_EXPORT=<json> cargo test --features s_session --bin s_structure_session --
/// --ignored --exact v3::financial_session::tests::export_real_s3_fixture_for_economic_content_domain`，
/// `cargo build --release --features s_session --bin s_structure_session`，再带三卷、`TB1716_S_EXPORT` 与
/// `TB1716_S_BINARY` 跑本测试。
#[test]
#[ignore = "需 TB1716_HFS_CARRIERS、TB1716_S_BINARY、TB1716_S_EXPORT 与 bind Unix socket 的权限"]
fn content_domain_activated_cap_consumed_by_real_s_reserve() {
    let carriers: Vec<PathBuf> = std::env::var("TB1716_HFS_CARRIERS")
        .expect("TB1716_HFS_CARRIERS 未设置")
        .split(':')
        .map(PathBuf::from)
        .collect();
    assert_eq!(carriers.len(), 3, "需要 E、B:alpha、B:beta 三个卷");
    let binary = env_path("TB1716_S_BINARY");
    let export_path = env_path("TB1716_S_EXPORT");
    let export = strict_json(&std::fs::read(&export_path).unwrap()).unwrap();
    let session = SSession::start(&binary, &export, export_path.parent().unwrap());
    let real_s = real_s_from_export(&binary, &export, &session.socket);
    // 用例断言失败也先停掉本测试起的 S 会话并留下 FINAL，再按原失败退出。
    let cases = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let equal = reserve_under_activated_cap(&carriers, &real_s, "90");
        let short = reserve_under_activated_cap(&carriers, &real_s, "89");
        (equal, short)
    }));
    let session_ready = session.ready.clone();
    let session_final = session.stop(&binary);
    let (equal, short) = cases.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    let evidence = json!({"carriers":carriers,"reading":"SessionClosure","response_timeout_ms":"3000",
        "s_source":real_s,"s_session_ready":session_ready,"s_session_final":session_final,
        "equal_cap":equal,"insufficient_cap":short});
    let root = std::env::var_os("TB03B_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    save(&root, "issue1716-content-domain-real-s.json", &evidence);
    println!(
        "ISSUE1716_CONTENT_DOMAIN_REAL_S {}",
        canonical_json(&evidence)
    );
}

/// 一个 /4 正式 socket 例：夹具以生产 handler 完成开局与既有责任导入后关闭全部连接，交由
/// `s_session/tests/tb1716_formal_consumption.py` 经 economic_service_control 起 E、两 B 正式服务进程，
/// 经 socket 直发两 B Prepare、E 激活 cap 版本 2、E Reserve 与两 B Commit 的正式子协议，最后以 ChongAdvance
/// 经 E 协调入口按原身份重送取回同一 Reserve 与 Commit 回执；不证明一次首发 ChongAdvance 包办全流程。
/// `fault` 为 manifest 的 TestOnly 故障缝，驱动在 marker 处 SIGKILL 实际 owner、recover 并以新 epoch 起新进程。
fn formal_socket_case(
    carriers: &[PathBuf],
    real_s: &Value,
    binaries: &Path,
    cap: &str,
    fault: Option<(&str, &str)>,
) -> Value {
    let f = Fixture::content_with_real_s(carriers, "100", "20", real_s, fault);
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 20);
    let activation = cap_payload(&f, "1", "2", cap, "100000");
    let fault = fault.map(|(message, phase)| {
        let domain = message
            .strip_prefix("tb03b-CommitDecision-")
            .map_or("E".to_owned(), |name| format!("B:{name}"));
        json!({"message_id":message,"phase":phase,"domain":domain,"marker_path":f.dir.join("fault-marker.json")})
    });
    let case = json!({"manifest":f.dir.join("manifest.json"),"activation":activation,"cap":cap,"fault":fault});
    save(&f.dir, "formal-case.json", &case);
    let dir = f.dir.clone();
    drop(f);
    let driver = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../s_session/tests/tb1716_formal_consumption.py");
    let status = std::process::Command::new("python3")
        .arg("-B")
        .arg(&driver)
        .arg("--case")
        .arg(dir.join("formal-case.json"))
        .arg("--binaries")
        .arg(binaries)
        .arg("--output")
        .arg(dir.join("formal"))
        .status()
        .unwrap();
    let result = strict_json(&std::fs::read(dir.join("formal/RESULT.json")).unwrap()).unwrap();
    assert!(status.success(), "{result}");
    assert_eq!(result["status"], "passed", "{result}");
    result
}

/// #1716：真实 S 单 bar 闭包上，/4 新 cap 经 E/B 正式服务进程与 Unix socket 被实际消费。cap=90 时 E Reserve
/// 与两 B Commit 经正式子协议提交，ChongAdvance 取回同一回执，责任 20+30+40=90；cap=89 时整槽等待、两 B 不提交。
/// 两个恢复例在 E Reserve 提交后、B:alpha Commit 提交后 SIGKILL 实际进程，recover 后以新 epoch 起新进程重送，
/// 取回原回执且每域恰一份 Reserve/Commit。运行方式同上一测试，另需 `TB1716_ECON_BINARIES`（含 release
/// `e_economic_session`、`b_chong_session`，ADR 0039 的生产构建配置）。S 会话原件写在 TB03B_TEST_ROOT/formal-s-session。
#[test]
#[ignore = "需 TB1716_HFS_CARRIERS、TB1716_S_BINARY、TB1716_S_EXPORT、TB1716_ECON_BINARIES 与 bind Unix socket 的权限"]
fn content_domain_activated_cap_consumed_through_formal_sockets() {
    let carriers: Vec<PathBuf> = std::env::var("TB1716_HFS_CARRIERS")
        .expect("TB1716_HFS_CARRIERS 未设置")
        .split(':')
        .map(PathBuf::from)
        .collect();
    assert_eq!(carriers.len(), 3, "需要 E、B:alpha、B:beta 三个卷");
    let binary = env_path("TB1716_S_BINARY");
    let binaries = env_path("TB1716_ECON_BINARIES");
    let export_path = env_path("TB1716_S_EXPORT");
    let export = strict_json(&std::fs::read(&export_path).unwrap()).unwrap();
    let root = std::env::var_os("TB03B_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let s_work = root.join("formal-s-session");
    std::fs::create_dir_all(&s_work).unwrap();
    let session = SSession::start(&binary, &export, &s_work);
    let real_s = real_s_from_export(&binary, &export, &session.socket);
    let cases = [
        ("equal", "90", None),
        ("insufficient", "89", None),
        (
            "e-reserve-after-commit",
            "90",
            Some(("tb03b-Reserve-slot", "after_commit")),
        ),
        (
            "b-alpha-commit-after-commit",
            "90",
            Some(("tb03b-CommitDecision-alpha", "after_commit")),
        ),
    ];
    let mut outcomes = serde_json::Map::new();
    for (name, cap, fault) in cases {
        outcomes.insert(
            name.into(),
            formal_socket_case(&carriers, &real_s, &binaries, cap, fault),
        );
    }
    let session_ready = session.ready.clone();
    let session_final = session.stop(&binary);
    let evidence = json!({"carriers":carriers,"reading":"SessionClosure","response_timeout_ms":"3000",
        "s_source":real_s,"s_session_ready":session_ready,"s_session_final":session_final,"cases":outcomes});
    let (path, _) = save(&root, "issue1716-formal-socket-consumption.json", &evidence);
    println!("ISSUE1716_FORMAL_SOCKET_CONSUMPTION {path}");
}
