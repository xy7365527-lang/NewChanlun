// #1419 M8D 3A：生产增量（owned）与 native delta 两构建同注册的受管 SQL reader 锁（P1、材料期）。
// 真实认证 owner、真实 WAL 文件库与真实 publish；不依赖 native-only 读数计数器作判据。
fn m8d_parent(owner: &AuditConnection) -> native::Budget {
    owner
        .history_clamp
        .borrow()
        .as_ref()
        .unwrap()
        .parent()
        .unwrap()
        .clone()
}
fn m8d_nodes(f: &prepared::tests::Fixture, n: usize) -> Vec<([u8; 32], u8, Vec<u8>)> {
    let mut stmt = f
        .conn
        .prepare("SELECT digest, tag, body FROM s_content_nodes ORDER BY digest LIMIT ?1")
        .unwrap();
    let rows = stmt
        .query_map([n as i64], |r| {
            let digest: Vec<u8> = r.get(0)?;
            let tag: i64 = r.get(1)?;
            Ok((
                <[u8; 32]>::try_from(digest.as_slice()).unwrap(),
                u8::try_from(tag).unwrap(),
                r.get::<_, Vec<u8>>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(rows.len() >= 2, "夹具须有至少两个真实节点");
    rows
}
fn m8d_owner_in() -> (
    prepared::tests::Fixture,
    AuditConnection,
    native::Budget,
    request_audit::RequestAuditContext,
) {
    let f = fixture();
    let mut c = context();
    let owner = h1_certified_owner(&f, &mut c);
    let parent = m8d_parent(&owner);
    (f, owner, parent, c)
}
fn m8d_owner() -> (prepared::tests::Fixture, AuditConnection, native::Budget) {
    let (f, owner, parent, _) = m8d_owner_in();
    (f, owner, parent)
}

#[test]
fn h1_m8d_access_descriptor_real_life_diff() {
    // 生产读口 TxStore 与 owner 签发 reader 对同一事务逐帧对拍；缺失同为 None；读数逐次可对账。
    use newchan_rust::s_content::NodeSource;
    let (f, owner, parent) = m8d_owner();
    let nodes = m8d_nodes(&f, 16);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let mut probe: Vec<[u8; 32]> = nodes.iter().map(|n| n.0).collect();
    probe.push([0; 32]);
    let expected = {
        let mut store = crate::content_store::TxStore::new(active.tx()).unwrap();
        probe
            .iter()
            .map(|digest| {
                store
                    .checked_read(digest)
                    .unwrap()
                    .map(|frame| (frame.tag(), frame.body().to_vec()))
            })
            .collect::<Vec<_>>()
    };
    #[cfg(feature = "s_capture_native_testonly")]
    let before = m8d_selects();
    let base = parent.live();
    let (actual, receipt) = active
        .with_step_reader(&parent, None, None, |reader| {
            probe
                .iter()
                .map(|digest| {
                    reader
                        .frame(digest, |tag, body| (tag, body.to_vec()))
                        .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.last(), Some(&None));
    assert_eq!(receipt.node_selects, probe.len() as u64);
    #[cfg(feature = "s_capture_native_testonly")]
    assert_eq!(m8d_selects() - before, probe.len() as u64);
    // 构造 1 次 + 每次点查 1 次 + 步出口 1 次的固定域核，各 3 条元数据查询（夹持观测另计于原控制读）。
    assert_eq!(receipt.metadata_reads, 3 * (probe.len() as u64 + 2));
    assert_eq!(
        parent.live(),
        base,
        "reader 工作域、行副本许可与夹持控制读随生涯结束回到父账"
    );
    active.read_end().unwrap();
    assert!(!owner.retired.get());
}

fn m8d_frames(
    reader: &mut newchan_rust::s_content::m8d_owner_bridge::GatedSqlReader<'_, '_, '_>,
    nodes: &[([u8; 32], u8, Vec<u8>)],
) -> Vec<Option<(u8, Vec<u8>)>> {
    nodes
        .iter()
        .map(|(digest, _, _)| {
            reader
                .frame(digest, |tag, body| (tag, body.to_vec()))
                .unwrap()
        })
        .collect()
}

type MaterialOutcome = std::rc::Rc<
    std::cell::RefCell<
        Option<Result<(Vec<Option<(u8, Vec<u8>)>>, step_reader::StepReaderReceipt), String>>,
    >,
>;

/// 真实 publish 在 effects 闭合前跑钩子：取真实材料能力见证，`unexplained` 时先经本事务做一笔
/// Publish 不允许的实际写，再由 owner 工厂以 Some(MaterialLive) 签发 reader 读已有原件。
fn m8d_material_publish(
    unexplained: bool,
) -> (
    prepared::tests::Fixture,
    AuditConnection,
    Vec<([u8; 32], u8, Vec<u8>)>,
    Result<(), String>,
    MaterialOutcome,
) {
    let (f, owner, _, mut c) = m8d_owner_in();
    let nodes = m8d_nodes(&f, 4);
    let envelope = b2_envelope(2);
    let token = b2_accept_begin(&owner, &envelope, &mut c);
    let outcome: MaterialOutcome = Default::default();
    let (hook_out, hook_nodes) = (outcome.clone(), nodes.clone());
    effects::arm_material_probe();
    step_reader::arm_material_hook(Box::new(move |active| {
        let material = effects::take_material_probe();
        let result = (|| {
            let material = material.ok_or("真实 publish 未签发材料能力")?;
            if !active.dirty.get() || active.history_write.borrow().is_some() {
                return Err("钩子不在 node 写后、effects 闭合前".to_string());
            }
            if unexplained {
                active
                    .tx()
                    .execute(
                        "UPDATE raw_events SET rowid=rowid WHERE rowid=(SELECT min(rowid) FROM raw_events)",
                        [],
                    )
                    .map_err(|e| e.to_string())?;
            }
            let parent = m8d_parent(active.owner);
            active.with_step_reader(&parent, Some(material), None, |reader| {
                m8d_frames(reader, &hook_nodes)
            })
        })();
        *hook_out.borrow_mut() = Some(result);
    }));
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let published = owner
        .publish(&mut runtime, &token, &envelope, &mut c)
        .and_then(|(receipt, ended)| {
            assert_eq!(receipt.generation, 2);
            owner.continue_with(ended)
        });
    assert!(
        effects::take_material_probe().is_none(),
        "探针已被钩子取走并撤防"
    );
    (f, owner, nodes, published, outcome)
}

#[test]
fn h1_m8d_material_phase_reader_after_node_writes() {
    // 正控：真实 publish 的 node 自身写之后、Recording 仍活、effects 未闭合时，以真实
    // MaterialLive 签发 reader；正文与已有原件逐字相同；随后 finish/candidate/提交照原门通过。
    let (_f, owner, nodes, published, outcome) = m8d_material_publish(false);
    let (frames, receipt) = outcome
        .borrow_mut()
        .take()
        .expect("钩子已运行")
        .expect("材料期 reader 签发与读取");
    assert_eq!(
        frames,
        nodes
            .iter()
            .map(|(_, tag, body)| Some((*tag, body.clone())))
            .collect::<Vec<_>>()
    );
    assert_eq!(receipt.node_selects, nodes.len() as u64);
    published.expect("材料期读取不改变自身写候选，effects 闭合与提交照常");
    assert!(!owner.retired.get());
    h1_compare_complete_history(&owner);
}

#[test]
fn h1_m8d_material_phase_unexplained_write_refuses_before_select() {
    // 负控：材料期本事务出现 Publish 未声明的实际写，材料见证失效；签发即拒、零节点 SELECT、
    // owner 粘滞退役；随后 effects 闭合同样拒绝，不以读期成功洗白。
    let (_f, owner, _nodes, published, outcome) = m8d_material_publish(true);
    let error = outcome
        .borrow_mut()
        .take()
        .expect("钩子已运行")
        .unwrap_err();
    assert!(error.contains("材料能力"), "{error}");
    assert!(owner.retired.get());
    assert!(published.is_err());
}

#[test]
fn h1_m8d_sqlite_parent_peak_and_first_error_receipts() {
    // P6 观察部分：真实 owner、真实父账与工作域。每帧租约存活时父账增量覆盖 Rust 行副本，
    // owner 私有连接上的 SQLite 页缓存与语句字节同时在场但不在父账；reader 生涯结束后
    // 父账回到基线而页缓存仍归连接。B_sql 同父覆盖尚未获批，覆盖额与少 1B 门不在本锁签署。
    let (f, owner, parent) = m8d_owner();
    let nodes = m8d_nodes(&f, 16);
    let status = |op| {
        let (mut current, mut high) = (0, 0);
        // SAFETY: owner 私有连接存活；只读当前值，不重置高水位、不执行 SQL。
        let rc = unsafe {
            rusqlite::ffi::sqlite3_db_status(
                owner.connection.handle(),
                op,
                &mut current,
                &mut high,
                0,
            )
        };
        assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
        current
    };
    let idle_cache = status(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED);
    let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let base = parent.live();
    let (points, receipt) = active
        .with_step_reader(&parent, None, None, |reader| {
            nodes
                .iter()
                .map(|(digest, _, expected)| {
                    reader
                        .frame(digest, |_, body| {
                            assert_eq!(body, expected.as_slice());
                            (
                                body.len(),
                                parent.live() - base,
                                status(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED),
                            )
                        })
                        .unwrap()
                        .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    let after_cache = status(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED);
    for (body, rust, cache) in &points {
        println!(
            "M8D_P6_RECEIPT body={body} parent_delta={rust} connection_cache={cache} coverage=unproven(B_sql)"
        );
        assert!(rust >= body, "租约存活时父账覆盖 Rust 行副本");
        assert!(*cache > 0, "页缓存读数为正，不能签 0");
    }
    println!(
        "M8D_P6_RECEIPT idle_cache={idle_cache} after_cache={after_cache} node_selects={} metadata_reads={}",
        receipt.node_selects, receipt.metadata_reads
    );
    assert_eq!(parent.live(), base, "reader 生涯结束父账回基线");
    assert!(after_cache > 0, "页缓存在 reader 生涯后仍归连接");
    active.read_end().unwrap();
    assert!(!owner.retired.get());
}
