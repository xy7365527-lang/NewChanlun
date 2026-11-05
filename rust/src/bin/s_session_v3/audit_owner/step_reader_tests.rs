// #1419 M8D 3A：owner 生涯封印签发的受管 SQL reader（L1–L5/P1）。真实认证 owner、真实 WAL 文件库、
// 真实外连接/换库/stat；拒绝点核 SELECT 不增长、首错保持与 owner 粘滞退役。
fn m8d_is_stale(error: &newchan_rust::s_content::CodecError) -> bool {
    error.to_string().contains("HistoricalAuditStale")
}
fn m8d_selects() -> u64 {
    newchan_rust::s_content::m8b_testonly::sql_reads_on_thread()
}
#[test]
fn m8d_descriptor_real_foreign_commit_stops_before_hit() {
    // 外连接在签发前提交：签发核（原 check）即拒，owner 粘滞退役。
    let (f, owner, parent) = m8d_owner();
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    f.conn
        .execute("UPDATE meta SET value='999' WHERE key='generation'", [])
        .unwrap();
    let mut active = active;
    let before = m8d_selects();
    assert!(active
        .with_step_reader(&parent, None, None, |_| ())
        .is_err());
    assert_eq!(m8d_selects(), before);
    assert!(owner.retired.get());
    drop(active);

    // 外连接在首读之后、下一次点查前提交：拒在 SELECT 前，首错保持，后续零 SELECT，步出口不交付成功。
    let (f, owner, parent) = m8d_owner();
    let nodes = m8d_nodes(&f, 2);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let mut seen = None;
    let before_all = m8d_selects();
    let error = active
        .with_step_reader(&parent, None, None, |reader| {
            let first = reader
                .frame(&nodes[0].0, |tag, body| (tag, body.to_vec()))
                .unwrap();
            f.conn
                .execute("UPDATE meta SET value='998' WHERE key='generation'", [])
                .unwrap();
            let before = m8d_selects();
            let second = reader.frame(&nodes[1].0, |_, _| ()).unwrap_err();
            let third = reader.frame(&nodes[0].0, |_, _| ()).unwrap_err();
            seen = Some((first, second, third, m8d_selects() - before));
        })
        .unwrap_err();
    let (first, second, third, selects) = seen.unwrap();
    assert_eq!(first, Some((nodes[0].1, nodes[0].2.clone())));
    assert!(m8d_is_stale(&second), "{second}");
    assert_eq!(second.to_string(), third.to_string(), "首错保持");
    assert_eq!(error, second.to_string(), "步出口返回首错");
    assert_eq!(selects, 0, "拒绝点之后不得再发节点 SELECT");
    assert_eq!(m8d_selects() - before_all, 1);
    assert!(owner.retired.get(), "入核后外部提交粘滞退役");
    drop(active);
    h1_assert_stale(&owner);

    // 步出口负锁：读完之后外连接提交、consume 不再点查，出口门仍拒，不交付成功。
    let (f, owner, parent) = m8d_owner();
    let nodes = m8d_nodes(&f, 2);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let error = active
        .with_step_reader(&parent, None, None, |reader| {
            reader.frame(&nodes[0].0, |tag, _| tag).unwrap().unwrap();
            f.conn
                .execute("UPDATE meta SET value='997' WHERE key='generation'", [])
                .unwrap();
        })
        .unwrap_err();
    assert!(error.contains("HistoricalAuditStale"), "{error}");
    assert!(owner.retired.get());
    drop(active);
}

#[test]
fn m8d_descriptor_main_wal_replace_and_stat_refuse() {
    let replace = |path: PathBuf| {
        let replacement = path.with_extension("replacement");
        std::fs::copy(&path, &replacement).unwrap();
        std::fs::rename(replacement, &path).unwrap();
    };
    for case in ["main", "wal", "stat"] {
        let (f, owner, parent) = m8d_owner();
        let nodes = m8d_nodes(&f, 2);
        let main = f.db_path();
        let wal = PathBuf::from(format!("{}-wal", main.display()));
        let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let mut seen = None;
        let exit = active
            .with_step_reader(&parent, None, None, |reader| {
                let first = reader.frame(&nodes[0].0, |tag, _| tag).unwrap();
                match case {
                    "main" => replace(main.clone()),
                    "wal" => replace(wal.clone()),
                    _ => {
                        // 只改 mtime，不改 SQL 原件：data_version 不能替代文件观测。
                        let file = std::fs::OpenOptions::new().write(true).open(&main).unwrap();
                        file.set_times(
                            std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH),
                        )
                        .unwrap();
                    }
                }
                let before = m8d_selects();
                let error = reader.frame(&nodes[1].0, |_, _| ()).unwrap_err();
                seen = Some((first, error, m8d_selects() - before));
            })
            .unwrap_err();
        let (first, error, selects) = seen.unwrap();
        assert_eq!(exit, error.to_string(), "{case}");
        assert_eq!(first, Some(nodes[0].1), "{case}");
        assert!(m8d_is_stale(&error), "{case}: {error}");
        assert_eq!(selects, 0, "{case}");
        assert!(owner.retired.get(), "{case}");
        assert!(
            owner
                .history_clamp
                .borrow()
                .as_ref()
                .is_none_or(|c| c.integrity_refusal()),
            "{case}：文件身份失证按完整性拒绝"
        );
    }
}

const M8D_CHILD_DB: &str = "NEWCHAN_M8D_CHILD_DB";
fn m8d_frame_line(digest: &[u8; 32], tag: u8, body: &[u8]) -> String {
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let body = body.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!("{hex}:{tag}:{body}")
}

/// 子进程入口：未设库路径时什么也不做；设了则只凭库路径冷认证后签发 reader。
#[test]
fn m8d_descriptor_child_process_cold_reader() {
    let Some(db) = std::env::var_os(M8D_CHILD_DB) else {
        return;
    };
    let db = PathBuf::from(db);
    let mut c = context();
    let owner = AuditConnection::open_certified_history_component(&db, 1 << 20, None, &mut c, None)
        .unwrap();
    let parent = m8d_parent(&owner);
    let digests: Vec<[u8; 32]> = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        let mut stmt = conn
            .prepare("SELECT digest FROM s_content_nodes ORDER BY digest LIMIT 3")
            .unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, Vec<u8>>(0))
            .unwrap()
            .map(|d| <[u8; 32]>::try_from(d.unwrap().as_slice()).unwrap())
            .collect();
        rows
    };
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let (frames, _) = active
        .with_step_reader(&parent, None, None, |reader| {
            digests
                .iter()
                .map(|digest| {
                    reader
                        .frame(digest, |tag, body| m8d_frame_line(digest, tag, body))
                        .unwrap()
                        .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    active.read_end().unwrap();
    println!("M8D_CHILD life={} frames={}", owner.life, frames.join(","));
}

#[test]
fn m8d_descriptor_dirty_phase_reads_after_closed_effects() {
    // 自身写阶段：未闭合时只拒签发、不退役；effects 闭合（原 candidate）后，以 check_candidate
    // 核过的那一份观测签发 reader 读原件；提交后 owner 生涯照常继续。
    let (f, owner, parent, mut c) = m8d_owner_in();
    let nodes = m8d_nodes(&f, 3);
    let active = owner
        .begin_in_request(TransactionBehavior::Immediate, Some(&c))
        .unwrap();
    mint(&active, &mut c);
    let mut active = active;
    active
        .before_fixed_write(FixedOperation::Accept, Some(&c))
        .unwrap();
    active
        .tx()
        .execute(
            "UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal WHERE singleton=1",
            [],
        )
        .unwrap();
    assert!(active.dirty.get());
    let error = active
        .with_step_reader(&parent, None, None, |_| ())
        .unwrap_err();
    assert!(error.contains("尚未闭合"), "{error}");
    assert!(!owner.retired.get(), "签发时点错误不进 owner 核");
    active.finish_fixed_write().unwrap();
    assert!(active.history_write.borrow().is_some());
    let parent = {
        let current = m8d_parent(&owner);
        assert!(current.same_parent(&parent));
        current
    };
    let (frames, receipt) = active
        .with_step_reader(&parent, None, None, |reader| {
            nodes
                .iter()
                .map(|(digest, _, _)| {
                    reader
                        .frame(digest, |tag, body| (tag, body.to_vec()))
                        .unwrap()
                        .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(
        frames,
        nodes
            .iter()
            .map(|(_, tag, body)| (*tag, body.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(receipt.node_selects, nodes.len() as u64);
    assert!(!owner.retired.get());
    let ended = active.commit().unwrap();
    owner.continue_with(ended).unwrap();
    assert!(
        !owner.retired.get(),
        "reader 未改变自身写的已核候选，提交照常"
    );
}

#[test]
fn m8d_descriptor_new_process_cannot_import_old_authority() {
    // 旧门只认 owner 私有连接：同库同内容的另一个连接的事务不能用它读，A 粘滞退役。
    let (f, owner, parent) = m8d_owner();
    let other = AuditConnection::open(&f.db_path()).unwrap();
    assert_ne!(other.life, owner.life);
    let foreign = other.begin(TransactionBehavior::Deferred).unwrap();
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let error = active
        .gate_check_on_for_test(&parent, foreign.tx())
        .unwrap_err();
    assert!(error.contains("owner 私有连接"), "{error}");
    assert!(owner.retired.get());
    drop(active);
    foreign.read_end().unwrap();

    // 真子进程：只拿到库路径，不得到任何 guard/门/资格；必须从 g1 冷认证建立自己的生涯后
    // 才能签发 reader，读出逐字相同的原件。子进程 life 计数从 1 起，与本进程数字重合也不转移资格。
    let expected = m8d_nodes(&f, 3)
        .iter()
        .map(|(digest, tag, body)| m8d_frame_line(digest, *tag, body))
        .collect::<Vec<_>>()
        .join(",");
    let name = concat!(module_path!(), "::m8d_descriptor_child_process_cold_reader");
    let name = name.split_once("::").unwrap().1;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([name, "--exact", "--nocapture", "--test-threads", "1"])
        .env(M8D_CHILD_DB, f.db_path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let line = stdout
        .lines()
        .find_map(|line| line.split_once("M8D_CHILD ").map(|(_, rest)| rest))
        .unwrap_or_else(|| panic!("子进程未运行冷认证 reader：{stdout}"));
    let (life, frames) = line.split_once(" frames=").unwrap();
    assert!(life.starts_with("life="), "{line}");
    assert_eq!(frames, expected, "子进程冷认证 reader 与本进程原件逐字相同");

    // 正控：新连接从冷认证建立自己的生涯后读出逐字相同的原件；旧门没有任何导入口。
    let mut c = context();
    let fresh = h1_certified_owner(&f, &mut c);
    let fresh_parent = m8d_parent(&fresh);
    let nodes = m8d_nodes(&f, 3);
    let mut active = fresh.begin(TransactionBehavior::Deferred).unwrap();
    let (frames, receipt) = active
        .with_step_reader(&fresh_parent, None, None, |reader| {
            nodes
                .iter()
                .map(|(digest, _, _)| {
                    reader
                        .frame(digest, |tag, body| (tag, body.to_vec()))
                        .unwrap()
                        .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(
        frames,
        nodes
            .iter()
            .map(|(_, tag, body)| (*tag, body.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(receipt.node_selects, 3);
    active.read_end().unwrap();
    assert!(!fresh.retired.get());
}

#[test]
fn m8d_descriptor_wrong_parent_keeps_prior_success() {
    let (f, owner, parent) = m8d_owner();
    let nodes = m8d_nodes(&f, 2);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    // 健康异父在入核前拒：不撤 owner，不发 SELECT。
    let foreign_parent = native::Budget::fresh(1 << 24).unwrap();
    let before = m8d_selects();
    let error = active
        .with_step_reader(&foreign_parent, None, None, |_| ())
        .unwrap_err();
    assert!(error.contains("父账"), "{error}");
    assert_eq!(m8d_selects(), before);
    assert!(!owner.retired.get());
    // 原请求绝对期限已过：签发即拒，不重起计时，也不退役。
    let expired = std::time::Instant::now().checked_sub(std::time::Duration::from_millis(1));
    assert!(active
        .with_step_reader(&parent, None, expired, |_| ())
        .is_err());
    assert!(!owner.retired.get());
    // 同父可读；读期间期限到期只拒本次，首错保持，不退役。
    let deadline = Some(std::time::Instant::now() + std::time::Duration::from_millis(3000));
    let mut seen = None;
    let exit = active
        .with_step_reader(&parent, None, deadline, |reader| {
            let first = reader.frame(&nodes[0].0, |tag, _| tag).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3100));
            let late = reader.frame(&nodes[1].0, |_, _| ()).unwrap_err();
            let again = reader.frame(&nodes[0].0, |_, _| ()).unwrap_err();
            seen = Some((first, late, again));
        })
        .unwrap_err();
    let (first, late, again) = seen.unwrap();
    assert_eq!(first, Some(nodes[0].1));
    assert!(late.to_string().contains("VerifyDeadline"), "{late}");
    assert_eq!(late.to_string(), again.to_string());
    assert_eq!(exit, late.to_string());
    assert!(!owner.retired.get(), "期限不是完整性失败");
    // 步出口负锁：consume 内超期但不再点查，出口按原绝对期限拒绝，不交付成功、不退役。
    let deadline = Some(std::time::Instant::now() + std::time::Duration::from_millis(3000));
    let exit = active
        .with_step_reader(&parent, None, deadline, |reader| {
            reader.frame(&nodes[0].0, |tag, _| tag).unwrap().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3100));
        })
        .unwrap_err();
    assert!(exit.contains("VerifyDeadline"), "{exit}");
    assert!(!owner.retired.get());
    active.read_end().unwrap();
}

#[test]
fn m8d_descriptor_own_write_checkpoint_and_epoch_boundaries() {
    let (f, owner, parent, mut c) = m8d_owner_in();
    let nodes = m8d_nodes(&f, 2);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let ((denied, after), receipt) = active
        .with_step_reader(&parent, None, None, |reader| {
            let alias = &owner.connection;
            let mut denied = Vec::new();
            denied.push(alias.execute_batch("COMMIT").is_err());
            denied.push(alias.execute_batch("ROLLBACK").is_err());
            denied.push(alias.unchecked_transaction().is_err());
            denied.push(
                alias
                    .execute("UPDATE meta SET value=value WHERE key='generation'", [])
                    .is_err(),
            );
            denied.push(alias.execute_batch("CREATE TEMP TABLE m8d(x)").is_err());
            denied.push(
                alias
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                    .is_err(),
            );
            denied.push(alias.execute_batch("PRAGMA wal_checkpoint").is_err());
            denied.push(alias.execute_batch("PRAGMA foreign_keys=OFF").is_err());
            denied.push(alias.execute_batch("SAVEPOINT m8d").is_err());
            assert!(!alias.is_autocommit(), "别名不能终结 reader 独占的事务");
            let after = reader
                .frame(&nodes[1].0, |tag, body| (tag, body.to_vec()))
                .unwrap();
            (denied, after)
        })
        .unwrap();
    assert!(denied.iter().all(|d| *d), "{denied:?}");
    assert_eq!(after, Some((nodes[1].1, nodes[1].2.clone())));
    assert_eq!(receipt.node_selects, 1);
    assert!(!owner.retired.get(), "被拒的别名没有改变任何观测");
    active.read_end().unwrap();
    // 正控：reader 结束后连接封印撤去，自身写、提交与 checkpoint 沿原夹持继续，不误报 foreign；
    // 之后在新事务重新签发 reader 读到新代原件。
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    h1_publish_round(&owner, &mut runtime, &mut c, 2);
    assert!(!owner.retired.get());
    let parent = m8d_parent(&owner);
    let nodes = m8d_nodes(&f, 4);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let (frames, _) = active
        .with_step_reader(&parent, None, None, |reader| {
            nodes
                .iter()
                .map(|(digest, _, _)| reader.frame(digest, |tag, _| tag).unwrap().unwrap())
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(frames, nodes.iter().map(|n| n.1).collect::<Vec<_>>());
    active.read_end().unwrap();
    assert!(!owner.retired.get());
}
