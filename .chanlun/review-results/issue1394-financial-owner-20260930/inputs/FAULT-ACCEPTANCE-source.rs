// #1394 /2 故障验收追加源。include! 于 finance_tests.rs；不替换旧 /1 测试。
// 此入口是 Rust test-child 真实 S3 + 本地持久边界；正式 CLI/Unix 会话另验。
#[cfg(target_os = "macos")]
mod v2_fault_acceptance {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::process::ExitStatusExt;
    const INPUTS_SHA: &str = "5a7280178e45f6728c3d4afa9d313b98664af8c4141cccac10c99df639dfa82f";
    const RESOURCES_SHA: &str = "f8e9e3ca737b3c232c0c62bf748f524badeed575646c53338dca35b1804953dc";
    const CHILD: &str = "economic_session::finance::tests::v2_fault_acceptance::durable_v2_child";
    const OPS: [(&str, &str, usize); 6] = [
        ("PrepareDecision", "alpha", 1),
        ("Reserve", "slot", 0),
        ("CommitDecision", "alpha", 1),
        ("ImportResponsibility", "existing20", 0),
        ("AbortDecision", "alpha", 1),
        ("CloseApplicants", "slot", 0),
    ];
    fn pin(path: &Path, expected: &str) -> Vec<u8> {
        let raw = std::fs::read(path).unwrap();
        assert_eq!(sha256_hex(&raw), expected, "pin {}", path.display());
        raw
    }
    fn hex(raw: &str) -> Vec<u8> {
        assert_eq!(raw.len() % 2, 0);
        (0..raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
            .collect()
    }
    fn decode_cache_node(row: &Value) -> Result<([u8; 32], u8, Vec<u8>), String> {
        let digest: [u8; 32] = hex(required(row, "digest")?)
            .try_into()
            .map_err(|_| "cache digest must be exactly 32 bytes".to_owned())?;
        // Empty NULL/BOOL/empty sequence bodies are real bytes. Absence or a
        // non-string value is rejected, never replaced with default bytes.
        let body = hex(row
            .get("body")
            .and_then(Value::as_str)
            .ok_or("cache body must be a hex string, including empty")?);
        let tag = u8::try_from(
            row["tag"]
                .as_u64()
                .ok_or("cache tag must be an unsigned integer")?,
        )
        .map_err(|_| "cache tag exceeds u8".to_owned())?;
        crate::s_content::job::CheckedNodeUsage::checked(&digest, tag, &body)
            .map_err(|error| error.to_string())?;
        Ok((digest, tag, body))
    }
    #[test]
    fn cache_empty_bodies_keep_real_tags_and_digest_checks() {
        for tag in [
            crate::s_content::tag::NULL,
            crate::s_content::tag::BOOL_FALSE,
            crate::s_content::tag::BOOL_TRUE,
        ] {
            let digest = crate::s_content::frame(tag, &[]);
            let digest_hex = crate::s_content::root_string(&digest)[4..].to_owned();
            let row = json!({"digest":digest_hex,"tag":tag,"body":""});
            assert_eq!(decode_cache_node(&row).unwrap(), (digest, tag, vec![]));
            let mut wrong_digest = row.clone();
            wrong_digest["digest"] = json!("00".repeat(32));
            assert!(decode_cache_node(&wrong_digest).is_err());
            for non_string in [Value::Null, json!(false), json!(0), json!([]), json!({})] {
                let mut bad = row.clone();
                bad["body"] = non_string;
                assert!(decode_cache_node(&bad).is_err());
            }
            let mut absent = row.clone();
            absent.as_object_mut().unwrap().remove("body");
            assert!(decode_cache_node(&absent).is_err());
        }
        // Read only the same frozen cache used by root's failed first case.
        // This is a decoder test, never an economic DB/HFS or SIGKILL run.
        let cache_path = Path::new("/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-eb-content-owner-20260930/acceptance-raw/published-cache.json");
        let cache: Value = strict_json(&pin(
            cache_path,
            "6219f3af68b78ac9eed0e314d1aeacb984c0a61556c59ce0fee39261ceda7efa",
        ))
        .unwrap();
        let rows = cache["nodes"].as_array().unwrap();
        assert_eq!(rows.len(), 1307);
        let mut empty_tags = std::collections::BTreeSet::new();
        for row in rows {
            let (_, tag, body) = decode_cache_node(row).unwrap();
            if body.is_empty() {
                empty_tags.insert(tag);
            }
        }
        assert_eq!(empty_tags, std::collections::BTreeSet::from([0, 1, 2, 17]));
    }
    fn create_json(dir: &Path, name: &str, value: &Value) -> (PathBuf, String) {
        use std::io::Write;
        let path = dir.join(name);
        let raw = canonical_json(value);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        f.write_all(raw.as_bytes()).unwrap();
        f.sync_all().unwrap();
        (path, sha256_hex(raw.as_bytes()))
    }
    fn env_json(sid: &str, msg: &str, payload: Value) -> Value {
        json!({"schema_revision":SCHEMA_V2,"session_id":sid,"session_generation":"1",
            "source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1",
            "message_id":msg,"payload_hash":hash(&payload),"causal_refs":[],"payload":payload})
    }
    fn node_count(c: &Connection, table: &str) -> i64 {
        assert!(matches!(table, "content_nodes" | "content_staging"));
        c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn bounded_wait(child: &mut std::process::Child) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "child {status}");
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("child deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    fn spawn(dir: &Path, job: &Value, label: &str) -> std::process::Child {
        let (jp, jh) = create_json(dir, &format!("{label}-job.json"), job);
        let exe = std::env::current_exe().unwrap();
        create_json(
            dir,
            &format!("{label}-command.json"),
            &json!({"executable":exe,
            "args":["--exact",CHILD,"--ignored","--nocapture"],"job":jp,"job_sha256":jh,
            "proof_domain":"test-child, real release S3, fixed HFS owner; NOT formal E/B CLI/Unix session"}),
        );
        std::process::Command::new(exe)
            .args(["--exact", CHILD, "--ignored", "--nocapture"])
            .env("EB_V2_FAULT_CHILD_JOB", jp)
            .env("EB_V2_FAULT_CHILD_JOB_SHA256", jh)
            .stdout(std::fs::File::create(dir.join(format!("{label}.stdout"))).unwrap())
            .stderr(std::fs::File::create(dir.join(format!("{label}.stderr"))).unwrap())
            .spawn()
            .unwrap()
    }
    #[test]
    #[ignore = "只能由具名 /2 故障调度传入冻结 job"]
    fn durable_v2_child() {
        let jp = PathBuf::from(std::env::var("EB_V2_FAULT_CHILD_JOB").unwrap());
        let job: Value = strict_json(&pin(
            &jp,
            &std::env::var("EB_V2_FAULT_CHILD_JOB_SHA256").unwrap(),
        ))
        .unwrap();
        let mp = Path::new(required(&job, "manifest").unwrap());
        pin(mp, required(&job, "manifest_sha256").unwrap());
        let db = Path::new(required(&job, "db").unwrap());
        assert_eq!(
            db.parent().unwrap().canonicalize().unwrap(),
            PathBuf::from(required(&job, "mount").unwrap())
        );
        assert_eq!(
            db.parent().unwrap().metadata().unwrap().dev().to_string(),
            job["mount_dev"].as_str().unwrap()
        );
        let config = Config::load(mp, db, job["chong"].as_str()).unwrap();
        assert_eq!(
            config.manifest_hash,
            job["config_manifest_hash"].as_str().unwrap()
        );
        let result = if job["action"] == "recover" {
            recover(&config).unwrap()
        } else {
            let (mut c, _) = open(db, false).unwrap();
            // /2 write 自行取 pinned input、真实 S3 当次 basis、peer proof；禁止传入单位 witness。
            write(&mut c, &config, 1, &job["request"]).unwrap()
        };
        let (mut c, _) = open(db, false).unwrap();
        audit(&c, &config).unwrap();
        let query = service::handle(
            &mut c,
            &config,
            if job["action"] == "recover" { 2 } else { 1 },
            1,
            &json!({}),
            &env_json(
                &config.session,
                "fresh-child-read",
                json!({"op":"ReadView","cut":null,"as_known_ns":null}),
            ),
        )["payload"]
            .clone();
        assert_eq!(query["kind"], "EconomicView", "{query}");
        create_json(
            jp.parent().unwrap(),
            if job["action"] == "recover" {
                "recovered.json"
            } else {
                "child-result.json"
            },
            &json!({"pid":std::process::id().to_string(),"result":result,"public_read":query,
            "content_staging":node_count(&c,"content_staging")}),
        );
    }
    struct V2Fixture {
        dir: PathBuf,
        manifest: PathBuf,
        manifest_sha: String,
        configs: Vec<Config>,
        dbs: Vec<Connection>,
        data: Value,
    }
    impl V2Fixture {
        fn new(binding: &Value, case: &Value, op: &str, name: &str, phase: &str) -> Self {
            let dir = PathBuf::from(required(case, "evidence_dir").unwrap());
            assert!(!dir.exists(), "fresh evidence dir required");
            std::fs::create_dir_all(&dir).unwrap();
            // 只复用既有 /1 TestOnly 数值/声部权威模板，不把单位 S 当作 /2 证据。
            let old = Fixture::new("100");
            let root = PathBuf::from(required(binding, "small_source_root").unwrap());
            let inputs_raw = pin(
                &root.join("INPUTS.json"),
                required(binding, "inputs_sha256").unwrap(),
            );
            let inputs: Value = strict_json(&inputs_raw).unwrap();
            let resources: Value =
                strict_json(&pin(&root.join("RESOURCES.json"), RESOURCES_SHA)).unwrap();
            assert_eq!(inputs["reader_resources_sha256"], RESOURCES_SHA);
            // INPUTS_SHA 常量在夹具交付前由具名原件实际 sha256 代入。
            assert_eq!(required(binding, "inputs_sha256").unwrap(), INPUTS_SHA);
            let binary = dir.join("s_structure_session");
            let source_db = dir.join("s.sqlite");
            let raw = pin(
                Path::new(required(&inputs, "binary").unwrap()),
                required(&inputs, "binary_sha256").unwrap(),
            );
            std::fs::write(&binary, raw).unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o555)).unwrap();
            let raw = pin(
                Path::new(required(&inputs, "source_db").unwrap()),
                required(&inputs, "source_db_sha256").unwrap(),
            );
            std::fs::write(&source_db, raw).unwrap();
            let cache: Value = strict_json(&pin(
                Path::new(required(binding, "cache_path").unwrap()),
                required(binding, "cache_sha256").unwrap(),
            ))
            .unwrap();
            assert_eq!(cache["schema_revision"], "testonly-1394-published-cache/1");
            assert_eq!(cache["nodes"].as_array().unwrap().len(), 1307);
            assert_eq!(cache["basis_ref"], inputs["basis_ref"]);
            assert_eq!(cache["snapshot_ref"], inputs["snapshot_ref"]);
            let mut manifest = old.configs[0].manifest.clone();
            let sid = required(&inputs, "session_id").unwrap();
            manifest["session_id"] = json!(sid);
            manifest["protocol_revision"] = json!(SCHEMA_V2);
            manifest["finance"]["schema_revision"] = json!(REV_V2);
            manifest["resources"]["response_timeout_ms"] = json!("60000");
            manifest["finance"]["structure"] = json!({"binary":binary,"binary_sha256":inputs["binary_sha256"],
                "db":source_db,"session_id":sid,"session_generation":"1","writer_epoch":"1",
                "reader_revision":"s-financial-basis/3","content_format":"s-content/1","reader_resources":resources});
            let mut data = old.data.clone();
            data["schema_revision"] = json!(REV_V2);
            for plan in data["plans"].as_array_mut().unwrap() {
                plan["s_ref"] = json!({"schema_revision":"s-object-ref/2","object_id":inputs["object_id"],
                    "object_hash":"ac45fa2252898929523a432c7c5ca7f44b61225c1ac08b0681c52774b7f9f1ea","profile_id":"ohlc_integer_tb02a_v1"});
                plan["o_ref"]["s_object_id"] = inputs["object_id"].clone();
            }
            let (ip, ih) = create_json(&dir, "finance-input.json", &data);
            manifest["finance"]["input_path"] = json!(ip);
            manifest["finance"]["input_sha256"] = json!(ih);
            let marker = dir.join("fault-marker.json");
            manifest["test_only_fault"] =
                json!({"message_id":format!("{op}-{name}"),"phase":phase,"marker_path":marker});
            let mut seen = std::collections::BTreeSet::new();
            for (n, owner) in ["e", "alpha", "beta"].iter().enumerate() {
                let mount = PathBuf::from(required(&case["owners"][owner], "mount").unwrap());
                assert_eq!(
                    mount.canonicalize().unwrap(),
                    mount,
                    "canonical direct mount required"
                );
                let dev = mount.metadata().unwrap().dev();
                assert!(seen.insert(dev), "distinct owner volumes");
                assert_eq!(
                    dev.to_string(),
                    case["owners"][owner]["dev"].as_str().unwrap()
                );
                assert!(
                    std::fs::read_dir(&mount)
                        .unwrap()
                        .all(|entry| entry.unwrap().file_name() == ".fseventsd"),
                    "empty NEW carrier required"
                );
                let db = mount.join(format!("{owner}.db"));
                if n == 0 {
                    manifest["e"]["db"] = json!(db);
                    manifest["e"]["socket"] = json!(dir.join("e.sock"));
                } else {
                    manifest["chongs"][n - 1]["db"] = json!(db);
                    manifest["chongs"][n - 1]["socket"] = json!(dir.join(format!("{owner}.sock")));
                }
            }
            let (mp, mh) = create_json(&dir, "manifest.json", &manifest);
            let mut configs = vec![];
            let mut dbs = vec![];
            for (n, owner) in ["e", "alpha", "beta"].iter().enumerate() {
                let db = PathBuf::from(required(&case["owners"][owner], "mount").unwrap())
                    .join(format!("{owner}.db"));
                let config =
                    Config::load(&mp, &db, if n == 0 { None } else { Some(owner) }).unwrap();
                init(&config).unwrap();
                assert_eq!(
                    meta(&open(&db, false).unwrap().0, "schema_revision").unwrap(),
                    SCHEMA_V2
                );
                assert_eq!(
                    super::super::super::content_store::carrier_capacity(&db)
                        .unwrap()
                        .to_string(),
                    case["owners"][owner]["capacity"].as_str().unwrap()
                );
                let (mut c, _) = open(&db, false).unwrap();
                assert_eq!(node_count(&c, "content_nodes"), 0);
                let tx = c.transaction().unwrap();
                for row in cache["nodes"].as_array().unwrap() {
                    let (digest32, tag, body) = decode_cache_node(row).unwrap();
                    tx.execute(
                        "INSERT INTO content_nodes(digest,tag,body)VALUES(?1,?2,?3)",
                        rusqlite::params![&digest32[..], i64::from(tag), body],
                    )
                    .unwrap();
                }
                tx.commit().unwrap();
                c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
                assert_eq!(node_count(&c, "content_nodes"), 1307);
                // 缓存导入先以真实 SQLite/物理 Existing 核闭包。这份 setup Job 不替代随后每操作自己的 Job。
                let limits = super::super::super::financial_reader_resources::FinancialReaderResources::from_json(
                    &config.manifest["finance"]["structure"]["reader_resources"]).unwrap().job_limits;
                let mut cache_job = new_content_job(&c, &config).unwrap();
                let refs: Vec<_> = ["basis_ref", "snapshot_ref"]
                    .into_iter()
                    .map(|field| {
                        let r = &cache[field];
                        assert_eq!(r["kind"], "object");
                        crate::s_content::ContentRef::new(
                            crate::s_content::Kind::Object,
                            &crate::s_content::parse_root(required(r, "root").unwrap()).unwrap(),
                            Some(required(r, "extent").unwrap().parse().unwrap()),
                        )
                    })
                    .collect();
                {
                    let mut stage = super::super::super::content_store::Stage::new(
                        &mut c,
                        &db,
                        limits,
                        "testonly-cache-verification".into(),
                    )
                    .unwrap();
                    let mut metered =
                        crate::s_content::job::JobNodeSource::new(&mut cache_job, &mut stage);
                    assert_eq!(
                        crate::s_content::verify_many(&mut metered, refs).unwrap(),
                        1307
                    );
                }
                audit(&c, &config).unwrap();
                configs.push(config);
                dbs.push(c);
            }
            let mut f = Self {
                dir,
                manifest: mp,
                manifest_sha: mh,
                configs,
                dbs,
                data,
            };
            for (i, (owner, fixed, quantity)) in [("alpha", "12", "4"), ("beta", "20", "15")]
                .iter()
                .enumerate()
            {
                let mut doc = book::tests::document();
                let event = &mut doc["events"][0];
                event["book_identity"] =
                    json!({"chong_id":owner,"symbol":owner,"op_level":"2","fixed_q":fixed});
                event["observed_existing_book"]["units"] = json!(quantity);
                event["source"]["external_fact_id"] = json!(format!("opening-{owner}"));
                event["allocation_id"] = json!(format!("allocation-{owner}"));
                let got=f.handle(0,&format!("capture-{owner}"),json!({"op":"CaptureExternal","raw_utf8":canonical_json(&doc),"json_pointer":"/events/0","token":format!("token-{owner}"),"source_event_time":null}));
                assert_eq!(got["kind"], "Recorded", "{got}");
                let applied=f.handle(i+1,&format!("apply-{owner}"),json!({"op":"ApplyAllocation","fact_key":got["fact_key"],"allocation_id":format!("allocation-{owner}"),"expected_fact_hash":got["fact_hash"]}));
                assert_eq!(applied["kind"], "Applied", "{applied}");
                let reference = |id: &str| json!({"authority_id":owner,"object_id":id,"revision":"0","content_hash":hash(&json!(id))});
                let bound=f.handle(i+1,&format!("bind-{owner}"),json!({"op":"BindVoice","binding_id":format!("voice-{owner}"),"evidence_id":owner,"carrier_ref":reference("unit-carrier"),"gamma_ref":reference("unit-gamma"),"parent_voice_id":null,"sigma":"Long","n":"0","opening_basis":{"fact_key":got["fact_key"],"expected_fact_hash":got["fact_hash"],"allocation_id":format!("allocation-{owner}")}}));
                assert_eq!(bound["kind"], "VoiceBound", "{bound}");
            }
            if op != "ImportResponsibility" {
                f.step(0, "ImportResponsibility", "existing20");
            }
            f
        }
        fn handle(&mut self, n: usize, msg: &str, payload: Value) -> Value {
            service::handle(
                &mut self.dbs[n],
                &self.configs[n],
                1,
                1,
                &json!({}),
                &env_json(&self.configs[n].session, msg, payload),
            )["payload"]
                .clone()
        }
        fn request(&self, op: &str, name: &str) -> Value {
            let mut p = json!({"op":op,"writer_epoch":"1"});
            match op {
                "ImportResponsibility" => p["responsibility_id"] = json!(name),
                "Reserve" => {
                    p["slot_id"] = json!(name);
                    p["request_set_hash"] = json!(hash(&self.data["slots"][0]["applicants"]));
                }
                "CloseApplicants" => {
                    p["slot_id"] = json!(name);
                    p["closed"] = json!([{"owner":"B:alpha","applicant_epoch":"1","applicant_generation":"1"}]);
                    p["reason"] = json!("unit explicit supervised closure");
                }
                _ => {
                    let q = item(list(&self.data, "plans").unwrap(), "decision_id", name).unwrap();
                    p["decision_id"] = json!(name);
                    p["plan_hash"] = json!(hash(q));
                    if op == "AbortDecision" {
                        p["reason"] = json!("unit abort no release");
                    }
                    if op == "CommitDecision" {
                        let r = row(
                            &self.dbs[0],
                            "receipts",
                            &key_for(
                                &self.configs[0],
                                &self.request("Reserve", "slot")["payload"],
                            )
                            .unwrap(),
                        )
                        .unwrap()
                        .unwrap();
                        p["grant_id"] = r["record"]["grants"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|g| g["decision_id"] == name)
                            .unwrap()["grant_id"]
                            .clone();
                    }
                }
            }
            env_json(&self.configs[0].session, &format!("{op}-{name}"), p)
        }
        fn step(&mut self, n: usize, op: &str, name: &str) -> Value {
            let request = self.request(op, name);
            let result = write(&mut self.dbs[n], &self.configs[n], 1, &request).unwrap();
            if matches!(op, "PrepareDecision" | "Reserve" | "CommitDecision") {
                let image = latest(&self.dbs[n]).unwrap().3;
                let witness =
                    &image["history"].as_array().unwrap().last().unwrap()["witness"]["structure"];
                assert_eq!(witness["schema_revision"], "s-financial-evidence/1");
                assert_eq!(witness["session_id"], self.configs[0].session);
            }
            result
        }
        fn job(&self, n: usize, request: &Value) -> Value {
            json!({"action":"write","manifest":self.manifest,"manifest_sha256":self.manifest_sha,
                "config_manifest_hash":self.configs[n].manifest_hash,"db":self.configs[n].db,
                "mount":self.configs[n].db.parent().unwrap(),"mount_dev":self.configs[n].db.parent().unwrap().metadata().unwrap().dev().to_string(),
                "chong":if n==0{Value::Null}else{json!(if n==1{"alpha"}else{"beta"})},"request":request})
        }
    }
    fn validate_case_selection(cases: &[Value], filter: Option<&str>) -> Result<(), String> {
        let expected: std::collections::BTreeSet<String> = OPS
            .iter()
            .flat_map(|(op, _, _)| {
                ["before_commit", "after_commit"]
                    .into_iter()
                    .flat_map(move |phase| {
                        (0..2).map(move |repetition| format!("{op}-{phase}-{repetition}"))
                    })
            })
            .collect();
        if filter.is_some_and(|selected| !expected.contains(selected)) {
            return Err("unknown EB_V2_FAULT_CASE".into());
        }
        let mut bound = std::collections::BTreeSet::new();
        for case in cases {
            let id = required(case, "case_id")?;
            if !expected.contains(id) {
                return Err(format!("unknown case_id: {id}"));
            }
            if !bound.insert(id) {
                return Err(format!("duplicate case_id: {id}"));
            }
        }
        if let Some(selected) = filter {
            if !bound.contains(selected) {
                return Err("selected case is not bound".into());
            }
        } else if bound.len() != expected.len() {
            return Err("unfiltered matrix requires all 24 unique cases".into());
        }
        Ok(())
    }
    #[test]
    fn case_selection_accepts_bound_subset_and_rejects_invalid_ids() {
        let full: Vec<Value> = OPS.iter().flat_map(|(op, _, _)| {
            ["before_commit", "after_commit"].into_iter().flat_map(move |phase| {
                (0..2).map(move |repetition| json!({"case_id":format!("{op}-{phase}-{repetition}")}))
            })
        }).collect();
        let selected = "PrepareDecision-before_commit-0";
        assert!(validate_case_selection(&full, None).is_ok());
        assert!(validate_case_selection(&full[..1], Some(selected)).is_ok());
        // 非selected只核身份，不读任何不存在的 owners/mount，允许尚未创建载体。
        assert!(validate_case_selection(&full[..2], Some(selected)).is_ok());
        assert!(validate_case_selection(&full[..1], None).is_err());
        assert!(validate_case_selection(&full[..23], None).is_err());
        assert!(
            validate_case_selection(&[full[0].clone(), full[0].clone()], Some(selected)).is_err()
        );
        assert!(validate_case_selection(&full, Some("PrepareDecision-before_commit-2")).is_err());
        assert!(validate_case_selection(&full, Some("unknown")).is_err());
        assert!(validate_case_selection(&full, Some("")).is_err());
        assert!(validate_case_selection(&full[1..], Some(selected)).is_err());
        assert!(validate_case_selection(&[], Some(selected)).is_err());
        assert!(validate_case_selection(&[json!({"case_id":"unknown"})], Some(selected)).is_err());
        assert!(validate_case_selection(&[json!({})], Some(selected)).is_err());
    }
    #[test]
    #[ignore = "24 fresh cases require explicit frozen NEW HFS binding; never touch prior four owners"]
    fn transaction_v2_real_s3_before_after_sigkill_two_fresh_runs_each() {
        let filter = match std::env::var("EB_V2_FAULT_CASE") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                panic!("EB_V2_FAULT_CASE must be valid UTF-8")
            }
        };
        let binding_path = PathBuf::from(std::env::var("EB_V2_FAULT_BINDING").unwrap());
        let binding_sha = std::env::var("EB_V2_FAULT_BINDING_SHA256").unwrap();
        let binding: Value = strict_json(&pin(&binding_path, &binding_sha)).unwrap();
        assert_eq!(
            binding["schema_revision"],
            "testonly-1394-v2-fault-binding/1"
        );
        let cases = binding["cases"].as_array().unwrap();
        validate_case_selection(cases, filter.as_deref()).unwrap();
        let mut used = std::collections::BTreeSet::new();
        let mut observations = vec![];
        for (op, name, n) in OPS {
            for phase in ["before_commit", "after_commit"] {
                let mut two = vec![];
                for repetition in 0..2 {
                    let id = format!("{op}-{phase}-{repetition}");
                    if filter.as_ref().is_some_and(|selected| selected != &id) {
                        continue;
                    }
                    let case = cases
                        .iter()
                        .find(|c| c["case_id"] == id)
                        .expect("every case binding");
                    for owner in ["e", "alpha", "beta"] {
                        assert!(
                            used.insert(
                                required(&case["owners"][owner], "mount")
                                    .unwrap()
                                    .to_owned()
                            ),
                            "new carriers per case"
                        );
                    }
                    let mut f = V2Fixture::new(&binding, case, op, name, phase);
                    if !matches!(op, "PrepareDecision" | "ImportResponsibility") {
                        f.step(1, "PrepareDecision", "alpha");
                        f.step(2, "PrepareDecision", "beta");
                    }
                    if matches!(op, "CommitDecision" | "AbortDecision" | "CloseApplicants") {
                        let reserve = f.step(0, "Reserve", "slot");
                        assert_eq!(reserve["record"]["retained_total"], "90");
                    }
                    let request = f.request(op, name);
                    let previous = latest(&f.dbs[n]).unwrap();
                    let previous_total = responsibility_total(&latest(&f.dbs[0]).unwrap().3);
                    let job = f.job(n, &request);
                    let mut process = spawn(&f.dir, &job, "persist");
                    let pid = process.id();
                    let marker = f.dir.join("fault-marker.json");
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
                    while !marker.exists() {
                        if let Some(status) = process.try_wait().unwrap() {
                            panic!("not reached phase {status}: {}", f.dir.display());
                        }
                        if std::time::Instant::now() >= deadline {
                            let _ = process.kill();
                            let _ = process.wait();
                            panic!("phase deadline {}", f.dir.display());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    let reached: Value = strict_json(&std::fs::read(&marker).unwrap()).unwrap();
                    assert_eq!(reached["phase"], phase);
                    assert_eq!(reached["message_id"], request["message_id"]);
                    assert_eq!(reached["domain_id"], f.configs[n].domain);
                    assert_eq!(reached["pid"].as_u64().unwrap(), u64::from(pid));
                    process.kill().unwrap();
                    let killed = process.wait().unwrap();
                    assert_eq!(killed.signal(), Some(9));
                    let current = latest(&f.dbs[n]).unwrap();
                    let committed = phase == "after_commit";
                    assert_eq!(current.0, previous.0 + i64::from(committed));
                    let original = row(
                        &f.dbs[n],
                        "receipts",
                        &key_for(&f.configs[n], &request["payload"]).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(original.is_some(), committed);
                    if !committed {
                        assert_eq!(current, previous);
                    }
                    audit(&f.dbs[n], &f.configs[n]).unwrap();
                    let actual_kill_stage_rows = node_count(&f.dbs[n], "content_staging");
                    // Offline 只改本 case 的 S 副本，不改原 S/其他 owner 路径；manifest 始终冻结。
                    let structure = &f.configs[n].manifest["finance"]["structure"];
                    let sp = PathBuf::from(required(structure, "db").unwrap());
                    let bp = PathBuf::from(required(structure, "binary").unwrap());
                    std::fs::rename(&sp, sp.with_extension("offline")).unwrap();
                    std::fs::rename(&bp, bp.with_extension("offline")).unwrap();
                    let mut recovery_job = job.clone();
                    recovery_job["action"] = json!("recover");
                    let mut recovery = spawn(&f.dir, &recovery_job, "recover");
                    let recovery_pid = recovery.id();
                    assert_ne!(pid, recovery_pid);
                    bounded_wait(&mut recovery);
                    assert_eq!(meta(&f.dbs[n], "writer_epoch").unwrap(), "2");
                    assert_eq!(latest(&f.dbs[n]).unwrap(), current);
                    assert_eq!(
                        node_count(&f.dbs[n], "content_staging"),
                        0,
                        "recover must clean verified stale stage"
                    );
                    audit(&f.dbs[n], &f.configs[n]).unwrap();
                    pin(&f.manifest, &f.manifest_sha);
                    if committed {
                        assert_eq!(
                            write(&mut f.dbs[n], &f.configs[n], 2, &request).unwrap(),
                            original.clone().unwrap()
                        );
                    } else {
                        assert!(write(&mut f.dbs[n], &f.configs[n], 2, &request)
                            .unwrap_err()
                            .contains("StaleWriter"));
                    }
                    let mut new_identity = request.clone();
                    new_identity["message_id"] = json!(format!("stale-epoch-{op}"));
                    assert!(write(&mut f.dbs[n], &f.configs[n], 1, &new_identity)
                        .unwrap_err()
                        .contains("StaleWriter"));
                    let total = responsibility_total(&latest(&f.dbs[0]).unwrap().3);
                    let expected = match op {
                        "ImportResponsibility" => {
                            if committed {
                                20
                            } else {
                                0
                            }
                        }
                        "PrepareDecision" => 20,
                        "Reserve" => {
                            if committed {
                                90
                            } else {
                                20
                            }
                        }
                        _ => 90,
                    };
                    assert_eq!(total, expected);
                    assert!(total >= previous_total);
                    let comparison = json!({"op":op,"phase":phase,"original_receipt_exists":committed,"commit_delta":i64::from(committed).to_string(),"restored_writer_epoch":"2","retained_liability":total.to_string()});
                    two.push(comparison.clone());
                    let evidence = json!({"case_id":id,"pid":pid,"recovery_pid":recovery_pid,"signal":9,
                    "phase_marker":reached,"comparison":comparison,"original_receipt":original,"manifest_path":f.manifest,"manifest_sha256":f.manifest_sha,
                    "binding_path":binding_path,"binding_sha256":binding_sha,"actual_kill_stage_rows":actual_kill_stage_rows,
                    "recovered_stage_rows":"0","cache_count":"1307","cold_import_claim":false,"half_import_cleanup_claim":false,
                    "proof_domain":"Rust test-child with real S3, fixed new owners; formal E/B CLI + Unix transport still separate"});
                    create_json(&f.dir, "fault-result.json", &evidence);
                    observations.push(evidence);
                }
                if two.len() == 2 {
                    assert_eq!(two[0], two[1]);
                } else if filter.is_none() {
                    panic!("missing fresh repetition");
                }
            }
        }
        assert_eq!(observations.len(), if filter.is_some() { 1 } else { 24 });
        create_json(
            Path::new(required(&binding, "output_root").unwrap()),
            &format!("fault-matrix-{}.json", std::process::id()),
            &json!({"count":observations.len().to_string(),"filter":filter,"full_matrix_claim":observations.len()==24,"scenarios":observations}),
        );
    }
}
