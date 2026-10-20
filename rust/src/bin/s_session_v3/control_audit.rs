//! #1394 C §5.1/§5.4/§5.8：`s-session/3` 控制历史的**只读审计入口**（S3 消费端）。
//!
//! 本模块不新写第二套历史判断：全局控制行核验、profile/catalog 绑定、Writer 换代史、持久消息
//! 与原始回执、attempt 序与 clock 相位、普通投递前沿、以及逐代
//! `(batch_id, commit_ns, session_generation, clock_plan_hash)` 期望，全部来自
//! [`v2::validate_control_history`]（原 `prepare_control` 主体，唯一一份实现，只把协议版本与
//! `canonical_envelope` 校验参数化）。旧 v2 入口继续只认 `s-session/2`，本入口只认
//! `s-session/3`，两边的协议判据都显式且不放宽。
//!
//! S3 入口自己显式负责的只有四件：
//!
//! 1. **双协议标记**：根 `meta.protocol_revision` 与 `s_protocol_meta.protocol_revision` 必须都是
//!    `s-session/3`。混合域、旧域、未知版本都在任何读取复用之前拒绝（§5.8 拒绝边界），不回落
//!    到 v2 语义，也不允许旧 validator 解释新域。
//! 2. **唯一已认可 schema**：root `SCHEMA` + `v2::SCHEMA_V2` + 按 profile 的 `tb02a::SCHEMA` +
//!    `content_store` 两表 + `s_content_publications` + 三张辅助索引及对象生命周期索引。
//!    缺表、结构不符、**任何未知表**一律拒绝：
//!    这里不调用旧 `verify_schema` 的白名单冒充支持 3，也不用「先按旧表核一遍、新表另算」的
//!    降级通道。S3 域不含旧私有格式登记表（changes-only/1、证据图三表），它们出现即未知表。
//!    内容格式登记另外核**行数**：`s_content_meta` 必须恰一行 `singleton=1` 的 `s-content/1`
//!    （`CHECK` 可被旁路，模式声明与逐列类型扫描都不证明存量行数）。
//! 3. **当前冻结 profile**：复用 crate root 的 `verify_profile_binding`（唯一判据）；catalog 的
//!    来源/hash/revision 与静态行由共用核心逐字段核。
//! 4. **私有发布索引闭合**：`s_content_publications` 与 `structure_deltas` 逐代一一对应
//!    （generation 恰为 `1..=meta.generation`、`batch_id == index_frontier`、三根 32 字节）。
//!
//! 边界如实：**完整 content/对象/source/Delta 业务审计不在本模块**，由 root 的发布/内容审计
//! 模块承担；本模块返回的逐代期望只声明「控制历史自洽、每代都有真实 Commit 与发布索引行」，
//! 不冒充内容闭包、ID 配方、来源对拍或 crosswalk 已核。本模块只读，不写任何行。

use super::audit_store::{CodecRead, NativeAdmission};
use super::*;
use crate::v3::business_json::{self as control_json, TextValue};
use newchan_rust::s_content::capture_allocation::scratch::WorkScope;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};
use newchan_rust::sqlite_checked::CheckedStatement;

/// S3 控制历史审计入口（只读）。
///
/// 成功返回 v2/S3 共用的 [`v2::ControlHistory`]：调用方按真实批次逐代调用
/// [`v2::ControlHistory::consume_header`]（manifest 的 `protocol_revision` 必须先被调用方显式核为
/// `s-session/3`），最后 [`v2::ControlHistory::finish`] 检查逐代期望全部消费。
pub(super) fn prepare_s3_control_history(conn: &Connection) -> Result<v2::ControlHistory, String> {
    prepare_s3_control_history_with_owner(conn, None)
}
pub(super) fn prepare_s3_control_history_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<v2::ControlHistory, String> {
    prepare_s3_control_history_admitted(conn, owner, None)
}
pub(super) fn prepare_s3_control_history_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<v2::ControlHistory, String> {
    if let Some(work) = work {
        if owner.is_some() {
            return Err(work
                .reject(
                    newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger,
                )
                .to_string());
        }
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制审计未启用".into());
    }
    verify_s3_markers_admitted(conn, owner, work)?;
    verify_s3_schema_admitted(conn, owner, work)?;
    let _profile = crate::verify_profile_binding_admitted(conn, owner, work)?;
    let epoch = v2::verify_epoch_history_admitted(conn, true, owner, work)?;
    let history =
        v2::validate_control_history_admitted(conn, PROTOCOL_REVISION, epoch, owner, work)?;
    verify_s3_publications_admitted(conn, owner, work)?;
    Ok(history)
}

/// 双协议标记：根 `meta` 与控制行都必须逐字登记 `s-session/3`。
fn verify_s3_markers(conn: &Connection) -> Result<(), String> {
    verify_s3_markers_with_owner(conn, None)
}
fn verify_s3_markers_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<(), String> {
    verify_s3_markers_admitted(conn, owner, None)
}
fn verify_s3_markers_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let meta_marker = v2::control_meta_text_admitted(conn, "protocol_revision", owner, work)?;
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
    )
    .map_err(|e| {
        format!("StorageUnavailable：读 s_protocol_meta 标记失败（控制域不可判定）：{e}")
    })?;
    let table_marker = match stmt.next().map_err(|e| {
        format!("StorageUnavailable：读 s_protocol_meta 标记失败（控制域不可判定）：{e}")
    })? {
        Some(mut row) => Some(
            TextValue::from_sql_admitted(&mut row, 0, owner, work).map_err(|e| {
                format!("StorageUnavailable：读 s_protocol_meta 标记失败（控制域不可判定）：{e}")
            })?,
        ),
        None => None,
    };
    stmt.finalize().map_err(|e| {
        format!("StorageUnavailable：读 s_protocol_meta 标记失败（控制域不可判定）：{e}")
    })?;
    if meta_marker.as_deref() != Some(PROTOCOL_REVISION)
        || table_marker.as_deref() != Some(PROTOCOL_REVISION)
    {
        return Err(format!(
            "IdentityConflict：控制审计只属于已登记的 {PROTOCOL_REVISION} 域（meta标记={meta_marker:?}，控制表标记={table_marker:?}）；混合域/旧域/未知版本一律拒绝"
        ));
    }
    Ok(())
}

/// S3 唯一已认可 schema：逐表全等比 DDL（未知表/缺表即拒）+ 存量行类型核 + 内容格式登记值。
pub(super) fn schema_ddls(conn: &Connection) -> Result<Vec<&'static str>, String> {
    schema_ddls_with_owner(conn, None).map(|v| v.into_iter().collect())
}
pub(crate) fn schema_ddls_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<CodecVec<&'static str>, String> {
    schema_ddls_admitted(conn, owner, None)
}
pub(crate) fn schema_ddls_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<&'static str>, String> {
    let mut ddls = control_json::new_work(owner);
    for ddl in [SCHEMA, v2::SCHEMA_V2] {
        NativeAdmission(work)
            .native_codec_room(&mut ddls, 1)
            .map_err(|e| e.to_string())?;
        ddls.try_push(ddl).map_err(|e| e.to_string())?;
    }
    if v2::control_meta_text_admitted(conn, "profile_id", owner, work)?.as_deref()
        == Some(tb02a::PROFILE)
    {
        NativeAdmission(work)
            .native_codec_room(&mut ddls, 1)
            .map_err(|e| e.to_string())?;
        ddls.try_push(tb02a::SCHEMA).map_err(|e| e.to_string())?;
    }
    for ddl in [
        content_store::SCHEMA,
        commit::SCHEMA,
        aux_index::SCHEMA,
        object_index::SCHEMA,
    ] {
        NativeAdmission(work)
            .native_codec_room(&mut ddls, 1)
            .map_err(|e| e.to_string())?;
        ddls.try_push(ddl).map_err(|e| e.to_string())?;
    }
    Ok(ddls)
}
pub(super) fn verify_s3_schema(conn: &Connection) -> Result<(), String> {
    verify_s3_schema_with_owner(conn, None)
}
pub(crate) fn verify_s3_schema_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<(), String> {
    verify_s3_schema_admitted(conn, owner, None)
}
pub(crate) fn verify_s3_schema_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let mut expected = v2::ControlSchema::new(owner);
    for ddl in schema_ddls_admitted(conn, owner, work)? {
        expected.add_ddl_admitted("S3固定DDL", ddl, work)?;
    }
    v2::verify_control_schema_admitted(conn, &expected, work)?;
    content_store::ensure_format(conn).map_err(|e| format!("StorageUnavailable：{e}"))?;
    verify_content_registration_rows(conn)?;
    Ok(())
}

/// `ensure_format` 只按 `singleton=1` 取值，而 CHECK 可被旁路写入第二行登记（或另起号）；
/// 模式比对与逐列类型扫描都不证明行数。S3 入口在此补全表：格式登记必须**恰一行**且
/// `singleton=1`——多出的登记行是未声明域，与未知表同门拒绝。
fn verify_content_registration_rows(conn: &Connection) -> Result<(), String> {
    let (rows, singletons): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(singleton=1),0) FROM s_content_meta",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("StorageUnavailable：读 s_content_meta 登记行失败：{e}"))?;
    if rows != 1 || singletons != 1 {
        return Err(format!(
            "IdentityConflict：s_content_meta 必须恰一行 singleton=1 的 s-content/1 登记（实际 {rows} 行，singleton=1 有 {singletons} 行）"
        ));
    }
    Ok(())
}

/// S3 私有发布索引与结构 Delta 逐代闭合。这里只核索引行的存在性与代号绑定；
/// manifest/snapshot/Delta 的内容闭包由 root 的发布/内容审计模块核。
fn verify_s3_publications(conn: &Connection) -> Result<(), String> {
    verify_s3_publications_with_owner(conn, None)
}
fn verify_s3_publications_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<(), String> {
    verify_s3_publications_admitted(conn, owner, None)
}
fn verify_s3_publications_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let generation = v2::control_meta_i64_admitted(conn, "generation", owner, work)?;
    let (count, head): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*),COALESCE(MAX(generation),0) FROM s_content_publications",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("StorageUnavailable：读发布索引失败：{e}"))?;
    let delta_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM structure_deltas", [], |r| r.get(0))
        .map_err(|e| format!("StorageUnavailable：读结构 Delta 行数失败：{e}"))?;
    if generation < 0 || count != generation || head != generation || delta_count != generation {
        return Err(format!(
            "IdentityConflict：S3 发布索引/结构 Delta 与根代号不闭合（generation={generation}，发布行={count}，发布最大代={head}，Delta行={delta_count}）"
        ));
    }
    let mut q = CheckedStatement::prepare(
        conn,
        c"SELECT p.generation,p.batch_id,d.index_frontier,
                    length(p.manifest_root),length(p.snapshot_root),length(p.delta_root)
             FROM s_content_publications p
             LEFT JOIN structure_deltas d ON d.generation=p.generation
             ORDER BY p.generation",
    )
    .map_err(|e| format!("StorageUnavailable：prepare 发布索引核验失败：{e}"))?;
    let mut expected_generation = 1i64;
    while let Some(mut r) = q
        .next()
        .map_err(|e| format!("StorageUnavailable：读发布索引行失败：{e}"))?
    {
        let g: i64 = r
            .integer(0)
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let batch_id = TextValue::from_sql_admitted(&mut r, 1, owner, work)
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let delta_batch = TextValue::optional_sql_admitted(&mut r, 2, owner, work)
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let root_lens = [
            r.optional_integer(3)
                .map_err(|e| format!("StorageUnavailable：{e}"))?,
            r.optional_integer(4)
                .map_err(|e| format!("StorageUnavailable：{e}"))?,
            r.optional_integer(5)
                .map_err(|e| format!("StorageUnavailable：{e}"))?,
        ];
        validate_publication_pair(
            g,
            expected_generation,
            batch_id.as_str(),
            delta_batch.as_deref(),
            root_lens,
        )?;
        expected_generation += 1;
    }
    q.finalize()
        .map_err(|e| format!("StorageUnavailable：读发布索引行失败：{e}"))?;
    Ok(())
}

pub(super) fn validate_publication_pair(
    g: i64,
    expected_generation: i64,
    batch_id: &str,
    delta_batch: Option<&str>,
    root_lens: [Option<i64>; 3],
) -> Result<(), String> {
    if g != expected_generation
        || delta_batch != Some(batch_id)
        || root_lens != [Some(32), Some(32), Some(32)]
    {
        return Err(format!(
                "IdentityConflict：发布索引第 {g} 代与结构 Delta 不闭合（应代={expected_generation}，batch_id={batch_id}，delta批次={delta_batch:?}，三根长度={root_lens:?}）"
            ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::session_protocol::canonical_json;
    use rusqlite::TransactionBehavior;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    const SESSION: &str = "s3-audit-test";
    const CLOCK_ID: &str = "s3-audit-test-clock";
    /// `commit::tests::prepare` 的固定发布夹具把提交时刻取作 `12`，夹具与它逐字一致。
    const COMMIT_NS: &str = "12";

    /// 一次真实 S3 发布留下的夹具：`commit::tests::fixture()` 的真接纳 + 真 Begin（含持久
    /// `begin_*` 历史证据），再走生产的 `prepare` / `publish_prepared` 真发布。
    struct Domain {
        fixture: super::super::commit::tests::Fixture,
        clock_hash: String,
        batch_id: String,
    }

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "s1394-s3-audit-{}-{stamp}-{number}",
                std::process::id()
            ));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn repo_fixture(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session")
            .join(path)
    }

    /// 显式 clock fixture：input-1 的 accept 10 → begin 11 → commit 12。
    fn clock_value(clock_id: &str) -> Value {
        json!({
            "schema_revision": "s-clock-plan/1",
            "clock_plan_id": clock_id,
            "origin_utc": "2026-09-18T00:00:00Z",
            "unit": "ns",
            "events": {
                "input-1": {"accept_ns": "10",
                    "attempts": [{"begin_ns": "11", "commit_ns": COMMIT_NS}]},
            }
        })
    }

    /// 全新空域（0 代）：init_v3 建域，不写任何接纳/发布行。
    fn empty_domain(temp: &Temp) -> (PathBuf, v2::ClockPlan) {
        let db = temp.path("s3-empty.sqlite");
        let clock_path = temp.path("clock.json");
        std::fs::write(&clock_path, canonical_json(&clock_value(CLOCK_ID))).unwrap();
        super::super::init_v3(
            &db,
            SESSION,
            &repo_fixture("catalog/signed-catalog.json"),
            &repo_fixture("profiles/testonly_tick_1_1_ohlc.json"),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let clock = v2::ClockPlan::load(&clock_path).unwrap();
        (db, clock)
    }

    /// 真实 1 代 S3 域：接纳与 Begin 来自 `commit::tests::fixture()`（真 raw 行、真消息行、
    /// accept/begin phase 与持久 `begin_*`），发布走生产的 `prepare` + `publish_prepared`
    /// （真 manifest/snapshot/Delta 根与节点、真 Delta 行、真发布索引行、根前沿推进）。
    /// 本片不手写控制行、不复制发布判据、不另立第二套发布上下文。
    fn published_domain() -> Domain {
        let mut fixture = super::super::commit::tests::fixture();
        let clock_hash = v2::stored_clock(&fixture.conn).unwrap().hash;
        let tx = fixture
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (metadata, roots, manifest) =
            super::super::commit::tests::prepare(&tx, &fixture.envelope);
        let receipt = super::super::commit::publish_prepared(
            tx,
            &fixture.token,
            &metadata,
            &roots,
            &manifest,
        )
        .unwrap();
        assert_eq!(receipt.generation, 1);
        Domain {
            fixture,
            clock_hash,
            batch_id: receipt.batch_id.as_str().to_owned(),
        }
    }

    /// 篡改必须以**回滚**边界施加：拒绝若来自夹具残破而非篡改，回滚后的健康核对就会失败。
    fn assert_tamper_rejected<F>(domain: &mut Domain, what: &str, tamper: F)
    where
        F: FnOnce(&rusqlite::Transaction<'_>),
    {
        let tx = domain
            .fixture
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tamper(&tx);
        let error = match prepare_s3_control_history(&tx) {
            Ok(_) => panic!("{what}（预期拒绝）"),
            Err(error) => error,
        };
        assert!(!error.is_empty(), "{what} 的拒绝必须带原因");
        tx.rollback().unwrap();
        let mut history = prepare_s3_control_history(&domain.fixture.conn).unwrap();
        history
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS),
            )
            .unwrap();
        history.finish().unwrap();
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r12_native_control_cold_matches_plain_published_facts_on_parent_scope() {
        let domain = published_domain();
        let mut plain = prepare_s3_control_history(&domain.fixture.conn).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(32 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                // 代表调用方已持有的父域工作；控制链必须继续使用这一本账。
                let _caller_payload = NativeAdmission(Some(scope))
    .native_owned_text("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
    .map_err(|e| e.to_string())?;
                let before = budget.live();
                let mut native =
                    prepare_s3_control_history_admitted(&domain.fixture.conn, None, Some(scope))?;
                assert_eq!(native.session_generation(), plain.session_generation());
                assert_eq!(native.clock_plan_hash(), plain.clock_plan_hash());
                assert_eq!(
                    native.remaining_commit(1, &domain.batch_id),
                    plain.remaining_commit(1, &domain.batch_id)
                );
                assert!(budget.live() > before);
                let after = budget.live();
                for history in [&mut plain, &mut native] {
                    history.consume_header(
                        1,
                        &domain.batch_id,
                        Some("1"),
                        Some(&domain.clock_hash),
                        Some(COMMIT_NS),
                    )?;
                }
                // 第十七处裁改断言（主控裁定 https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6298532692）。
                // 原断言（162d6b7b30，随 PR #1573 入 main）：finish 后 `live == after`「消费已有事实不再复制文本，Native许可留在原父域」，
                // 离域后 `live > 137`「离开WorkScope不提前释放父域许可」——依赖 main 的 NativeReservation::retain「保守留在父账到父作用域结束」模型。
                // v4 合同 :9「许可随对象移动，在对象析构以后返还」：finish 析构事实对象后许可回到 before，离域后只剩父账元数据。
                // 原意图中仍成立的“消费已有事实不再复制文本”保留为 finish 前的 `== after`。
                assert_eq!(budget.live(), after, "消费已有事实不再复制文本");
                plain.finish()?;
                native.finish()?;
                assert_eq!(
                    budget.live(),
                    before,
                    "v4：Native许可随事实对象析构返还原父域，回到调用方已持有工作"
                );
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(
            budget.live(),
            super::super::audit_owner::native::Budget::metadata_bytes(),
            "v4：离开WorkScope且全部对象析构后只剩父账元数据，无滞留许可"
        );
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r12_native_control_cold_matches_plain_empty_domain_on_parent_scope() {
        let temp = Temp::new();
        let (db, _) = empty_domain(&temp);
        let conn = open_db(&db).unwrap();
        let plain = prepare_s3_control_history(&conn).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(32 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let native = prepare_s3_control_history_admitted(&conn, None, Some(scope))?;
                assert_eq!(native.session_generation(), plain.session_generation());
                assert_eq!(native.clock_plan_hash(), plain.clock_plan_hash());
                assert_eq!(
                    native.remaining_commit(1, "absent"),
                    plain.remaining_commit(1, "absent")
                );
                native.finish()?;
                plain.finish()?;
                assert!(budget.live() > 0);
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_control_vertical_chain_matches_plain_commit_and_releases() {
        let domain = published_domain();
        let plain = prepare_s3_control_history(&domain.fixture.conn).unwrap();
        let budget = OwnedBudget::new(10_000_000);
        let mut owned =
            prepare_s3_control_history_with_owner(&domain.fixture.conn, Some(&budget)).unwrap();
        assert_eq!(owned.session_generation(), plain.session_generation());
        assert_eq!(owned.clock_plan_hash(), plain.clock_plan_hash());
        assert_eq!(
            owned.remaining_commit(1, &domain.batch_id),
            plain.remaining_commit(1, &domain.batch_id)
        );
        let before = budget.snapshot();
        assert!(before.live_requested_layout_bytes > 0);
        assert!(owned
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some("121")
            )
            .is_err());
        assert_eq!(owned.remaining_commit(1, &domain.batch_id), Some(COMMIT_NS));
        owned
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS),
            )
            .unwrap();
        assert_eq!(
            budget.snapshot().allocation_attempts,
            before.allocation_attempts
        );
        assert!(owned
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS)
            )
            .is_err());
        owned.finish().unwrap();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_control_vertical_chain_preserves_negative_gate_order() {
        let domain = published_domain();
        let tx = domain.fixture.conn.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE s_clock_events SET semantic_ns='999' WHERE phase='commit'",
            [],
        )
        .unwrap();
        tx.execute(
            "UPDATE s_protocol_meta SET logical_phase_frontier='9999'",
            [],
        )
        .unwrap();
        let plain = prepare_s3_control_history(&tx).err().unwrap();
        let budget = OwnedBudget::new(10_000_000);
        let owned = prepare_s3_control_history_with_owner(&tx, Some(&budget))
            .err()
            .unwrap();
        assert_eq!(owned, plain);
        assert!(plain.contains("phase时间/代际与计划不一致"), "{plain}");
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        tx.rollback().unwrap();
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_control_vertical_chain_zero_budget_refuses_without_plain_fallback() {
        let domain = published_domain();
        let budget = OwnedBudget::new(0);
        let error = prepare_s3_control_history_with_owner(&domain.fixture.conn, Some(&budget))
            .err()
            .unwrap();
        assert!(
            error.contains(newchan_rust::s_content::capture_allocation::LIMIT),
            "{error}"
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(budget.snapshot().rejected_layouts > 0);
        prepare_s3_control_history(&domain.fixture.conn).unwrap();
    }

    #[test]
    fn s3_audit_accepts_fresh_domain_with_no_generation() {
        let temp = Temp::new();
        let (db, _clock) = empty_domain(&temp);
        let conn = open_db(&db).unwrap();
        let history = prepare_s3_control_history(&conn).unwrap();
        assert_eq!(history.pending_count(), 0);
        history.finish().unwrap();
    }

    #[test]
    fn s3_audit_consumes_real_accept_begin_commit_history() {
        let domain = published_domain();
        let mut history = prepare_s3_control_history(&domain.fixture.conn).unwrap();
        assert_eq!(history.session_generation(), "1");
        assert_eq!(history.clock_plan_hash(), domain.clock_hash);
        assert_eq!(
            history.remaining_commit(1, &domain.batch_id),
            Some(COMMIT_NS),
            "逐代期望必须来自真实 Commit phase 的 clock 时刻"
        );
        // manifest 协议标记由调用方显式核为 s-session/3 后才消费共用核心。
        assert!(history
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS)
            )
            .is_ok());
        history.finish().unwrap();
        // 重复消费/缺失代/未消费都要拒绝。
        let mut history = prepare_s3_control_history(&domain.fixture.conn).unwrap();
        assert!(history
            .consume_header(
                2,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS)
            )
            .is_err());
        assert!(history
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some("121")
            )
            .is_err());
        history
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS),
            )
            .unwrap();
        assert!(history
            .consume_header(
                1,
                &domain.batch_id,
                Some("1"),
                Some(&domain.clock_hash),
                Some(COMMIT_NS)
            )
            .is_err());
        history.finish().unwrap();
    }

    #[test]
    fn s3_audit_rejects_tampered_clock_message_receipt() {
        let mut domain = published_domain();
        assert_tamper_rejected(&mut domain, "commit 语义时刻被篡改", |tx| {
            tx.execute(
                "UPDATE s_clock_events SET semantic_ns='119' WHERE phase='commit'",
                [],
            )
            .unwrap();
        });
        assert_tamper_rejected(&mut domain, "持久消息 payload_hash 被篡改", |tx| {
            tx.execute(
                "UPDATE s_input_messages SET payload_hash=?1",
                params!["0".repeat(64)],
            )
            .unwrap();
        });
        assert_tamper_rejected(&mut domain, "原始回执 receipt_id 被篡改", |tx| {
            tx.execute("UPDATE raw_events SET receipt_id='rcpt-tampered'", [])
                .unwrap();
        });
        assert_tamper_rejected(&mut domain, "持久消息规范原件被替换", |tx| {
            let raw: String = tx
                .query_row("SELECT canonical_envelope FROM s_input_messages", [], |r| {
                    r.get(0)
                })
                .unwrap();
            let mut value: Value = strict_json(raw.as_bytes()).unwrap();
            value["payload"]["writer_epoch"] = json!("2");
            tx.execute(
                "UPDATE s_input_messages SET canonical_envelope=?1",
                params![canonical_json(&value)],
            )
            .unwrap();
        });
    }

    #[test]
    fn s3_audit_rejects_tampered_delivery_epoch_profile_and_publications() {
        let mut domain = published_domain();
        assert_tamper_rejected(&mut domain, "普通投递引用被删除", |tx| {
            tx.execute("DELETE FROM s_delivery_refs", []).unwrap();
        });
        assert_tamper_rejected(
            &mut domain,
            "writer 换代历史被塞入未闭合行",
            |tx| {
                tx.execute(
                    "INSERT INTO writer_epoch_history(ordinal,from_epoch,to_epoch,
                     generation_at_transition,advance_state_at_transition,transitioned_at)
                 VALUES(1,'1','2','0','idle','5')",
                    [],
                )
                .unwrap();
            },
        );
        assert_tamper_rejected(&mut domain, "冻结 profile hash 被改写", |tx| {
            tx.execute(
                "UPDATE meta SET value=?1 WHERE key='profile_hash'",
                params!["f".repeat(64)],
            )
            .unwrap();
        });
        assert_tamper_rejected(&mut domain, "发布索引行被删除", |tx| {
            tx.execute("DELETE FROM s_content_publications", [])
                .unwrap();
        });
        assert_tamper_rejected(
            &mut domain,
            "发布索引批次与结构 Delta 不同",
            |tx| {
                tx.execute(
                    "UPDATE s_content_publications SET batch_id='batch3-other'",
                    [],
                )
                .unwrap();
            },
        );
        assert_tamper_rejected(
            &mut domain,
            "发布索引根长度不足 32 字节（CHECK 被旁路）",
            |tx| {
                // 存量行不证明满足 CHECK 声明：显式旁路约束后写入 16 字节根，审计仍须拒绝。
                tx.execute_batch("PRAGMA ignore_check_constraints=ON")
                    .unwrap();
                tx.execute(
                    "UPDATE s_content_publications SET manifest_root=?1",
                    params![&[7u8; 16][..]],
                )
                .unwrap();
            },
        );
    }

    #[test]
    fn s3_audit_rejects_unknown_tables_and_mixed_domain_markers() {
        let mut domain = published_domain();
        assert_tamper_rejected(&mut domain, "出现未登记的未知表", |tx| {
            tx.execute("CREATE TABLE rogue_table(x)", []).unwrap();
        });
        assert_tamper_rejected(&mut domain, "控制表协议标记被改成旧域", |tx| {
            tx.execute(
                "UPDATE s_protocol_meta SET protocol_revision='s-session/2'",
                [],
            )
            .unwrap();
        });
        assert_tamper_rejected(&mut domain, "根协议标记被改成旧域", |tx| {
            tx.execute(
                "UPDATE meta SET value='s-session/2' WHERE key='protocol_revision'",
                [],
            )
            .unwrap();
        });
        assert_tamper_rejected(&mut domain, "旧私有格式登记表混入新域", |tx| {
            tx.execute(
                "CREATE TABLE s_storage_format(singleton INTEGER PRIMARY KEY, tag TEXT)",
                [],
            )
            .unwrap();
        });
    }

    /// 静态发现：`content_store::ensure_format` 只按 `singleton=1` 取值，而 CHECK 可被旁路；
    /// 模式比对与逐列类型扫描都不证明登记行数，所以第二行格式登记必须由 S3 入口自己拒掉。
    #[test]
    fn s3_audit_rejects_extra_content_format_registration() {
        let mut domain = published_domain();
        assert_tamper_rejected(
            &mut domain,
            "格式登记表被旁路 CHECK 塞入第二行",
            |tx| {
                tx.execute_batch("PRAGMA ignore_check_constraints=ON")
                    .unwrap();
                tx.execute(
                    "INSERT INTO s_content_meta(singleton,content_format) VALUES(2,'s-content/1')",
                    [],
                )
                .unwrap();
            },
        );
    }

    /// 共用核心的参数守卫：任何非 `s-session/2`/`s-session/3` 的协议版本都必须拒绝，
    /// 不允许「传什么都当 v2」的默认通道。
    #[test]
    fn shared_core_rejects_any_other_protocol_revision() {
        let domain = published_domain();
        assert!(
            v2::validate_control_history(&domain.fixture.conn, "s-session/1", Vec::new()).is_err()
        );
        assert!(
            v2::validate_control_history(&domain.fixture.conn, "s-session/2", Vec::new()).is_err()
        );
    }
}
