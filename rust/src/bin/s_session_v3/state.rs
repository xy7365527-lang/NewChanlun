//! #1394 C §5：`s-session/3` 的**读写共用完整状态审计**。
//!
//! [`audit`] 把提交口（`prepared::Runtime::prepare`）与只读入口要跑的整段旧状态核验收成**同一份**
//! 判据，全部读取在同一只读事务快照内完成，且发生在任何 SQLite 内容/索引/目录写入之前：
//!
//! 1. **控制历史**：双协议标记、唯一已认可 schema、冻结 profile/catalog 绑定、逐代发布索引闭合
//!    （[`control_audit::prepare_s3_control_history`]，唯一一份判据）；
//! 2. **真实原始账**：`raw_events` 逐序回读（含 profile 富化与身份/修订链/内容hash/receipt 自检），
//!    不另读一份副本；
//! 3. **逐代发布闭包**：manifest 原件与配方、控制来源、原始与生效账、独立出生/撤回、公开变化集合
//!    （[`history::read_history`]）；
//! 4. **未运行目录**：13 轴之外的行必须仍是 `not_implemented/not_proved/not_run/{}`；
//!    generation=0 的空初态要求**全部**行都未运行；
//! 5. **逐代来源审计**：真实源八字段原件与生成对象配方逐代对拍
//!    （[`source_audit::audit_generated`]）；本层不重跑分类；
//! 6. **目录证据**：Delta 的 `catalog_run_status`/evidence 与真实来源审计计数对拍、逐代 CC-056
//!    变化集合与真实原始账及 replaces 对拍、当前 13 轴行与本代最终生成态同根
//!    （[`catalog_evidence::verify_cut`] / [`catalog_evidence::verify_current`]）；
//! 7. **独立索引**：对象索引与非空辅助索引按当前已发布代加载，并与持久原件互相绑定
//!    （[`object_index::load`] / [`aux_index::load`]）。
//!
//! 边界如实：
//!
//! - **控制层冻结静态列/hash/版本**已由共享控制审计核过，本模块不复制第二套，只补核「从未运行」
//!   的动态列；「索引自洽」「历史闭包」「来源对拍」「目录证据」各自只声明自己核过的那一段，
//!   互不冒充。
//! - **非 OHLC 档**：来源审计与目录证据只接线 OHLC 档；非 OHLC 在此显式拒绝（其他档仍待接线，
//!   不宣布永久缩小范围），绝不因「没有 13 轴」而跳过。
//! - **原件归属**：节点读取层是 SQLite 只读源 + [`history::HashReader`] 内存派生层；派生节点只
//!   服务本次核验并随本函数内的 store 一起销毁，返回值只承载从 SQLite 行/列读出的结构与根，
//!   不把内存派生根当已持久原件带走。
//! - 本模块只读：不写任何 SQLite 行，也不把 overlay 当写入通道。

use super::*;
use newchan_rust::s_content::{self as sc, ContentRef, NodeStore};
use rusqlite::Transaction;

/// 完整状态审计的返回值：全部来自同一只读快照与 SQLite 已持久原件。
pub(super) struct AuditedState {
    /// 同一快照内的真实原始账（`raw_events` 逐序回读）。
    pub raw: Vec<Value>,
    /// 逐代已发布闭包（manifest、实际根、批次身份与快照/Delta 根）。
    pub cuts: Vec<history::PublishedCut>,
    /// 当前已发布代的对象索引（独立出生/撤回行）。
    pub objects: Vec<object_index::Entry>,
    /// 当前已发布代的辅助索引（见证/关系/观察）。
    pub auxiliary: aux_index::Index,
}

/// 只读完整状态审计：提交口与只读入口共用这一份判据。
pub(super) fn audit(tx: &Transaction<'_>) -> Result<AuditedState, String> {
    audit_until(tx, None)
}

pub(super) fn audit_until(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
) -> Result<AuditedState, String> {
    audit_bounded(tx, deadline, None)
}

pub(super) fn audit_bounded(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
) -> Result<AuditedState, String> {
    audit_with_budget(tx, deadline, max_render_bytes, None)
}
/// 计费只覆盖实际访问；完整成功后才能留作同一不可变捕获的私有用量收据。
pub(super) fn audit_with_access(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    job: &mut dyn sc::access::Access,
) -> Result<(AuditedState, audit_budget::Receipt), String> {
    let hard = job.read_deadline().map_err(err)?;
    let deadline = match (deadline, hard) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let meter = audit_budget::AuditJob::new(job);
    let state = audit_with_budget(tx, deadline, max_render_bytes, Some(&meter))?;
    audit_budget::Budget::precheck(&meter).map_err(err)?;
    Ok((state, meter.finish()))
}
fn audit_with_budget(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
) -> Result<AuditedState, String> {
    #[cfg(test)]
    trace_event("control", None);
    history::check_deadline(deadline)?;
    if let Some(b) = budget {
        b.precheck().map_err(err)?;
    }
    // 1. 控制历史：共享 schema/profile/catalog 绑定与逐代发布索引闭合（唯一判据在 control_audit）。
    let control = control_audit::prepare_s3_control_history(tx)?;
    #[cfg(test)]
    trace_event("raw", None);
    // 2. 真实原始账：逐序回读并做身份/修订链/内容hash/receipt 自检。
    let raw = read_raw_events_until(tx, deadline)?;
    #[cfg(test)]
    trace_event("history", None);
    // 3. 逐代发布闭包：消费每代控制头并核内容闭包、原始/生效账与独立变化集合（唯一判据在 history）。
    let history::IndexedHistory {
        cuts,
        objects,
        auxiliary,
    } = history::read_indexed_history(tx, &raw, control, deadline, max_render_bytes, budget)?;
    #[cfg(test)]
    trace_event("catalog_metadata", None);
    // 4. 从未运行的目录行：空初态（generation=0）要求全部行都仍是未运行。
    verify_unrun_catalog(tx, cuts.is_empty())?;
    // 5. 来源审计与目录证据只接线 OHLC 档；非 OHLC 明确拒绝，不跳过也不宣布永久缩小范围。
    let profile_id = meta_get_opt(tx, "profile_id")?.ok_or_else(|| err("缺冻结字段 profile_id"))?;
    if profile_id != tb02a::PROFILE {
        return Err(err(format!(
            "非OHLC档（profile=`{profile_id}`）：完整状态审计的来源/目录证据只接线 `{}`，其他档仍待接线，拒绝按此口径放行",
            tb02a::PROFILE
        )));
    }
    // 只读源 + 内存派生层：overlay 随本函数内的 store 一起销毁，不是 SQLite 写入通道。
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    let mut store = history::HashReader::metered(&mut reader, deadline, max_render_bytes, budget);
    // 7a. 当前代对象索引已在同次history中完整加载并绑定原件；继续用它核逐代变化。
    let mut previous = -1i64;
    for cut in &cuts {
        #[cfg(test)]
        trace_event("generated_sources", Some(cut.metadata.generation));
        history::check_deadline(deadline)?;
        let prefix = &raw[..=cut.metadata.input_frontier as usize];
        // 5'. 真实源与生成对象配方逐代对拍；本层不重跑分类。
        let counts = source_audit::audit_generated_metered(
            &mut store,
            &cut.roots.generated_state,
            prefix,
            &cut.metadata.profile_id,
            &cut.metadata.rule_revision,
            deadline,
            budget,
        )?;
        let (withdrawals, replaces) = object_index::changes_at(&objects, cut.metadata.generation)?;
        #[cfg(test)]
        trace_event("catalog_cut", Some(cut.metadata.generation));
        // 6'. Delta 目录证据与 CC-056 变化集合必须与本代真实来源、replaces 一致。
        catalog_evidence::verify_cut_metered(
            &mut store,
            &cut.metadata,
            &cut.roots.generated_state,
            &cut.delta,
            prefix,
            previous,
            &counts,
            &withdrawals,
            &replaces,
            deadline,
            budget,
        )?;
        previous = cut.metadata.input_frontier;
    }
    if let Some(last) = cuts.last() {
        #[cfg(test)]
        trace_event("catalog_current", None);
        // 6''. 当前 13 轴 catalog 行必须与本代最终生成态同根。
        let axes = field(&mut store, &last.roots.generated_state, "catalog_axes")?;
        catalog_evidence::verify_current(tx, &mut store, &axes)?;
    }
    // 7b. 辅助索引也来自同次history的完整加载，既不跨事务也不减少业务来源核验。
    history::check_deadline(deadline)?;
    #[cfg(test)]
    trace_event("complete", None);
    Ok(AuditedState {
        raw,
        cuts,
        objects,
        auxiliary,
    })
}

// #1394：仅ignored成本诊断开启的当前线程阶段记录，不进入生产代码或资源判据。
#[cfg(test)]
thread_local! {
    static AUDIT_TRACE: std::cell::RefCell<Option<(std::time::Instant, Vec<Value>)>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
fn trace_event(phase: &'static str, generation: Option<i64>) {
    AUDIT_TRACE.with(|trace| {
        if let Some((start, events)) = trace.borrow_mut().as_mut() {
            events.push(json!({"phase":phase,"generation":generation,
                "elapsed_ns":start.elapsed().as_nanos().to_string()}));
        }
    });
}
#[cfg(test)]
pub(super) fn traced<T>(action: impl FnOnce() -> T) -> (T, Vec<Value>) {
    AUDIT_TRACE.with(|trace| {
        assert!(trace.borrow().is_none(), "审计诊断不得嵌套");
        *trace.borrow_mut() = Some((std::time::Instant::now(), Vec::new()));
    });
    let result = action();
    let events = AUDIT_TRACE.with(|trace| trace.borrow_mut().take().unwrap().1);
    (result, events)
}

/// 静态列/冻结原件由共享控制审计核验；本层补核从未运行的动态列。
fn verify_unrun_catalog(tx: &Transaction<'_>, initial: bool) -> Result<(), String> {
    let mut stmt = tx
        .prepare("SELECT catalog_id,impl_status,proof_status,run_status,evidence_json FROM catalog")
        .map_err(err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(err)?;
    for row in rows {
        let (id, implemented, proof, run, evidence) = row.map_err(err)?;
        if (initial || !catalog_evidence::AXIS_IDS.contains(&id.as_str()))
            && (implemented != "not_implemented"
                || proof != "not_proved"
                || run != "not_run"
                || evidence != "{}")
        {
            return Err(err(format!("未运行目录{id}带无来源运行状态")));
        }
    }
    Ok(())
}

fn err(e: impl std::fmt::Display) -> String {
    format!("S3StateInvalid：{e}")
}

fn field(s: &mut dyn NodeStore, r: &ContentRef, key: &str) -> Result<ContentRef, String> {
    sc::path::lookup_path(s, r, &[sc::path::PathStep::Field(key.into())]).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::prepared::tests::Fixture;

    /// 用真实 OHLC 夹具连接跑一次完整状态审计（只读事务，审计不写任何行）。
    fn audit_now(f: &mut Fixture) -> Result<AuditedState, String> {
        let tx = f.conn.transaction().map_err(err)?;
        audit(&tx)
    }

    /// 断言损坏状态被拒绝，并取回拒绝理由。
    fn audit_error(f: &mut Fixture) -> String {
        match audit_now(f) {
            Ok(_) => panic!("损坏状态必须被拒绝"),
            Err(error) => error,
        }
    }

    /// 13 轴之外的目录行（未运行行）之一；轴集合只用 `catalog_evidence::AXIS_IDS`，不另抄一份。
    fn non_axis_catalog_id(f: &Fixture) -> String {
        let mut stmt = f
            .conn
            .prepare("SELECT catalog_id FROM catalog ORDER BY catalog_id")
            .unwrap();
        let ids = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        ids.into_iter()
            .find(|id| !catalog_evidence::AXIS_IDS.contains(&id.as_str()))
            .expect("目录必须含 13 轴之外的行")
    }

    fn publish_one(f: &mut Fixture) {
        let (e, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        assert_eq!(f.publish(&e, &token).unwrap().generation, 1);
    }

    /// generation=0 的空初态也必须被真实核过：控制域/原始账/目录都读，空初态不是「什么都不查」。
    #[test]
    fn generation_zero_empty_state_is_audited_and_still_checks_control_raw_and_catalog() {
        let mut f = Fixture::new();
        let audited = audit_now(&mut f).unwrap();
        assert!(audited.raw.is_empty());
        assert!(audited.cuts.is_empty());
        assert!(audited.objects.is_empty());
        // 真实接纳（未发布）后：原始账必须被核到，cuts 不得凭空出现。
        f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        let audited = audit_now(&mut f).unwrap();
        assert_eq!(audited.raw.len(), 1);
        assert!(audited.cuts.is_empty());
        // 空初态下未运行目录行不得被赋状态（generation=0 要求全部行都未运行）。
        let id = non_axis_catalog_id(&f);
        f.conn
            .execute(
                "UPDATE catalog SET run_status='run' WHERE catalog_id=?1",
                params![id],
            )
            .unwrap();
        let error = audit_error(&mut f);
        assert!(error.contains("未运行目录"), "{error}");
    }

    /// 只读入口：同一审计器在 `PRAGMA query_only=ON` 连接上必须成功。
    /// 该连接上任何写入都会失败，故成功即证明审计全程只有读。
    #[test]
    fn published_state_audits_on_query_only_connection() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let (e, token) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        assert_eq!(f.publish(&e, &token).unwrap().generation, 2);
        let baseline = audit_now(&mut f).unwrap();
        assert_eq!(baseline.raw.len(), 2);
        assert_eq!(baseline.cuts.len(), 2);
        assert!(!baseline.objects.is_empty());
        let mut reader = Connection::open(f.db_path()).unwrap();
        reader.pragma_update(None, "query_only", true).unwrap();
        let tx = reader.transaction().unwrap();
        let audited = audit(&tx).unwrap();
        assert_eq!(audited.raw.len(), baseline.raw.len());
        assert_eq!(audited.cuts.len(), baseline.cuts.len());
        assert_eq!(audited.objects.len(), baseline.objects.len());
        assert_eq!(
            audited.auxiliary.witnesses.len(),
            baseline.auxiliary.witnesses.len()
        );
        // audit 内的派生 overlay 已销毁；所有返回根仍须直接从 SQLite 完整读取。
        let mut persisted = content_store::ReadStore::new(&tx).unwrap();
        for cut in &audited.cuts {
            for root in [
                &cut.metadata.scope,
                &cut.snapshot,
                &cut.delta,
                &cut.roots.generated_state,
            ] {
                sc::verify(&mut persisted, root).unwrap();
            }
            for root in [&cut.roots.publication_template, &cut.roots.delta_template] {
                sc::verify_template(&mut persisted, root).unwrap();
            }
        }
        for object in &audited.objects {
            for root in [
                object.generated.content_ref(),
                object.internal.content_ref(),
                object.immutable.content_ref(),
                object.birth_wire.clone(),
            ] {
                sc::verify(&mut persisted, &root).unwrap();
            }
        }
        for witness in &audited.auxiliary.witnesses {
            sc::verify(&mut persisted, &witness.generated).unwrap();
            sc::verify(&mut persisted, &witness.wire).unwrap();
        }
        for relation in &audited.auxiliary.relations {
            sc::verify(&mut persisted, &relation.generated).unwrap();
        }
        for observation in &audited.auxiliary.observations {
            sc::verify(&mut persisted, &observation.generated).unwrap();
            sc::verify(&mut persisted, &observation.wire).unwrap();
        }
    }

    /// 历史根被改：控制索引行仍是 32 字节（控制审计照样过），但内容闭包解析不到该根 → 明确拒绝。
    #[test]
    fn damaged_published_history_root_is_rejected_by_audit() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        audit_now(&mut f).unwrap();
        let changed = f
            .conn
            .execute(
                "UPDATE s_content_publications SET snapshot_root=zeroblob(32)",
                [],
            )
            .unwrap();
        assert_eq!(changed, 1);
        let error = audit_error(&mut f);
        assert!(error.contains("HistoryInvalid"), "{error}");
    }

    /// 原始 source 内容被改（OHLC 与接纳时内容 hash 不再一致）→ 明确拒绝。
    #[test]
    fn damaged_raw_source_is_rejected_by_audit() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        audit_now(&mut f).unwrap();
        let changed = f
            .conn
            .execute(
                "UPDATE raw_ohlc SET high='13' WHERE identity_key=(SELECT identity_key FROM raw_events WHERE seq=0) AND revision=1",
                [],
            )
            .unwrap();
        assert_eq!(changed, 1);
        let error = audit_error(&mut f);
        assert!(error.contains("内容hash"), "{error}");
    }

    /// 当前目录（13 轴行）与生成态轴值不一致 → 明确拒绝。
    #[test]
    fn damaged_current_catalog_row_is_rejected_by_audit() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        audit_now(&mut f).unwrap();
        let changed = f
            .conn
            .execute(
                "UPDATE catalog SET run_status='not_run' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        assert_eq!(changed, 1);
        let error = audit_error(&mut f);
        assert!(error.contains("与轴值不同"), "{error}");
    }

    /// 发布之后，13 轴之外的未运行行仍必须保持 `not_run`；被赋状态即拒绝。
    #[test]
    fn unrun_catalog_row_assigned_status_is_rejected_by_audit() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let id = non_axis_catalog_id(&f);
        f.conn
            .execute(
                "UPDATE catalog SET run_status='run' WHERE catalog_id=?1",
                params![id],
            )
            .unwrap();
        let error = audit_error(&mut f);
        assert!(error.contains("未运行目录"), "{error}");
    }
}
