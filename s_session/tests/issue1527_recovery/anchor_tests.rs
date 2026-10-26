//! #1527 Rust 源码测试；本轮 NOT_RUN，须在根协调模块接线后具名构建执行。
use super::*;
use serde_json::json;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    db: PathBuf,
    state: PathBuf,
    owner: PathBuf,
    limits: PathBuf,
    binding: Value,
    raw: Vec<u8>,
}

fn identity(path: &Path) -> Value {
    let meta = std::fs::metadata(path).unwrap();
    json!({"device": meta.dev(), "inode": meta.ino(), "uid": meta.uid(), "mode": meta.mode() & 0o7777})
}

fn write_file(path: &Path, bytes: &[u8], mode: u32) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(mode))
        .unwrap();
}

fn limit_node() -> Value {
    // TestOnly 小文件预算，不是部署默认或原生产帽的替代。
    json!({
        "max_total_wire_bytes":"1048576", "max_total_node_bytes":"1",
        "max_total_nodes":"1", "max_total_records":"1", "max_total_requests":"1",
        "max_elapsed_ms":"60000", "max_staged_bytes":"1", "max_store_bytes":"1"
    })
}

impl Fixture {
    fn new() -> Self {
        Self::with_raw(serde_json::to_vec(&limit_node()).unwrap())
    }

    fn with_raw(raw: Vec<u8>) -> Self {
        let root = std::env::temp_dir().join(format!(
            "issue1527-anchor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let db = root.join("db");
        let state = root.join("state");
        write_file(&db, "TestOnly普通文件，不打开SQLite".as_bytes(), 0o600);
        std::fs::create_dir(&state).unwrap();
        // state 正常较宽权限；只有 s-admission 必须 0700。
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755)).unwrap();
        let published = state.join(PUBLISHED);
        std::fs::create_dir(&published).unwrap();
        std::fs::set_permissions(&published, std::fs::Permissions::from_mode(0o700)).unwrap();
        let limits = published.join(LIMITS_NAME);
        let binding_path = published.join(BINDING_NAME);
        write_file(&limits, &raw, 0o600);
        write_file(&binding_path, b"", 0o600);
        let binding = json!({
            "db":db, "path":limits, "sha256":sha256_hex(&raw),
            "file_identity":identity(&limits), "binding_identity":identity(&binding_path),
            "directory_identity":identity(&published)
        });
        std::fs::write(&binding_path, serde_json::to_vec(&binding).unwrap()).unwrap();
        let owner = root.join("db.control-owner.json");
        let wire = json!({"db":db,"state_dir":state,"admission":binding});
        write_file(&owner, &serde_json::to_vec(&wire).unwrap(), 0o600);
        Self {
            root,
            db,
            state,
            owner,
            limits,
            binding,
            raw,
        }
    }

    fn load(&self) -> Result<PreparedAdmission, AdmissionError> {
        load_for_db(&self.db, &self.limits, ControlEnv::from_values(None, None))
    }

    fn env(&self) -> ControlEnv {
        ControlEnv::from_values(
            Some(OsString::from(self.binding["sha256"].as_str().unwrap())),
            Some(OsString::from(
                serde_json::to_string(&self.binding).unwrap(),
            )),
        )
    }

    fn total(&self) -> u64 {
        [
            &self.owner,
            &self.limits,
            &self.state.join(PUBLISHED).join(BINDING_NAME),
        ]
        .iter()
        .map(|path| std::fs::metadata(path).unwrap().len())
        .sum()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn valid_disk_anchor_succeeds_with_or_without_both_env_and_exact_wire() {
    let f = Fixture::new();
    let before = Instant::now();
    let ready = f.load().unwrap();
    assert!(ready.prelude.first_read_at.unwrap() >= before);
    assert_eq!(ready.prelude.wire_bytes, f.total());
    assert_eq!(
        ready.limits,
        JobLimits::from_canonical_json(&limit_node()).unwrap()
    );
    let consumed = ready.consume().unwrap();
    consumed.check(Stage::E8).unwrap();
    assert_eq!(consumed.lease().db_path(), f.db);
    assert_eq!(consumed.prelude.wire_bytes, f.total());
    load_for_db(&f.db, &f.limits, f.env()).unwrap();
    std::fs::set_permissions(&f.owner, std::fs::Permissions::from_mode(0o644)).unwrap();
    f.load().unwrap();
}

#[test]
fn arc_keeps_original_fds_after_prepared_is_moved_and_last_owner_drop_releases_them() {
    let f = Fixture::new();
    let ready = f.load().unwrap();
    let weak = Arc::downgrade(&ready.lease);
    let consumed = ready.consume().unwrap();
    let lease = Arc::clone(&consumed.lease);
    drop(consumed);
    assert!(weak.upgrade().is_some());
    lease.revalidate_paths().unwrap();
    drop(lease);
    assert!(weak.upgrade().is_none());
}

#[test]
fn same_bytes_replacement_of_each_pinned_object_is_rejected_without_adopting_it() {
    for kind in [
        "owner",
        "binding",
        "limits",
        "published",
        "state",
        "db",
        "parent",
    ] {
        let f = Fixture::new();
        let ready = f.load().unwrap();
        let path = match kind {
            "owner" => f.owner.clone(),
            "binding" => f.state.join(PUBLISHED).join(BINDING_NAME),
            "limits" => f.limits.clone(),
            "published" => f.state.join(PUBLISHED),
            "state" => f.state.clone(),
            "db" => f.db.clone(),
            _ => f.root.clone(),
        };
        let old = path.with_extension("original");
        std::fs::rename(&path, &old).unwrap();
        if old.is_dir() {
            std::fs::create_dir(&path).unwrap();
        } else {
            write_file(&path, &std::fs::read(&old).unwrap(), 0o600);
        }
        let error = ready.consume().err().unwrap();
        assert_eq!(error.kind, ErrorKind::Changed, "{kind}");
        assert!(error.wire_bytes > 0);
        if old.is_dir() {
            std::fs::remove_dir(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        std::fs::rename(old, path).unwrap();
    }
}

#[test]
fn no_env_cannot_skip_missing_owner_pending_or_lost_publication() {
    for action in ["owner", "pending", "lost", "pending_only", "unanchored"] {
        let f = Fixture::new();
        match action {
            "owner" => std::fs::remove_file(&f.owner).unwrap(),
            "pending" => std::fs::create_dir(f.state.join(PENDING)).unwrap(),
            "lost" => std::fs::rename(f.state.join(PUBLISHED), f.state.join("retained")).unwrap(),
            "pending_only" => {
                std::fs::rename(f.state.join(PUBLISHED), f.state.join(PENDING)).unwrap()
            }
            _ => std::fs::write(
                &f.owner,
                serde_json::to_vec(&json!({"db":f.db,"state_dir":f.state})).unwrap(),
            )
            .unwrap(),
        }
        let expected = match action {
            "owner" => ErrorKind::AnchorMissing,
            "pending" | "pending_only" => ErrorKind::AnchorUnpublished,
            "lost" => ErrorKind::AnchorLost,
            _ => ErrorKind::AnchorInvalid,
        };
        assert_eq!(f.load().unwrap_err().kind, expected);
        assert_eq!(f.load().unwrap_err().kind, expected, "重试不得重签");
    }
}

#[test]
fn env_shape_and_real_db_identity_are_checked_without_using_env_as_locator() {
    let f = Fixture::new();
    let other = Fixture::new();
    for env in [
        ControlEnv::from_values(Some("a".repeat(64).into()), None),
        ControlEnv::from_values(None, Some("{}".into())),
        ControlEnv::from_values(Some("A".repeat(64).into()), Some("{}".into())),
        ControlEnv::from_values(Some("".into()), Some("".into())),
    ] {
        let error = load_for_db(&f.db, &f.limits, env).unwrap_err();
        assert_eq!(error.stage, Stage::E0);
        assert_eq!(error.wire_bytes, 0);
        assert!(error.first_read_at.is_none());
    }
    assert_eq!(
        load_for_db(&f.db, &f.limits, other.env()).unwrap_err().kind,
        ErrorKind::ControlMismatch
    );
    // 复制别库 owner 到实际 db 旁仍失败，不得按 env 转向另一个根。
    std::fs::write(&f.owner, std::fs::read(&other.owner).unwrap()).unwrap();
    assert_eq!(
        load_for_db(&f.db, &f.limits, other.env()).unwrap_err().kind,
        ErrorKind::Identity
    );
}

#[test]
fn unsafe_file_kinds_and_permissions_reject_before_payload_read() {
    for kind in ["symlink", "fifo", "hardlink", "mode"] {
        let f = Fixture::new();
        let old = f.owner.with_extension("saved");
        std::fs::rename(&f.owner, &old).unwrap();
        match kind {
            "symlink" => symlink(&old, &f.owner).unwrap(),
            "hardlink" => std::fs::hard_link(&old, &f.owner).unwrap(),
            "mode" => write_file(&f.owner, &std::fs::read(old).unwrap(), 0o666),
            _ => {
                let name = c_name(f.owner.as_os_str()).unwrap();
                // SAFETY：临时测试路径有效；只创建 TestOnly FIFO，不发起生产 I/O。
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
        }
        let error = f.load().unwrap_err();
        assert_eq!(error.kind, ErrorKind::Changed, "{kind}");
        assert_eq!(error.wire_bytes, 0);
    }
}

#[test]
fn strict_schemas_reject_duplicates_noncanonical_limits_and_invalid_identity_integers() {
    for raw in [b"".to_vec(), b"{}".to_vec(), b"{\"x\":1,\"x\":2}".to_vec()] {
        let f = Fixture::with_raw(raw);
        assert_eq!(f.load().unwrap_err().kind, ErrorKind::ResourcesInvalid);
    }
    for value in [
        json!(0),
        json!(true),
        json!("0"),
        json!("01"),
        json!("-1"),
        json!("1.0"),
        json!("9223372036854775808"),
    ] {
        let mut node = limit_node();
        node["max_total_nodes"] = value;
        let f = Fixture::with_raw(serde_json::to_vec(&node).unwrap());
        assert_eq!(f.load().unwrap_err().kind, ErrorKind::ResourcesInvalid);
    }
    let f = Fixture::new();
    for bad in [
        json!(true),
        json!(-1),
        json!(1.0),
        json!("1"),
        json!(18446744073709551615u64),
    ] {
        let mut binding = f.binding.clone();
        binding["file_identity"]["uid"] = bad;
        let owner = json!({"db":f.db,"state_dir":f.state,"admission":binding});
        std::fs::write(&f.owner, serde_json::to_vec(&owner).unwrap()).unwrap();
        assert_eq!(f.load().unwrap_err().kind, ErrorKind::AnchorInvalid);
    }
}

#[test]
fn metadata_only_recheck_does_not_charge_and_detects_in_place_write_and_pending() {
    let f = Fixture::new();
    let ready = f.load().unwrap();
    let wire = ready.prelude.wire_bytes;
    check_lease(&ready.lease, &ready.prelude, Stage::E4).unwrap();
    assert_eq!(ready.prelude.wire_bytes, wire);
    std::fs::write(&f.limits, b"changed").unwrap();
    assert_eq!(ready.consume().err().unwrap().wire_bytes, wire);
    let f = Fixture::new();
    let ready = f.load().unwrap();
    symlink("missing", f.state.join(PENDING)).unwrap();
    assert!(ready.consume().is_err());
}

#[test]
fn partial_read_failure_and_65537th_byte_remain_in_the_error_account() {
    struct Partial {
        delivered: bool,
    }
    impl Read for Partial {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.delivered {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            self.delivered = true;
            out[..3].copy_from_slice(b"abc");
            Ok(3)
        }
    }
    let mut prelude = ControlPrelude::new();
    prelude.first_read_at = Some(Instant::now());
    let error =
        read_counted(&mut Partial { delivered: false }, &mut prelude, Stage::E1).unwrap_err();
    assert_eq!((error.kind, error.wire_bytes), (ErrorKind::ReadFailed, 3));
    assert!(!error.budget_trusted);
    let mut prelude = ControlPrelude::new();
    let error = read_counted(
        &mut &vec![b'x'; CONTROL_BYTES + 5][..],
        &mut prelude,
        Stage::E1,
    )
    .unwrap_err();
    assert_eq!((error.kind, error.wire_bytes), (ErrorKind::TooLarge, 65537));
}

#[test]
fn deadline_is_single_and_wins_when_wire_also_exceeds_without_fake_clock_in_production() {
    let t0 = Instant::now();
    let mut limits = JobLimits::from_canonical_json(&limit_node()).unwrap();
    limits.max_elapsed_ms = 10;
    limits.max_total_wire_bytes = 1;
    let prelude = ControlPrelude {
        first_read_at: Some(t0),
        wire_bytes: 2,
        trusted: Some(limits),
    };
    assert_eq!(
        prelude.gate_at(Stage::E4, t0).unwrap_err().kind,
        ErrorKind::Limit
    );
    let at = t0 + Duration::from_millis(10);
    for stage in [Stage::E4, Stage::E5, Stage::E7, Stage::E8] {
        let error = prelude.gate_at(stage, at).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Deadline);
        assert_eq!(error.first_read_at, Some(t0));
        assert_eq!(error.wire_bytes, 2);
    }
}

#[test]
fn duplicate_missing_or_alias_cli_budget_never_becomes_authority_and_errors_do_not_echo_input() {
    let f = Fixture::new();
    for args in [
        vec![],
        vec![LIMITS_FLAG],
        vec![LIMITS_FLAG, "secret-control-raw", LIMITS_FLAG, "again"],
        vec!["--admission-job-limits=secret"],
    ] {
        let args = args.into_iter().map(String::from).collect::<Vec<_>>();
        let error =
            from_args_for_db(&f.db, &args, ControlEnv::from_values(None, None)).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("secret"));
        assert_eq!(error.wire_bytes, 0);
    }
    let alias = f.root.join("same-bytes.json");
    write_file(&alias, &f.raw, 0o600);
    assert_eq!(
        load_for_db(&f.db, &alias, f.env()).unwrap_err().kind,
        ErrorKind::Identity
    );
}

fn start_gate_pipe() -> (File, File) {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    // SAFETY：pipe成功返回两个独占新fd，各转交一个File释放。
    unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) }
}
#[test]
fn start_gate_accepts_only_release_and_charges_original_wire() {
    for value in [Some(b'R'), Some(b'X'), None] {
        let f = Fixture::new();
        let mut prepared = f.load().unwrap();
        let first = prepared.prelude.first_read_at;
        let wire = prepared.prelude.wire_bytes;
        let (read, mut write) = start_gate_pipe();
        if let Some(byte) = value {
            write.write_all(&[byte]).unwrap();
        }
        drop(write);
        let result = prepared.wait_start_gate(read.as_raw_fd());
        assert_eq!(result.is_ok(), value == Some(b'R'));
        assert_eq!(prepared.prelude.first_read_at, first);
        assert_eq!(
            prepared.prelude.wire_bytes,
            wire + u64::from(value.is_some())
        );
    }
}

#[test]
fn start_gate_wait_consumes_original_deadline() {
    let mut node = limit_node();
    node["max_elapsed_ms"] = json!("80");
    let f = Fixture::with_raw(serde_json::to_vec(&node).unwrap());
    let mut prepared = f.load().unwrap();
    let (read, _write) = start_gate_pipe();
    let first = prepared.prelude.first_read_at;
    let error = prepared.wait_start_gate(read.as_raw_fd()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Deadline);
    assert_eq!(error.first_read_at, first);
}
#[test]
fn start_gate_does_not_accept_changed_anchor_or_nonpipe() {
    let f = Fixture::new();
    let mut prepared = f.load().unwrap();
    let (read, mut write) = start_gate_pipe();
    std::fs::write(&f.limits, b"changed").unwrap();
    write.write_all(b"R").unwrap();
    assert_eq!(
        prepared.wait_start_gate(read.as_raw_fd()).unwrap_err().kind,
        ErrorKind::Changed
    );
    let f = Fixture::new();
    let mut prepared = f.load().unwrap();
    let file = File::open(&f.db).unwrap();
    assert_eq!(
        prepared.wait_start_gate(file.as_raw_fd()).unwrap_err().kind,
        ErrorKind::ControlMismatch
    );
}

#[test]
fn mixed_resources_anchor_starts_same_job_and_closes_retained_parent() {
    use newchan_rust::s_content::{self as sc, job::CheckedNodeUsage};
    let mut limits = limit_node();
    limits["max_store_bytes"] = json!("1048576");
    limits["max_total_node_bytes"] = json!("1048576");
    limits["max_total_nodes"] = json!("2");
    let raw = serde_json::to_vec(&json!({
        "schema_revision":resources::MIXED_REVISION,
        "measurement_domains":["s-content/1","legacy-evidence/1"],
        "job_limits":limits,
        "native_ram":{"max_live_bytes":"1048576","max_operation_bytes":"1048576"}
    }))
    .unwrap();
    let fixture = Fixture::with_raw(raw);
    let prepared = fixture.load().unwrap();
    let first = prepared.prelude.first_read_at.unwrap();
    assert_eq!(prepared.prelude.wire_bytes, fixture.total());
    let consumed = prepared.consume().unwrap();
    let source = super::super::admission_provider::MeasuredDomain::measure(&consumed).unwrap();
    let mut started = consumed.into_job(source.existing).unwrap();
    assert_eq!(started.first_read_at, first);
    assert_eq!(
        started.accounting().snapshot().wire_bytes,
        fixture.total() as u128
    );
    let retained = started
        .with_accounting(|account| {
            let content = CheckedNodeUsage::checked(
                &sc::frame(sc::tag::BYTES_CHUNK, b"x"),
                sc::tag::BYTES_CHUNK,
                b"x",
            )?;
            let legacy = sc::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                &sc::legacy_evidence::node_id(0, b"x"),
                0,
                b"x",
                1,
            )
            .unwrap();
            account.accept_checked_usage(&content)?;
            account.accept_legacy_evidence(&legacy)?;
            account.charge_records(1)?;
            let grant = account.reserve_staged(0)?;
            account.release(&grant)?;
            assert_eq!(account.snapshot().nodes, 2);
            account
                .mixed_scope()
                .unwrap()
                .reserve_native(31)
                .map(|permit| permit.hold([0u8; 31]))
        })
        .unwrap();
    let finished = started.seal().unwrap();
    let ram = finished.ram.as_ref().unwrap();
    let before = ram.snapshot().unwrap();
    assert!(!before.active);
    assert!(ram.codec_work().is_err(), "seal后不能创建新分配域");
    drop(retained);
    assert_eq!(ram.snapshot().unwrap().job_live, before.job_live - 31);
    assert_eq!(finished.job.borrow().snapshot().nodes, 2);
}

#[test]
fn mixed_resources_fixed_job_layout_refusal_precedes_owner_publication() {
    let raw = serde_json::to_vec(&json!({
        "schema_revision":resources::MIXED_REVISION,
        "measurement_domains":["s-content/1","legacy-evidence/1"],
        "job_limits":limit_node(),
        "native_ram":{"max_live_bytes":"0","max_operation_bytes":"0"}
    }))
    .unwrap();
    let fixture = Fixture::with_raw(raw);
    let consumed = fixture.load().unwrap().consume().unwrap();
    let error = consumed
        .into_job(admission_job::Existing {
            store_bytes: 0,
            staged_bytes: 0,
        })
        .err()
        .unwrap();
    assert_eq!(error.kind, ErrorKind::Limit);
    assert_eq!(error.stage, Stage::E7);
    assert_eq!(error.wire_bytes, fixture.total());
}

#[test]
fn mixed_resources_anchor_ram_mutation_requires_new_signed_binding() {
    let raw = serde_json::to_vec(&json!({
        "schema_revision":resources::MIXED_REVISION,
        "measurement_domains":["s-content/1","legacy-evidence/1"],
        "job_limits":limit_node(),
        "native_ram":{"max_live_bytes":"4096","max_operation_bytes":"1024"}
    }))
    .unwrap();
    let fixture = Fixture::with_raw(raw);
    let prepared = fixture.load().unwrap();
    let changed = String::from_utf8(fixture.raw.clone())
        .unwrap()
        .replace("4096", "81920");
    std::fs::write(&fixture.limits, changed).unwrap();
    assert_eq!(fixture.load().err().unwrap().kind, ErrorKind::Changed);
    assert_eq!(prepared.consume().err().unwrap().kind, ErrorKind::Changed);
}

#[test]
fn mixed_resources_swallowed_ram_failure_cannot_seal_success() {
    let mut limits = limit_node();
    limits["max_store_bytes"] = json!("1048576");
    let raw = serde_json::to_vec(&json!({
        "schema_revision":resources::MIXED_REVISION,
        "measurement_domains":["s-content/1","legacy-evidence/1"],
        "job_limits":limits,
        "native_ram":{"max_live_bytes":"1048576","max_operation_bytes":"1048576"}
    }))
    .unwrap();
    let fixture = Fixture::with_raw(raw);
    let consumed = fixture.load().unwrap().consume().unwrap();
    let source = super::super::admission_provider::MeasuredDomain::measure(&consumed).unwrap();
    let mut started = consumed.into_job(source.existing).unwrap();
    let result = started.with_accounting(|account| {
        assert!(account
            .mixed_scope()
            .unwrap()
            .reserve_native(2 << 20)
            .is_err());
        Ok::<_, newchan_rust::s_content::CodecError>(())
    });
    // 即使调用者吞掉内层资源Err，失败父也不能被finish_check洗成成功。
    let _ = result;
    assert!(started.seal().is_err());
}
