// Insert this item inside `mod v2_fault_acceptance` after its existing helpers.
// Only the financial stages below use the formal E/B Unix CLI. The fixture's
// Capture/Apply/BindVoice/existing20 setup remains an explicit TestOnly prefix.
#[cfg(target_os = "macos")]
mod formal_cli_acceptance {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Server {
        owner: String,
        child: Child,
        argv: Vec<String>,
        binary: PathBuf,
        binary_sha256: String,
        socket: PathBuf,
        stdout: PathBuf,
        stderr: PathBuf,
    }
    struct Servers {
        dir: PathBuf,
        children: Vec<Server>,
    }
    impl Servers {
        fn new(dir: &Path) -> Self {
            Self {
                dir: dir.to_owned(),
                children: Vec::new(),
            }
        }
        fn start(&mut self, owner: &str, f: &V2Fixture, binary: &Path, sha: &str) {
            assert!(binary.is_absolute());
            pin(binary, sha);
            let index = match owner {
                "e" => 0,
                "alpha" => 1,
                "beta" => 2,
                _ => panic!("unknown owner"),
            };
            let config = &f.configs[index];
            let mut argv = vec![
                "serve".to_owned(),
                "--manifest".to_owned(),
                f.manifest.display().to_string(),
                "--db".to_owned(),
                config.db.display().to_string(),
            ];
            if owner != "e" {
                argv.extend(["--chong".to_owned(), owner.to_owned()]);
            }
            argv.extend(["--epoch".to_owned(), "1".to_owned()]);
            if owner != "e" {
                argv.extend(["--e-epoch".to_owned(), "1".to_owned()]);
            }
            let stdout = self.dir.join(format!("formal-{owner}.stdout"));
            let stderr = self.dir.join(format!("formal-{owner}.stderr"));
            let out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stdout)
                .unwrap();
            let err = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stderr)
                .unwrap();
            let child = Command::new(binary)
                .args(&argv)
                .stdin(Stdio::null())
                .stdout(Stdio::from(out))
                .stderr(Stdio::from(err))
                .spawn()
                .unwrap();
            let socket = PathBuf::from(
                required(
                    if owner == "e" {
                        &config.manifest["e"]
                    } else {
                        config.route(owner).unwrap()
                    },
                    "socket",
                )
                .unwrap(),
            );
            self.children.push(Server {
                owner: owner.to_owned(),
                child,
                argv,
                binary: binary.to_owned(),
                binary_sha256: sha.to_owned(),
                socket: socket.clone(),
                stdout,
                stderr,
            });
            let until = Instant::now() + Duration::from_secs(20);
            while !socket.exists() {
                assert!(
                    self.children
                        .last_mut()
                        .unwrap()
                        .child
                        .try_wait()
                        .unwrap()
                        .is_none(),
                    "formal serve exited before socket: {owner}"
                );
                assert!(
                    Instant::now() < until,
                    "formal serve socket deadline: {owner}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        fn socket(&self, owner: &str) -> &Path {
            &self
                .children
                .iter()
                .find(|s| s.owner == owner)
                .unwrap()
                .socket
        }
    }
    impl Drop for Servers {
        fn drop(&mut self) {
            let mut rows = Vec::new();
            for server in &mut self.children {
                let pid = server.child.id();
                let already = server.child.try_wait().ok().flatten();
                let killed = already.is_none();
                if killed {
                    let _ = server.child.kill();
                }
                let status = already.or_else(|| server.child.wait().ok());
                rows.push(
                    json!({"owner":server.owner,"pid":pid.to_string(),"binary":server.binary,
                    "binary_sha256":server.binary_sha256,"argv":server.argv,"socket":server.socket,
                    "stdout":server.stdout,"stderr":server.stderr,"test_stopped":killed,
                    "exit_code":status.as_ref().and_then(|s|s.code()),
                    "signal":status.as_ref().and_then(|s|s.signal())}),
                );
            }
            let path = self.dir.join("formal-cli-processes.json");
            let all_reaped = rows.len() == self.children.len();
            let _ = std::fs::write(
                path,
                canonical_json(&json!({"schema_revision":"testonly-formal-cli-processes/1",
                "processes":rows,"all_reaped":all_reaped})),
            );
        }
    }
    fn send(socket: &Path, request: &Value, max_frame: usize) -> Value {
        let mut stream = std::os::unix::net::UnixStream::connect(socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(90)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(90)))
            .unwrap();
        let frame = format!("{}\n", canonical_json(request));
        assert!(frame.len() <= max_frame);
        stream.write_all(frame.as_bytes()).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut reply = Vec::new();
        stream
            .take((max_frame + 1) as u64)
            .read_to_end(&mut reply)
            .unwrap();
        assert!(reply.len() <= max_frame && reply.last() == Some(&b'\n'));
        assert!(
            !reply[..reply.len() - 1].contains(&b'\n'),
            "multiple LF frames"
        );
        let value = strict_json(&reply[..reply.len() - 1]).unwrap();
        assert_eq!(value["schema_revision"], SCHEMA_V2);
        assert_eq!(value["payload_hash"], hash(&value["payload"]));
        value
    }
    fn payload(reply: &Value, expected: &str) -> Value {
        let p = &reply["payload"];
        assert_eq!(p["kind"], expected, "{p}");
        p.clone()
    }
    fn amount_total(image: &Value) -> i64 {
        image["finance"]["responsibilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| required(r, "amount").unwrap().parse::<i64>().unwrap())
            .sum()
    }

    #[test]
    #[ignore = "requires an explicit fresh three-owner HFS+ binding, pinned CLI binaries and TestOnly S/cache"]
    fn formal_e_b_unix_prepare_reserve_commit_and_public_queries() {
        let binding_path = PathBuf::from(std::env::var("EB_FORMAL_CLI_BINDING").unwrap());
        let binding_sha = std::env::var("EB_FORMAL_CLI_BINDING_SHA256").unwrap();
        let binding: Value = strict_json(&pin(&binding_path, &binding_sha)).unwrap();
        assert_eq!(
            binding["schema_revision"],
            "testonly-1394-formal-cli-binding/1"
        );
        let case = &binding["formal_case"];
        assert_eq!(case["case_id"], "formal-cli-fresh-v2");
        let e_bin = PathBuf::from(std::env::var("EB_FORMAL_E_BIN").unwrap());
        let b_bin = PathBuf::from(std::env::var("EB_FORMAL_B_BIN").unwrap());
        let e_sha = std::env::var("EB_FORMAL_E_BIN_SHA256").unwrap();
        let b_sha = std::env::var("EB_FORMAL_B_BIN_SHA256").unwrap();
        assert_eq!(binding["cli"]["e"]["path"], e_bin.display().to_string());
        assert_eq!(binding["cli"]["e"]["sha256"], e_sha);
        assert_eq!(binding["cli"]["b"]["path"], b_bin.display().to_string());
        assert_eq!(binding["cli"]["b"]["sha256"], b_sha);
        pin(&e_bin, &e_sha);
        pin(&b_bin, &b_sha);
        // The fixture freezes manifest/input/S/cache before initializing any owner.
        // Its named fault marker is never addressed by the requests below.
        let f = V2Fixture::new(&binding, case, "FormalCliProbe", "none", "before_commit");
        assert_eq!(f.configs[0].manifest["clock"]["kind"], "TestOnly");
        assert_eq!(f.data["policy"]["model"], "TestOnlyUnleveragedCashBuy");
        assert_eq!(amount_total(&latest(&f.dbs[0]).unwrap().3), 20);
        for (index, owner) in [(1, "alpha"), (2, "beta")] {
            let image = latest(&f.dbs[index]).unwrap().3;
            assert!(!image["book"].is_null());
            assert!(image["book"]["voice"]["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["binding_id"] == format!("voice-{owner}")));
        }
        create_json(
            &f.dir,
            "formal-cli-runtime.json",
            &json!({"schema_revision":"testonly-formal-cli-runtime/1",
            "binding_sha256":binding_sha,"manifest_sha256":f.manifest_sha,
            "e_binary":e_bin,"e_sha256":e_sha,"b_binary":b_bin,"b_sha256":b_sha}),
        );
        let mut servers = Servers::new(&f.dir);
        servers.start("e", &f, &e_bin, &e_sha);
        servers.start("alpha", &f, &b_bin, &b_sha);
        servers.start("beta", &f, &b_bin, &b_sha);
        let frame_limit: usize = required(&f.configs[0].manifest["resources"], "max_frame_bytes")
            .unwrap()
            .parse()
            .unwrap();
        let ask = |owner: &str, msg: &str, p: Value, servers: &Servers| -> Value {
            let request = env_json(&f.configs[0].session, msg, p);
            send(servers.socket(owner), &request, frame_limit)
        };
        let pa_req = f.request("PrepareDecision", "alpha");
        let pb_req = f.request("PrepareDecision", "beta");
        let pa = payload(
            &send(servers.socket("alpha"), &pa_req, frame_limit),
            "PreparedReceipt",
        );
        let pb = payload(
            &send(servers.socket("beta"), &pb_req, frame_limit),
            "PreparedReceipt",
        );
        assert_eq!(pa["record"]["prepared"]["locked_amount"], "30");
        assert_eq!(pb["record"]["prepared"]["locked_amount"], "40");
        let reserve_req = f.request("Reserve", "slot");
        let reserve = payload(
            &send(servers.socket("e"), &reserve_req, frame_limit),
            "ReserveReceipt",
        );
        assert_eq!(reserve["record"]["retained_total"], "90");
        let grants = reserve["record"]["grants"].as_array().unwrap();
        assert_eq!(grants.len(), 2);
        assert_eq!(grants[0]["amount"], "30");
        assert_eq!(grants[1]["amount"], "40");
        let ca_req = f.request("CommitDecision", "alpha");
        let cb_req = f.request("CommitDecision", "beta");
        let ca = payload(
            &send(servers.socket("alpha"), &ca_req, frame_limit),
            "LocalCommitReceipt",
        );
        let cb = payload(
            &send(servers.socket("beta"), &cb_req, frame_limit),
            "LocalCommitReceipt",
        );
        assert_eq!(ca["record"]["state"], "Committed");
        assert_eq!(cb["record"]["state"], "Committed");
        assert_eq!(ca["record"]["execution_eligibility"], "None");
        assert_eq!(cb["record"]["execution_eligibility"], "None");
        assert_eq!(
            payload(
                &send(servers.socket("alpha"), &ca_req, frame_limit),
                "LocalCommitReceipt"
            ),
            ca
        );
        assert_eq!(
            payload(
                &send(servers.socket("beta"), &cb_req, frame_limit),
                "LocalCommitReceipt"
            ),
            cb
        );
        let e_view = payload(
            &ask(
                "e",
                "formal-read-e",
                json!({"op":"ReadView","cut":null,"as_known_ns":null}),
                &servers,
            ),
            "EconomicView",
        );
        assert_eq!(amount_total(&e_view["image"]), 90);
        let a_view = payload(
            &ask(
                "alpha",
                "formal-read-alpha",
                json!({"op":"ReadView","cut":null,"as_known_ns":null}),
                &servers,
            ),
            "EconomicView",
        );
        let b_view = payload(
            &ask(
                "beta",
                "formal-read-beta",
                json!({"op":"ReadView","cut":null,"as_known_ns":null}),
                &servers,
            ),
            "EconomicView",
        );
        assert_eq!(
            a_view["image"]["finance"]["decisions"][0]["state"],
            "Committed"
        );
        assert_eq!(
            b_view["image"]["finance"]["decisions"][0]["state"],
            "Committed"
        );
        let mut decisions = Vec::new();
        for (owner, name, receipt) in [("alpha", "alpha", &ca), ("beta", "beta", &cb)] {
            let q = payload(
                &ask(
                    owner,
                    &format!("formal-decision-{name}"),
                    json!({"op":"QueryDecision",
                "decision_id":name,"cut":null,"as_known_ns":null}),
                    &servers,
                ),
                "EconomicDecision",
            );
            assert_eq!(q["decision"]["state"], "Committed");
            assert!(q["receipts"].as_array().unwrap().contains(receipt));
            decisions.push(q);
        }
        let mut grant_queries = Vec::new();
        for grant in grants {
            let id = required(grant, "grant_id").unwrap();
            let q = payload(
                &ask(
                    "e",
                    &format!("formal-grant-{id}"),
                    json!({"op":"QueryGrant",
                "grant_id":id,"cut":null,"as_known_ns":null}),
                    &servers,
                ),
                "EconomicGrant",
            );
            assert_eq!(q["grant"], *grant);
            assert_eq!(q["receipt"], reserve);
            grant_queries.push(q);
        }
        let mut watches = Vec::new();
        for (owner, view) in [("e", &e_view), ("alpha", &a_view), ("beta", &b_view)] {
            let q = payload(
                &ask(
                    owner,
                    &format!("formal-watch-{owner}"),
                    json!({"op":"Watch","after_cut":null}),
                    &servers,
                ),
                "EconomicChanges",
            );
            assert_eq!(q["cut"], view["cut"]);
            assert_eq!(
                q["changes"].as_array().unwrap().last().unwrap()["image"],
                view["image"]
            );
            watches.push(q);
        }
        assert!(
            !f.dir.join("fault-marker.json").exists(),
            "unused TestOnly fault was triggered"
        );
        create_json(
            &f.dir,
            "formal-cli-results.json",
            &json!({"schema_revision":"testonly-formal-cli-acceptance/1",
            "scope":"formal E/B Unix serve, new HFS owners, pinned S/cache; setup Capture/Apply/BindVoice/existing20 remains library-only",
            "prepare":[pa,pb],"reserve":reserve,"commit":[ca,cb],"read_views":[e_view,a_view,b_view],
            "query_decisions":decisions,"query_grants":grant_queries,"watches":watches}),
        );
        drop(servers); // SIGKILL/reap is recorded; no transaction remains active.
        assert!(f.dir.join("formal-cli-processes.json").exists());
    }
}
