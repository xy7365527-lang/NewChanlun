// #1481 S1L：旧写入口必须在只读前缀拒绝 v3，不能先转 WAL 或授予补列资格。
// 由 construction_tests 在 test+s_incremental+unix 下 include。

fn h1_legacy_prepolicy_mainonly() -> H1AdmissionFixture {
    let f = H1AdmissionFixture::new();
    h1_admission_seed_three(&f);
    // 留下一份真实冷重建的受管台账缓存，负例必须连它一起保持原字节。
    drop(h1_admission_driver(&f, "1").unwrap());
    assert!(h1_admission_ledger_path(&f.db).is_file());
    f.conn()
        .execute_batch(
            "ALTER TABLE structure_deltas DROP COLUMN policy_records; PRAGMA journal_mode=DELETE",
        )
        .unwrap();
    h1_admission_assert_clean_close(&f.db);
    h1_legacy_assert_prepolicy_delete(&f.db);
    f
}

fn h1_legacy_assert_prepolicy_delete(db: &Path) {
    let conn = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(crate::policy_records_missing(&conn).unwrap());
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "delete");
}

fn h1_legacy_all_source_bytes(db: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    [
        "",
        "-wal",
        "-shm",
        "-journal",
        ".audit-ledger.sqlite",
        ".audit-ledger.sqlite-wal",
        ".audit-ledger.sqlite-shm",
        ".audit-ledger.sqlite-journal",
    ]
    .into_iter()
    .map(|suffix| {
        let mut path = db.as_os_str().to_os_string();
        path.push(suffix);
        let bytes = match std::fs::read(Path::new(&path)) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("读取源及台账旁挂失败：{suffix}：{error}"),
        };
        (suffix.to_string(), bytes)
    })
    .collect()
}

#[cfg(test)]
fn h1_legacy_cli_args(f: &H1AdmissionFixture, command: &str) -> Vec<String> {
    // 发布真实控制原件，再让命令自己调用 from_args_for_db；不复用测试预审 scope。
    drop(super::super::admission_anchor::test_prepare(&f.db));
    let mut owner_path = f.db.as_os_str().to_os_string();
    owner_path.push(".control-owner.json");
    let owner: Value =
        serde_json::from_slice(&std::fs::read(Path::new(&owner_path)).unwrap()).unwrap();
    vec![
        "s_structure_session".into(),
        command.into(),
        "--db".into(),
        f.db.to_string_lossy().into_owned(),
        "--admission-job-limits".into(),
        owner["admission"]["path"].as_str().unwrap().into(),
    ]
}

#[cfg(test)]
fn h1_legacy_write_rejects_before_migration(command: &str, clock_recover: bool) {
    let f = h1_legacy_prepolicy_mainonly();
    h1_legacy_assert_write_rejected(&f, command, clock_recover);
}

#[cfg(test)]
fn h1_legacy_assert_write_rejected(f: &H1AdmissionFixture, command: &str, clock_recover: bool) {
    let mut args = h1_legacy_cli_args(f, command);
    match command {
        "recover" => args.extend(["--new-epoch".to_string(), "2".to_string()]),
        "accept" => {
            let input = f.db.with_extension("input.json");
            std::fs::write(&input, canonical_json(&b2_envelope(4))).unwrap();
            args.extend([
                "--input".into(),
                input.to_string_lossy().into_owned(),
                "--profile".into(),
                f.profile.to_string_lossy().into_owned(),
                "--writer-epoch".into(),
                "1".into(),
            ]);
        }
        "advance" => args.extend(["--writer-epoch".to_string(), "1".to_string()]),
        _ => unreachable!(),
    }
    if clock_recover {
        args.extend([
            "--clock-plan".into(),
            f.db.with_extension("clock.json")
                .to_string_lossy()
                .into_owned(),
            "--clock-event-id".into(),
            "recover-1".into(),
        ]);
    }
    let before_bytes = h1_legacy_all_source_bytes(&f.db);
    assert!(before_bytes[0].1.is_some());
    assert!(before_bytes[1..4].iter().all(|(_, bytes)| bytes.is_none()));
    assert!(before_bytes[4].1.is_some());
    let before_calls = state::full_audit_calls_for_test();
    // 与根 run 完全相同的入口分派前缀；任何成功 scope 都会使测试失败。
    assert!(!super::super::dispatch_fixture(&args).unwrap());
    let error = if clock_recover {
        v2::dispatch_fixture(&args)
            .err()
            .expect("v2 recover 必须在只读前缀拒绝 v3")
    } else {
        assert!(!v2::dispatch_fixture(&args).unwrap());
        v2::legacy_write_lock(&args)
            .err()
            .expect("v1 写入口必须在只读前缀拒绝 v3")
    };
    assert!(error.starts_with("VersionMismatch："), "{error}");
    assert!(error.contains("--protocol-revision s-session/3"), "{error}");
    assert_eq!(state::full_audit_calls_for_test(), before_calls);
    assert_eq!(h1_legacy_all_source_bytes(&f.db), before_bytes);
    h1_legacy_assert_prepolicy_delete(&f.db);
    assert_eq!(h1_legacy_all_source_bytes(&f.db), before_bytes);
}

#[cfg(test)]
#[test]
fn h1_admission_legacy_v1_recover_prepolicy_mainonly_rejects_unchanged() {
    h1_legacy_write_rejects_before_migration("recover", false);
}

#[cfg(test)]
#[test]
fn h1_admission_legacy_v1_accept_prepolicy_mainonly_rejects_unchanged() {
    h1_legacy_write_rejects_before_migration("accept", false);
}

#[cfg(test)]
#[test]
fn h1_admission_legacy_v1_advance_prepolicy_mainonly_rejects_unchanged() {
    h1_legacy_write_rejects_before_migration("advance", false);
}

#[cfg(test)]
#[test]
fn h1_admission_legacy_v2_clock_recover_prepolicy_mainonly_rejects_unchanged() {
    h1_legacy_write_rejects_before_migration("recover", true);
}

#[cfg(test)]
#[test]
fn h1_admission_v3_selector_advance_prepolicy_mainonly_authenticates_before_migration() {
    const CHILD: &str = "H1_ADMISSION_LEGACY_SELECTOR_ARGS";
    if let Some(args) = std::env::var_os(CHILD) {
        let args: Vec<String> = serde_json::from_str(args.to_str().unwrap()).unwrap();
        let db = PathBuf::from(
            args.windows(2)
                .find(|pair| pair[0] == "--db")
                .map(|pair| pair[1].as_str())
                .unwrap(),
        );
        h1_legacy_assert_prepolicy_delete(&db);
        let before_calls = state::full_audit_calls_for_test();
        let (result, events, _) = state::traced(false, || super::super::dispatch_fixture(&args));
        assert!(result.unwrap(), "显式选择器必须走 v3 advance");
        assert_eq!(state::full_audit_calls_for_test(), before_calls);
        // 冷认证逐代消费 g1..3，随后输入真实提交 g4 并推进同一资格步。
        assert_eq!(h1_qualified_cuts(&events), vec![1, 2, 3, 4]);
        h1_admission_assert_clean_close(&db);
        // #1481 S1N：WAL 上的只读检查连接也会重建旁挂，且无法在最后关闭时清理。
        // 沿用生产成功路径的普通读写关闭语义；这里只查询，不修改库内容。
        let conn =
            Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        assert!(!crate::policy_records_missing(&conn).unwrap());
        assert_eq!(meta_i64(&conn, "generation").unwrap(), 4);
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        drop(conn);
        h1_admission_assert_clean_close(&db);
        return;
    }
    let f = h1_legacy_prepolicy_mainonly();
    // 同一物理库先经四种旧入口逐一只读拒绝，再由显式 v3 入口认证并成功迁移。
    for (command, clock_recover) in [
        ("recover", false),
        ("accept", false),
        ("advance", false),
        ("recover", true),
    ] {
        h1_legacy_assert_write_rejected(&f, command, clock_recover);
    }
    let mut args = h1_legacy_cli_args(&f, "advance");
    let resources = f.db.with_extension("resources.json");
    let gates: serde_json::Map<String, Value> = super::super::writer_cli::GATE_FIELDS
        .into_iter()
        .map(|field| (field.to_string(), json!("536870912")))
        .collect();
    std::fs::write(
        &resources,
        canonical_json(&json!({"schema_revision":"s-structure-resources/2","s_resources":gates})),
    )
    .unwrap();
    let input = f.db.with_extension("input.json");
    std::fs::write(&input, canonical_json(&b2_envelope(4))).unwrap();
    args.extend([
        "--protocol-revision".into(),
        "s-session/3".into(),
        "--profile".into(),
        f.profile.to_string_lossy().into_owned(),
        "--writer-epoch".into(),
        "1".into(),
        "--clock-plan".into(),
        f.db.with_extension("clock.json")
            .to_string_lossy()
            .into_owned(),
        "--resource-config".into(),
        resources.to_string_lossy().into_owned(),
        "--input".into(),
        input.to_string_lossy().into_owned(),
        "--max-input-bytes".into(),
        "1048576".into(),
    ]);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "v3::audit_owner::construction_tests::h1_admission_v3_selector_advance_prepolicy_mainonly_authenticates_before_migration", "--nocapture"])
        .env(CHILD, serde_json::to_string(&args).unwrap());
    // 独立进程冻结 native 对照模式，避免污染其他并行测试的全局环境。
    #[cfg(feature = "s_capture_native_testonly")]
    child.env("S_CAPTURE_NATIVE_TESTONLY", "1");
    let output = child.output().unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running 1 test"), "{stdout}");
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
    h1_admission_assert_clean_close(&f.db);
}
