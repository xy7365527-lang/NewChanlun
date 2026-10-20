// Insert this module inside an isolated copy of v2_fault_acceptance, never product.
// SQLite hooks observe final rows and exhaust only this case's own carrier.
#[cfg(target_os = "macos")]
mod final_enospc_acceptance {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;
    use std::sync::{Arc, Mutex};
    #[derive(Default)]
    struct Observed {
        updates: Vec<Value>,
        hooks: Vec<Value>,
        pressure_triggered: bool,
    }
    fn free_bytes(path: &Path) -> u64 {
        use std::os::unix::ffi::OsStrExt;
        let p = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        let mut v = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        assert_eq!(unsafe { libc::statvfs(p.as_ptr(), v.as_mut_ptr()) }, 0);
        let v = unsafe { v.assume_init() };
        u64::try_from(u128::from(v.f_bavail) * u128::from(v.f_frsize)).unwrap()
    }
    #[test]
    #[ignore = "new dedicated frozen HFS carriers only; actual final-COMMIT ENOSPC"]
    fn actual_os_enospc_at_final_economic_commit_rolls_back_atomic_group() {
        let binding_path = PathBuf::from(std::env::var("EB_FINAL_ENOSPC_BINDING").unwrap());
        let binding_sha = std::env::var("EB_FINAL_ENOSPC_BINDING_SHA256").unwrap();
        let binding: Value = strict_json(&pin(&binding_path, &binding_sha)).unwrap();
        assert_eq!(
            binding["schema_revision"],
            "testonly-1394-v2-fault-binding/1"
        );
        assert_eq!(binding["cases"].as_array().unwrap().len(), 1);
        let case = &binding["cases"][0];
        assert_eq!(case["case_id"], "PrepareDecision-final-enospc-0");
        // The manifest's unused sentinel is never sent; no product pause is triggered.
        let mut f = V2Fixture::new(
            &binding,
            case,
            "NeverFinalEnospcFault",
            "unreachable",
            "before_commit",
        );
        let config = f.configs[1].clone();
        let db = config.db.clone();
        let mount = db.parent().unwrap().to_owned();
        let pressure = mount.join("this-case-only-final-commit-pressure.bin");
        assert!(!pressure.exists());
        let before = latest(&f.dbs[1]).unwrap();
        let e_before = latest(&f.dbs[0]).unwrap();
        assert_eq!(responsibility_total(&e_before.3), 20);
        assert!(!before.3["book"].is_null());
        assert_eq!(node_count(&f.dbs[1], "content_nodes"), 1307);
        assert_eq!(node_count(&f.dbs[1], "content_staging"), 0);
        let request = f.request("PrepareDecision", "alpha");
        let key = key_for(&config, &request["payload"]).unwrap();
        assert!(row(&f.dbs[1], "receipts", &key).unwrap().is_none());
        let messages_before: i64 = f.dbs[1]
            .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        let pub_before: i64 = node_count(&f.dbs[1], "content_nodes");
        let capacity = super::super::super::super::content_store::carrier_capacity(&db).unwrap();
        let allocated_before = super::super::super::super::content_store::allocated(&db).unwrap();
        create_json(
            &f.dir,
            "final-enospc-before.json",
            &json!({"latest":before,"e_latest":e_before,
            "public_nodes":pub_before,"messages":messages_before,"allocated_bytes":allocated_before,
            "free_bytes":free_bytes(&mount),"capacity":capacity,
            "reader_resources":config.manifest["finance"]["structure"]["reader_resources"],
            "response_timeout_ms":config.manifest["resources"]["response_timeout_ms"],
            "manifest":f.manifest,"manifest_sha256":f.manifest_sha,"binding_sha256":binding_sha}),
        );
        let observed = Arc::new(Mutex::new(Observed::default()));
        let updates = Arc::clone(&observed);
        f.dbs[1].update_hook(Some(move|action:rusqlite::hooks::Action,database:&str,table:&str,rowid:i64| {
            updates.lock().unwrap().updates.push(json!({"action":action as i32,"database":database,"table":table,"rowid":rowid}));
        })).unwrap();
        let state = Arc::clone(&observed);
        let hook_db = db.clone();
        let hook_pressure = pressure.clone();
        let hook_mount = mount.clone();
        let expected_before = before.clone();
        f.dbs[1].commit_hook(Some(move|| {
            let mut s=state.lock().unwrap();
            let call=s.hooks.len()+1;
            if s.pressure_triggered {s.hooks.push(json!({"call":call,"pressure_repeated":false,"returned_rollback":false}));return false;}
            let commits=s.updates.iter().any(|u|u["database"]=="main"&&u["table"]=="commits"&&u["action"]==rusqlite::ffi::SQLITE_INSERT&&u["rowid"]==expected_before.0+1);
            let receipt=s.updates.iter().any(|u|u["database"]=="main"&&u["table"]=="receipts"&&u["action"]==rusqlite::ffi::SQLITE_INSERT);
            let message=s.updates.iter().any(|u|u["database"]=="main"&&u["table"]=="messages"&&u["action"]==rusqlite::ffi::SQLITE_INSERT);
            // This is a different, read-only connection, dropped before pressure/COMMIT.
            let visible={
                let p=Connection::open_with_flags(&hook_db,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
                p.busy_timeout(std::time::Duration::from_millis(100)).unwrap();latest(&p).unwrap()
            };
            let final_group=commits&&receipt&&message&&visible==expected_before;
            if !final_group {s.hooks.push(json!({"call":call,"final_group":false,"commits_next_seen":commits,
                "receipts_seen":receipt,"messages_seen":message,"visible_latest":visible,
                "returned_rollback":false,"pressure_created":false}));return false;}
            s.pressure_triggered=true;
            let free_before=free_bytes(&hook_mount);let mut count=0u64;let mut errno=None;let mut io_message=None;
            let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&hook_pressure).unwrap();
            let chunk=vec![0xa5u8;65536];
            while count<=capacity {
                match file.write(&chunk) {
                    Ok(0)=>{io_message=Some("real write returned zero".to_owned());break;},
                    Ok(n)=>count+=u64::try_from(n).unwrap(),
                    Err(error)=>{errno=error.raw_os_error();io_message=Some(error.to_string());break;}
                }
            }
            let sync=file.sync_all().err().map(|e|json!({"errno":e.raw_os_error(),"message":e.to_string()}));
            let meta=file.metadata().unwrap();drop(file);
            s.hooks.push(json!({"call":call,"final_group":true,"commits_next_seen":commits,"receipts_seen":receipt,
                "messages_seen":message,"visible_latest":visible,"pressure_bytes_written":count,"pressure_file_bytes":meta.len(),
                "pressure_allocated_bytes":meta.blocks()*512,"free_before":free_before,"free_after":free_bytes(&hook_mount),
                "actual_write_errno":errno,"actual_write_error":io_message,"sync_error":sync,"returned_rollback":false}));
            false // never fabricate rollback; let SQLite do its normal physical COMMIT.
        })).unwrap();
        let result = write(&mut f.dbs[1], &config, 1, &request);
        let primary = unsafe { rusqlite::ffi::sqlite3_errcode(f.dbs[1].handle()) };
        let extended = unsafe { rusqlite::ffi::sqlite3_extended_errcode(f.dbs[1].handle()) };
        let autocommit = f.dbs[1].is_autocommit();
        let snapshot = {
            let s = observed.lock().unwrap();
            json!({"updates":s.updates,"hooks":s.hooks,"pressure_triggered":s.pressure_triggered})
        };
        // Record actual return before assertions or cleanup; a mis-trigger stays INCONCLUSIVE.
        create_json(
            &f.dir,
            "final-enospc-observed.json",
            &json!({"request":request,"product_result":result,
            "sqlite_primary_after_product_return":primary,"sqlite_extended_after_product_return":extended,
            "autocommit_after_return":autocommit,"observation":snapshot,"sqlite_full_code_constant":rusqlite::ffi::SQLITE_FULL,
            "scope":"actual OS pressure at observed final COMMIT; no hook-induced rollback"}),
        );
        f.dbs[1].commit_hook::<fn() -> bool>(None).unwrap();
        f.dbs[1]
            .update_hook::<fn(rusqlite::hooks::Action, &str, &str, i64)>(None)
            .unwrap();
        if pressure.exists() {
            std::fs::remove_file(&pressure).unwrap();
        }
        create_json(
            &f.dir,
            "final-enospc-pressure-released.json",
            &json!({"pressure_absent":!pressure.exists(),
            "free_bytes":free_bytes(&mount),"allocated_bytes":super::super::super::super::content_store::allocated(&db).unwrap(),"hooks_cleared":true}),
        );
        assert_eq!(
            snapshot["hooks"][0]["final_group"], true,
            "INCONCLUSIVE: hook before final atomic group"
        );
        assert_eq!(
            snapshot["hooks"][0]["actual_write_errno"], 28,
            "INCONCLUSIVE: no real OS ENOSPC"
        );
        assert_eq!(snapshot["hooks"][0]["returned_rollback"], false);
        let error = result.expect_err("INCONCLUSIVE: physical COMMIT did not fail");
        assert!(
            error.contains("database or disk is full"),
            "INCONCLUSIVE: not SQLite FULL: {error}"
        );
        assert!(
            autocommit,
            "product failed without ending its economic transaction"
        );
        assert_eq!(
            latest(&f.dbs[1]).unwrap(),
            before,
            "BUG: failed COMMIT changed original image"
        );
        assert_eq!(
            latest(&f.dbs[0]).unwrap(),
            e_before,
            "BUG: public E liability changed"
        );
        assert!(row(&f.dbs[1], "receipts", &key).unwrap().is_none());
        assert_eq!(
            f.dbs[1]
                .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            messages_before
        );
        assert_eq!(
            node_count(&f.dbs[1], "content_nodes"),
            pub_before,
            "BUG: published history deleted"
        );
        assert_eq!(
            node_count(&f.dbs[1], "content_staging"),
            0,
            "BUG: stage leakage"
        );
        assert!(latest(&f.dbs[1]).unwrap().3["finance"]["decisions"]
            .as_array()
            .unwrap()
            .is_empty());
        for (c, config) in f.dbs.iter().zip(&f.configs) {
            audit(c, config).unwrap();
        }
        drop(f.dbs);
        let recovery = recover(&config).unwrap();
        assert_eq!(recovery["writer_epoch"], "2");
        let (c, _) = open(&db, false).unwrap();
        audit(&c, &config).unwrap();
        assert_eq!(latest(&c).unwrap(), before);
        assert_eq!(node_count(&c, "content_staging"), 0);
        pin(&f.manifest, &f.manifest_sha);
        create_json(
            &f.dir,
            "final-enospc-result.json",
            &json!({"actual_final_enospc_pass":true,"actual_os_errno":28,
            "actual_sql_error":error,"sqlite_primary_after_product_return":primary,"sqlite_extended_after_product_return":extended,
            "hook_first_final_atomic_group":true,"hook_rollbacks_requested":false,"hook_calls":snapshot["hooks"].as_array().unwrap().len(),
            "before_seq":before.0,"after_seq":latest(&c).unwrap().0,"old_image_root_unchanged":true,
            "original_receipt_absent":true,"messages_unchanged":true,"prepared_or_grant_created":false,"retained_liability":20,
            "published_nodes_retained":pub_before,"staging_zero":true,"recover_writer_epoch":"2","manifest_sha256":f.manifest_sha,
            "binding_sha256":binding_sha,"capacity":capacity,"reader_resources":config.manifest["finance"]["structure"]["reader_resources"],
            "scope":"one real S3 cached Prepare, observed final COMMIT hit OS ENOSPC; not all resources/CLI/goal"}),
        );
    }
}
