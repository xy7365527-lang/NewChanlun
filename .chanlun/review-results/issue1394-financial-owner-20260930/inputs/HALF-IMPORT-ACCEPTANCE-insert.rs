// Include this item INSIDE mod v2_fault_acceptance. Preparation only; not product code.
// This adds no fault hook, synthetic staging INSERT, budget or policy override.
mod half_import_acceptance {
    use super::*;
    use std::time::{Duration, Instant};

    // Only inspect executable names, never arbitrary process arguments.
    fn case_processes(owner_pid: u32, source_binary: &Path) -> Vec<(u32, char, String)> {
        let out = std::process::Command::new("/bin/ps")
            .args(["-ww", "-axo", "pid=,ppid=,state=,comm="])
            .output().unwrap();
        assert!(out.status.success(), "process inventory failed");
        let text = String::from_utf8(out.stdout).unwrap();
        let mut rows = Vec::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let Some(pid) = fields.next().and_then(|s| s.parse::<u32>().ok()) else { continue };
            let Some(ppid) = fields.next().and_then(|s| s.parse::<u32>().ok()) else { continue };
            let Some(state) = fields.next().and_then(|s| s.chars().next()) else { continue };
            let command = fields.collect::<Vec<_>>().join(" ");
            if ppid == owner_pid || command == source_binary.to_string_lossy() {
                rows.push((pid, state, command));
            }
        }
        rows
    }
    fn stop_source_children(owner_pid: u32, source_binary: &Path) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stopped = std::collections::BTreeSet::new();
        loop {
            let live: Vec<_> = case_processes(owner_pid, source_binary).into_iter()
                .filter(|(_, state, _)| *state != 'Z').collect();
            if live.is_empty() {
                return json!({"signalled_pids":stopped,"live_case_source_processes":0});
            }
            for (pid, _, command) in live {
                // An orphaned reader is accepted only by this fresh case's exact binary path.
                assert_eq!(command, source_binary.to_string_lossy());
                let rc = unsafe { libc::kill(i32::try_from(pid).unwrap(), libc::SIGKILL) };
                assert!(rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH));
                stopped.insert(pid);
            }
            assert!(Instant::now() < deadline, "case source subprocess cleanup deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    struct ChildGuard {
        child: std::process::Child,
        dir: PathBuf,
        label: String,
        source_binary: PathBuf,
        reaped: bool,
    }
    impl ChildGuard {
        fn new(f: &V2Fixture, job: &Value, label: &str) -> Self {
            let source_binary = PathBuf::from(required(
                &f.configs[1].manifest["finance"]["structure"], "binary").unwrap());
            Self { child:spawn(&f.dir,job,label),dir:f.dir.clone(),label:label.into(),source_binary,reaped:false }
        }
        fn record_exit(&mut self, status: std::process::ExitStatus, reason: &str) {
            self.reaped = true;
            let sources = stop_source_children(self.child.id(), &self.source_binary);
            create_json(&self.dir,&format!("{}-exit.json",self.label),
                &json!({"pid":self.child.id(),"exit_code":status.code(),"signal":status.signal(),
                    "success":status.success(),"reason":reason,"owner_reaped":true,"source_cleanup":sources}));
        }
        fn kill_observed(&mut self) -> std::process::ExitStatus {
            self.child.kill().unwrap();
            let status = self.child.wait().unwrap();
            self.record_exit(status,"SIGKILL after real durable staging observation");
            status
        }
        fn wait_success(&mut self) {
            bounded_wait(&mut self.child);
            let status = self.child.wait().unwrap();
            self.record_exit(status,"fresh recovery completed");
            assert!(status.success());
        }
    }
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.reaped { return }
            let status = match self.child.try_wait() {
                Ok(Some(status)) => Some(status),
                _ => { let _ = self.child.kill(); self.child.wait().ok() }
            };
            self.reaped = true;
            // Preserve a failure cleanup record without panicking again during unwind.
            let sources = std::panic::catch_unwind(|| stop_source_children(self.child.id(), &self.source_binary));
            let value = json!({"pid":self.child.id(),"exit_code":status.and_then(|s|s.code()),
                "signal":status.and_then(|s|s.signal()),"owner_reaped":status.is_some(),
                "reason":"controller failure cleanup; no acceptance claim",
                "source_cleanup":sources.unwrap_or_else(|_|json!({"cleanup_unverified":true}))});
            if let Ok(mut out) = std::fs::OpenOptions::new().write(true).create_new(true)
                .open(self.dir.join(format!("{}-cleanup-exit.json",self.label))) {
                use std::io::Write;
                let _ = out.write_all(canonical_json(&value).as_bytes()); let _ = out.sync_all();
            }
        }
    }
    fn artifact_files(db: &Path) -> Value {
        let mut out = Vec::new();
        for suffix in ["", "-wal", "-shm"] {
            let path = PathBuf::from(format!("{}{suffix}",db.display()));
            match std::fs::metadata(&path) {
                Ok(m) => out.push(json!({"path":path,"bytes":m.len(),"allocated_bytes":m.blocks()*512})),
                Err(e) if e.kind()==std::io::ErrorKind::NotFound => out.push(json!({"path":path,"absent":true})),
                Err(e) => panic!("{}: {e}",path.display()),
            }
        }
        json!(out)
    }
    #[test]
    #[ignore = "requires one NEW half-import HFS binding; prepare/compile does not run it"]
    fn real_durable_stage_sigkill_then_fresh_offline_recover() {
        let bp = PathBuf::from(std::env::var("EB_V2_HALF_IMPORT_BINDING").unwrap());
        let bh = std::env::var("EB_V2_HALF_IMPORT_BINDING_SHA256").unwrap();
        let binding: Value = strict_json(&pin(&bp,&bh)).unwrap();
        assert_eq!(binding["schema_revision"],"testonly-1394-v2-half-import-binding/1");
        let case = &binding["case"];
        assert_eq!(case["case_id"],"PrepareDecision-half-import-before_commit-0");
        // Its permanent fault message is never sent; no synthetic pause/fault drives this case.
        let mut f = V2Fixture::new(&binding,case,"NeverHalfImportFault","unreachable","before_commit");
        let previous_total = responsibility_total(&latest(&f.dbs[0]).unwrap().3);
        assert_eq!(previous_total,20);
        let mut cache_removal = Vec::new();
        for n in 0..f.dbs.len() {
            let before = latest(&f.dbs[n]).unwrap();
            assert!(super::super::super::super::content_store::image_refs(&before.3).unwrap().is_empty(),
                "test cache may be removed only when all committed history has no S refs");
            assert_eq!(node_count(&f.dbs[n],"content_nodes"),1307);
            assert_eq!(node_count(&f.dbs[n],"content_staging"),0);
            let removed = f.dbs[n].execute("DELETE FROM content_nodes",[]).unwrap();
            assert_eq!(removed,1307);
            f.dbs[n].execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
            assert_eq!(latest(&f.dbs[n]).unwrap(),before);
            audit(&f.dbs[n],&f.configs[n]).unwrap();
            cache_removal.push(json!({"domain":f.configs[n].domain,"removed_unreferenced_setup_nodes":removed,
                "published_history_unchanged":true,"files":artifact_files(&f.configs[n].db)}));
        }
        create_json(&f.dir,"half-import-cache-precondition.json",&json!({"owners":cache_removal,
            "scope":"only this fixture's NEW carriers; not original four owners or matrix cases"}));
        let request = f.request("PrepareDecision","alpha");
        assert_ne!(request["message_id"],f.configs[1].manifest["test_only_fault"]["message_id"]);
        let original = latest(&f.dbs[1]).unwrap();
        assert!(!original.3["book"].is_null());
        assert!(original.3["finance"]["decisions"].as_array().unwrap().is_empty());
        let job = f.job(1,&request);
        let mut writer = ChildGuard::new(&f,&job,"half-import-write");
        let pid = writer.child.id();
        let start = Instant::now();
        let deadline = start + Duration::from_secs(180);
        let observed = loop {
            if let Some(status) = writer.child.try_wait().unwrap() {
                writer.record_exit(status,"stage window not reached: writer exited");
                create_json(&f.dir,"half-import-window-not-reached.json",&json!({"pid":pid,
                    "reason":"writer exited before durable stage observation","exit_code":status.code(),
                    "signal":status.signal(),"acceptance_pass":false}));
                panic!("real staging window not reached; preserve case and do not retry it");
            }
            // One SELECT gives a consistent statement snapshot. Connection/statement die
            // each iteration; no read transaction spans requests, SIGKILL or recovery.
            let sampled = {
                let probe = Connection::open_with_flags(&f.configs[1].db,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
                probe.busy_timeout(Duration::from_millis(50)).unwrap();
                probe.query_row("SELECT (SELECT count(*) FROM content_staging),
                    (SELECT coalesce(sum(length(body)),0) FROM content_staging),
                    (SELECT max(seq) FROM commits)",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?))).unwrap()
            };
            if sampled.0>0 {
                assert_eq!(sampled.2,original.0,"already committed; not a half-import witness");
                assert!(writer.child.try_wait().unwrap().is_none(),"writer must remain live");
                break json!({"pid":pid,"elapsed_ms":start.elapsed().as_millis().to_string(),
                    "unix_observed_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string(),
                    "staging_rows":sampled.0,"staging_body_bytes":sampled.1,"commit_seq":sampled.2,
                    "files":artifact_files(&f.configs[1].db),"writer_alive":true});
            }
            if Instant::now()>=deadline {
                create_json(&f.dir,"half-import-window-not-reached.json",&json!({"pid":pid,
                    "reason":"controller observation deadline","last_sample":sampled,"acceptance_pass":false}));
                panic!("real staging window deadline; preserve case and do not restart it");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        // Capture evidence in memory, kill immediately, then persist it: logging must
        // not extend the race into a successful economic COMMIT.
        let killed = writer.kill_observed();
        assert_eq!(killed.signal(),Some(9));
        create_json(&f.dir,"half-import-observation.json",&observed);
        assert_eq!(latest(&f.dbs[1]).unwrap(),original,"COMMIT raced the kill; no half-import pass");
        let persisted_stage = node_count(&f.dbs[1],"content_staging");
        assert!(persisted_stage>0,"staging must survive actual SIGKILL before recover");
        assert_eq!(node_count(&f.dbs[1],"content_nodes"),0);
        assert!(row(&f.dbs[1],"receipts",&key_for(&f.configs[1],&request["payload"]).unwrap()).unwrap().is_none());
        create_json(&f.dir,"half-import-killed-state.json",&json!({"pid":pid,"signal":9,
            "stage_rows":persisted_stage,"commit_seq":original.0,"files":artifact_files(&f.configs[1].db)}));
        let structure = &f.configs[1].manifest["finance"]["structure"];
        let source_db = PathBuf::from(required(structure,"db").unwrap());
        let source_binary = PathBuf::from(required(structure,"binary").unwrap());
        let offline_db = source_db.with_extension("offline");
        let offline_binary = source_binary.with_extension("offline");
        assert!(!offline_db.exists() && !offline_binary.exists());
        std::fs::rename(&source_db,&offline_db).unwrap();
        std::fs::rename(&source_binary,&offline_binary).unwrap();
        create_json(&f.dir,"half-import-source-offline.json",&json!({"db":source_db,"offline_db":offline_db,
            "binary":source_binary,"offline_binary":offline_binary,"scope":"this case's S copies only"}));
        let mut recovery_job = job.clone(); recovery_job["action"] = json!("recover");
        let mut recovery = ChildGuard::new(&f,&recovery_job,"half-import-recover");
        let recovery_pid = recovery.child.id(); assert_ne!(pid,recovery_pid);
        recovery.wait_success();
        assert_eq!(meta(&f.dbs[1],"writer_epoch").unwrap(),"2");
        assert_eq!(node_count(&f.dbs[1],"content_staging"),0);
        assert_eq!(node_count(&f.dbs[1],"content_nodes"),0);
        assert_eq!(latest(&f.dbs[1]).unwrap(),original);
        audit(&f.dbs[1],&f.configs[1]).unwrap();
        let stale = write(&mut f.dbs[1],&f.configs[1],2,&request).unwrap_err();
        assert!(stale.contains("StaleWriter"),"{stale}");
        assert_eq!(latest(&f.dbs[1]).unwrap(),original);
        assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3),20);
        pin(&f.manifest,&f.manifest_sha);
        let live_sources = case_processes(pid,&source_binary).into_iter().filter(|(_,s,_)|*s!='Z').count();
        assert_eq!(live_sources,0,"this case must leave no live S reader");
        create_json(&f.dir,"half-import-result.json",&json!({"case_id":case["case_id"],
            "binding_path":bp,"binding_sha256":bh,"manifest_path":f.manifest,"manifest_sha256":f.manifest_sha,
            "writer_pid":pid,"recovery_pid":recovery_pid,"signal":9,"stage_rows_after_kill":persisted_stage,
            "stage_rows_after_fresh_recover":0,"published_node_count":0,"commit_seq":original.0,
            "old_image_unchanged":true,"prepared_or_grant_created":false,"retained_liability":20,
            "stale_original_request_error":stale,"owner_children_reaped":writer.reaped && recovery.reaped,
            "live_case_source_processes":live_sources,
            "proof_domain":"Rust test-child + real cold S3 + actual SIGKILL; NOT formal CLI or complete matrix",
            "actual_half_import_cleanup_pass":true}));
    }
}
