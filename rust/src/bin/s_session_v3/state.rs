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

/// #1394：第七资源门仍计跨请求审计源保留字节。
/// HashReader/overlay、source_audit producer 均是 audit 调用局部；AuditedState 不存入
/// Driver/prepared Runtime。当前这条路径的跨请求保留因此为 0，生成缓存和临时 heap 不在此量纲。
/// 若以后增加审计源 keeper，必须在此按原源成本计量并驱逐，不能继续宣称结构零保留。
pub(super) const AUDIT_SOURCE_RETAINED_BYTES: usize = 0;

pub(super) fn check_audit_source_retention(cap: usize) -> Result<(), String> {
    if cap == 0 {
        return Err("S3WriterResourcesInvalid：audit_cache_source_bytes 必须为显式正预算".into());
    }
    if AUDIT_SOURCE_RETAINED_BYTES > cap {
        return Err(
            "S3WriterResourcesInvalid：跨请求审计源保留超过 audit_cache_source_bytes".into(),
        );
    }
    Ok(())
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
    audit_with_capture(tx, deadline, max_render_bytes, budget, false).map(|(state, _)| state)
}
fn audit_with_capture(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
) -> Result<(AuditedState, Option<Value>), String> {
    audit_with_capture_impl(
        tx,
        deadline,
        max_render_bytes,
        budget,
        enable_prototype_capture,
        #[cfg(test)]
        None,
    )
}
fn audit_with_capture_impl(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
    #[cfg(test)] owned_budget: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<(AuditedState, Option<Value>), String> {
    // 外部不能带入旧捕获；只有本次fresh audit在None/None内可新建自有读集。
    #[cfg(test)]
    let capture_started = std::time::Instant::now();
    #[cfg(test)]
    let capture = if enable_prototype_capture && budget.is_none() && deadline.is_none() {
        Some(match owned_budget {
            Some(b) => sc::AuditCapture::from_slab_for_allocation_prototype(
                content_store::capture_slab_for_prototype(
                    tx,
                    b,
                    &content_store::CaptureCancellation::never(),
                )
                .map_err(err)?,
            ),
            None => content_store::capture_for_prototype(tx).map_err(err)?,
        })
    } else {
        None
    };
    #[cfg(not(test))]
    let capture: Option<sc::AuditCapture> = {
        let _ = enable_prototype_capture;
        None
    };
    #[cfg(test)]
    let capture_ns = capture_started.elapsed().as_nanos();
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
    } = history::read_indexed_history_for_audit_captured(
        tx,
        &raw,
        control,
        deadline,
        max_render_bytes,
        budget,
        capture.as_ref(),
    )?;
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
    if budget.is_none() {
        reader.enable_audit_same_material();
    }
    let mut store = match capture.as_ref() {
        Some(c) => history::HashReader::captured(c, max_render_bytes),
        None => history::HashReader::metered(&mut reader, deadline, max_render_bytes, budget),
    };
    // 7a. 当前代对象索引已在同次history中完整加载并绑定原件；继续用它核逐代变化。
    let mut previous = -1i64;
    for cut in &cuts {
        #[cfg(test)]
        trace_event("generated_sources", Some(cut.metadata.generation));
        history::check_deadline(deadline)?;
        let prefix = &raw[..=cut.metadata.input_frontier as usize];
        // 5'. 真实源与生成对象配方逐代对拍；本层不重跑分类。
        let counts = source_audit::audit_generated_for_audit(
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
        AUDIT_TRACE.with(|trace| {
            if let Some(t) = trace.borrow_mut().as_mut() {
                t.events.push(
                    json!({"phase":"source_business_result","generation":cut.metadata.generation,
                "counts":format!("{:?}",counts),"withdrawals":withdrawals,"replaces":replaces}),
                );
            }
        });
        #[cfg(test)]
        trace_event("catalog_cut", Some(cut.metadata.generation));
        // 6'. Delta 目录证据与 CC-056 变化集合必须与本代真实来源、replaces 一致。
        catalog_evidence::verify_cut_for_audit(
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
        catalog_evidence::verify_current_for_audit(tx, &mut store, &axes)?;
    }
    // 7b. 辅助索引也来自同次history的完整加载，既不跨事务也不减少业务来源核验。
    history::check_deadline(deadline)?;
    #[cfg(test)]
    trace_event("complete", None);
    let capacity = capture.as_ref().map(|c| {
        let mut value = c.retention();
        #[cfg(test)]
        {
            value["capture_ns"] = json!(capture_ns.to_string());
        }
        value
    });
    Ok((
        AuditedState {
            raw,
            cuts,
            objects,
            auxiliary,
        },
        capacity,
    ))
}

// #1394：仅ignored成本诊断开启的当前线程阶段记录，不进入生产代码或资源判据。
#[cfg(test)]
#[derive(Default, Clone, Copy, serde::Serialize)]
struct WorkCounts {
    source_reads: u64,
    overlay_reads: u64,
    overlay_puts: u64,
    node_checks: u64,
    unique_audit_nodes: u64,
    checked_frame_bytes: u64,
}
#[cfg(test)]
struct AuditTrace {
    start: std::time::Instant,
    events: Vec<Value>,
    work: WorkCounts,
    count_nodes: bool,
}
#[cfg(test)]
thread_local! {
    static AUDIT_TRACE: std::cell::RefCell<Option<AuditTrace>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
pub(super) fn trace_event(phase: &'static str, generation: Option<i64>) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace
                .events
                .push(json!({"phase":phase,"generation":generation,
                "elapsed_ns":trace.start.elapsed().as_nanos().to_string(),"work":trace.work}));
        }
    });
}
#[cfg(test)]
pub(super) fn trace_read(overlay: bool) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace
            .borrow_mut()
            .as_mut()
            .filter(|trace| trace.count_nodes)
        {
            if overlay {
                trace.work.overlay_reads += 1;
            } else {
                trace.work.source_reads += 1;
            }
        }
    });
}
#[cfg(test)]
pub(super) fn trace_put() {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace
            .borrow_mut()
            .as_mut()
            .filter(|trace| trace.count_nodes)
        {
            trace.work.overlay_puts += 1;
        }
    });
}
#[cfg(test)]
pub(super) fn trace_checked_node(unique: bool, frame_bytes: usize) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace
            .borrow_mut()
            .as_mut()
            .filter(|trace| trace.count_nodes)
        {
            trace.work.node_checks += 1;
            trace.work.unique_audit_nodes += u64::from(unique);
            trace.work.checked_frame_bytes += frame_bytes as u64;
        }
    });
}
#[cfg(test)]
pub(super) fn traced<T>(count_nodes: bool, action: impl FnOnce() -> T) -> (T, Vec<Value>, Value) {
    AUDIT_TRACE.with(|trace| {
        assert!(trace.borrow().is_none(), "审计诊断不得嵌套");
        *trace.borrow_mut() = Some(AuditTrace {
            start: std::time::Instant::now(),
            events: Vec::new(),
            work: WorkCounts::default(),
            count_nodes,
        });
    });
    let result = action();
    let trace = AUDIT_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
    (result, trace.events, json!(trace.work))
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

/// #1394 TestOnly observation, not a writer resource policy or a memo substitute.
#[cfg(test)]
pub(super) mod cold_audit_probe {
    use super::*;

    pub(crate) fn begin(inputs: &Value) -> usize {
        // Positive capacity derived from this fixture's canonical inputs. Keeping
        // the old memo enabled ensures zero retention is not caused by disabling it.
        let source_bytes = canonical_json(inputs).len();
        assert!(source_bytes > 0);
        cache::configure(source_bytes);
        assert!(cache::retention_enabled());
        cache::counters_reset();
        source_bytes
    }

    fn legacy_state() -> Value {
        let (root, handles, originals) = cache::retained_breakdown();
        json!({"retention_enabled":cache::retention_enabled(),"retained_source_bytes":cache::retained_bytes(),
            "capture_source_bytes":cache::capture_bytes_now(),
            "root_walk_source_bytes":root,"handle_source_bytes":handles,
            "original_cert_source_bytes":originals,"batch_header_source_bytes":cache::batch_header_bytes(),
            "counters":cache::counters_snapshot()})
    }

    pub(crate) fn observe<T>(
        label: &str,
        action: impl FnOnce() -> Result<T, String>,
    ) -> (Result<T, String>, Value) {
        let before = legacy_state();
        let frames = sc::frame_calls_on_thread();
        let started = std::time::Instant::now();
        let (result, events, work) = traced(true, action);
        let after = legacy_state();
        assert_eq!(after["retained_source_bytes"], json!(0));
        assert_eq!(after["capture_source_bytes"], json!(0));
        assert_eq!(
            before, after,
            "S3 must not mutate or use the old source audit memo"
        );
        let report = json!({"phase":label,"elapsed_ns":started.elapsed().as_nanos().to_string(),
            "work":work,"s_content_f_calls":sc::frame_calls_on_thread()-frames,
            "legacy_before":before,"legacy_after":after,"ok":result.is_ok(),
            "error":result.as_ref().err(),"audit_events":events});
        println!("S3_COLD_AUDIT_PHASE {}", canonical_json(&report));
        (result, report)
    }

    pub(crate) fn storage(c: &Connection, db: &Path) -> Value {
        use std::os::unix::fs::MetadataExt;
        let (nodes, bodies): (i64, i64) = c
            .query_row(
                "SELECT count(*),coalesce(sum(length(body)),0) FROM s_content_nodes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let files: Vec<Value> = ["", "-wal", "-shm"]
            .into_iter()
            .map(|suffix| {
                let p = PathBuf::from(format!("{}{suffix}", db.display()));
                match std::fs::metadata(&p) {
                    Ok(m) => json!({"path":p,"bytes":m.len(),"allocated_bytes":m.blocks()*512}),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        json!({"path":p,"absent":true})
                    }
                    Err(e) => panic!("{}: {e}", p.display()),
                }
            })
            .collect();
        json!({"files":files,"stored_nodes":nodes,
            "stored_node_framed_bytes":bodies as u128+nodes as u128*(sc::DOMAIN_SEPARATOR.len()+9) as u128,
            "sampling":"after operation; not peak, actual wire traffic or a per-job node read count"})
    }

    pub(crate) fn binding(c: &Connection, inputs: &Value, memo_cap: usize) -> Value {
        let clock = v2::stored_clock(c).unwrap();
        json!({"scope":"TestOnly two-input real OHLC production paths; no serve, no writer limits acceptance, no full9011",
            "inputs":inputs,"inputs_sha256":sha256_hex(canonical_json(inputs).as_bytes()),
            "clock":clock.value,"clock_sha256":clock.hash,
            "profile_hash":meta_get_opt(c,"profile_hash").unwrap(),
            "catalog_hash":meta_get_opt(c,"catalog_hash").unwrap(),
            "state_source_sha256":sha256_hex(include_bytes!("state.rs")),
            "driver_source_sha256":sha256_hex(include_bytes!("driver.rs")),
            "reported_code_revision":std::env::var("S3_COLD_AUDIT_CODE_REVISION").ok(),
            "legacy_memo_test_cap_source_bytes":memo_cap,
            "cap_scope":"test observer only; not s_resources or Q/job limit mapping",
            "seven_writer_fields_changed":false,"server_started":false,
            "source_reads_scope":"actual HashReader fallback reads to SQLite; excludes control/raw SQL and generator TxStore",
            "f_calls_scope":"all measured-thread F calls, including generated/derived hashes; not unique nodes",
            "generator_lane_zero_claim":false,"rss_zero_claim":false})
    }

    pub(crate) fn archive(c: &Connection, db: &Path, label: &str, report: &Value) {
        println!("S3_COLD_AUDIT_RESULT {}", canonical_json(report));
        let Ok(root) = std::env::var("S3_COLD_AUDIT_OUTPUT_DIR") else {
            return;
        };
        let dir = PathBuf::from(root).join(label);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::create_dir(&dir).expect("fresh evidence directory required");
        let (busy, _, _): (i64, i64, i64) = c
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(
            busy, 0,
            "artifact checkpoint must not hide an active reader"
        );
        std::fs::copy(db, dir.join("s.sqlite")).unwrap();
        let mut evidence = report.clone();
        evidence["archive"] = json!({"database_sha256":sha256_hex(&std::fs::read(dir.join("s.sqlite")).unwrap()),
            "checkpoint":"after all measured phases; not included in their timing/peak claim"});
        use std::io::Write;
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join("evidence.json"))
            .unwrap();
        out.write_all(canonical_json(&evidence).as_bytes()).unwrap();
        out.sync_all().unwrap();
    }

    pub(crate) fn fingerprint(s: &AuditedState) -> Value {
        json!({"raw_sha256":sha256_hex(canonical_json(&json!(s.raw)).as_bytes()),
            "cuts":s.cuts.iter().map(|cut|json!({"generation":cut.metadata.generation,
                "batch_id":cut.batch_id,"snapshot":cut.snapshot.to_transport_json(),
                "delta":cut.delta.to_transport_json()})).collect::<Vec<_>>(),
            "objects":s.objects.len(),"witnesses":s.auxiliary.witnesses.len(),
            "relations":s.auxiliary.relations.len(),"observations":s.auxiliary.observations.len()})
    }
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

    #[test]
    fn cold_audit_re_reads_after_drop_reopen_and_node_damage() {
        use super::cold_audit_probe as probe;
        let mut f = Fixture::new();
        let (a, t) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        f.publish(&a, &t).unwrap();
        let (b, t) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        f.publish(&b, &t).unwrap();
        let inputs = json!([a, b]);
        let cap = probe::begin(&inputs);
        let binding = probe::binding(&f.conn, &inputs, cap);
        let mut phases = Vec::new();
        let mut expected = None;
        for label in ["first_same_connection", "same_connection_after_drop"] {
            let tx = f.conn.transaction().unwrap();
            let (result, mut phase) = probe::observe(label, || audit(&tx));
            assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
            let audited = result.unwrap();
            let current = probe::fingerprint(&audited);
            if let Some(ref prior) = expected {
                assert_eq!(&current, prior)
            } else {
                expected = Some(current)
            }
            drop(audited);
            drop(tx);
            phase["storage_after"] = probe::storage(&f.conn, &f.db_path());
            phases.push(phase);
        }
        let mut reopened = Connection::open(f.db_path()).unwrap();
        reopened.pragma_update(None, "query_only", true).unwrap();
        let tx = reopened.transaction().unwrap();
        let (result, mut phase) = probe::observe("new_query_only_connection", || audit(&tx));
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        assert_eq!(
            probe::fingerprint(&result.unwrap()),
            *expected.as_ref().unwrap()
        );
        drop(tx);
        drop(reopened);
        phase["storage_after"] = probe::storage(&f.conn, &f.db_path());
        phases.push(phase);
        let (digest,body):(Vec<u8>,Vec<u8>) = f.conn.query_row(
            "SELECT n.digest,n.body FROM s_content_nodes n JOIN s_content_publications p ON n.digest=p.snapshot_root ORDER BY p.generation LIMIT 1",
            [],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        let mut damaged = body.clone();
        damaged[0] ^= 1;
        f.conn
            .execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![damaged, &digest],
            )
            .unwrap();
        let (result, mut phase) = probe::observe("damaged_published_node", || audit_now(&mut f));
        assert!(result.is_err());
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&f.conn, &f.db_path());
        phases.push(phase);
        f.conn
            .execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![body, &digest],
            )
            .unwrap();
        let (result, mut phase) = probe::observe("restored_original_node", || audit_now(&mut f));
        assert_eq!(probe::fingerprint(&result.unwrap()), expected.unwrap());
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&f.conn, &f.db_path());
        phases.push(phase);
        probe::archive(
            &f.conn,
            &f.db_path(),
            "state-cold-audit",
            &json!({"binding":binding,"phases":phases}),
        );
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

#[cfg(test)]
mod scoped_prototype_probe {
    use super::*;
    #[test]
    #[ignore]
    fn owned_current_material() {
        let db = std::env::var("AUDIT_PROTOTYPE_DB").unwrap();
        let mut c =
            Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let tx = c.transaction().unwrap();
        let sql = content_store::sql_reads_on_thread();
        let f = sc::frame_calls_on_thread();
        let started = std::time::Instant::now();
        let enabled = std::env::var_os("AUDIT_PROTOTYPE_CAPTURE").is_some();
        let owned_budget = std::env::var("AUDIT_PROTOTYPE_OWNED_PAYLOAD_BYTES")
            .ok()
            .map(|v| sc::capture_allocation::OwnedBudget::new(v.parse().unwrap()));
        let (result, events, work) = traced(true, || {
            audit_with_capture_impl(&tx, None, None, None, enabled, owned_budget.as_ref())
        });
        let elapsed = started.elapsed().as_nanos();
        let (s, capture) = result.unwrap();
        let cuts = s
            .cuts
            .iter()
            .map(|c| {
                format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    c.metadata, c.roots, c.batch_id, c.snapshot, c.delta
                )
            })
            .collect::<Vec<_>>();
        let complete = json!({"raw":s.raw,"cuts":cuts,"objects":format!("{:?}",s.objects),"auxiliary":format!("{:?}",s.auxiliary),"clock":v2::stored_clock(&tx).unwrap().value});
        let report = json!({"whole_audit_allocation_admitted":false,"capture_meter_after_drop":owned_budget.as_ref().map(|b|b.snapshot()),"elapsed_ns":elapsed.to_string(),"capture_ns":capture.as_ref().and_then(|v|v.get("capture_ns")).cloned(),"capture":capture,"sql_node_reads":content_store::sql_reads_on_thread()-sql,"f_calls":sc::frame_calls_on_thread()-f,"work":work,"events":events,"complete":complete,"complete_sha256":sha256_hex(canonical_json(&complete).as_bytes())});
        std::fs::write(
            std::env::var("AUDIT_PROTOTYPE_REPORT").unwrap(),
            canonical_json(&report),
        )
        .unwrap();
        println!(
            "AUDIT_PROTOTYPE elapsed_ns={} sql={} f={} output_sha={}",
            elapsed, report["sql_node_reads"], report["f_calls"], report["complete_sha256"]
        );
    }
}

#[cfg(test)]
mod audit_capture_gate_tests {
    use super::*;
    use sha2::{Digest, Sha256};
    struct Meter {
        trace: std::cell::RefCell<Sha256>,
        checks: std::cell::Cell<usize>,
        records: std::cell::Cell<usize>,
        stop: Option<usize>,
    }
    impl Meter {
        fn new(stop: Option<usize>) -> Self {
            Self {
                trace: Default::default(),
                checks: Default::default(),
                records: Default::default(),
                stop,
            }
        }
        fn event(&self, t: &[u8]) {
            self.trace.borrow_mut().update(t);
        }
        fn fingerprint(&self) -> String {
            format!("{:x}", self.trace.borrow().clone().finalize())
        }
    }
    impl audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            self.event(b"pre");
            self.checks.set(self.checks.get() + 1);
            if self.stop == Some(self.checks.get()) {
                Err(sc::CodecError::new("probe_budget_stop", "original gate"))
            } else {
                Ok(())
            }
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> {
            self.event(b"prepare");
            Ok(())
        }
        fn accept_prepared_frame(&self, f: &sc::job::CheckedFrame) -> Result<(), sc::CodecError> {
            self.event(b"accept");
            self.event(f.digest());
            Ok(())
        }
        fn node(&self, d: &sc::NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
            self.event(b"node");
            self.event(d);
            self.event(&[t]);
            self.event(b);
            sc::job::CheckedFrame::checked(d, t, b.to_vec()).map(|_| ())
        }
        fn record(&self) -> Result<(), sc::CodecError> {
            self.event(b"record");
            self.records.set(self.records.get() + 1);
            Ok(())
        }
    }
    fn fingerprint(s: &AuditedState) -> String {
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            s.raw,
            s.cuts
                .iter()
                .map(|c| format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    c.metadata, c.roots, c.batch_id, c.snapshot, c.delta
                ))
                .collect::<Vec<_>>(),
            s.objects,
            s.auxiliary
        )
    }
    #[test]
    fn audit_capture_budget_deadline_original_trace_and_outputs() {
        let mut f = prepared::tests::Fixture::new();
        let (e, t) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        f.publish(&e, &t).unwrap();
        let tx = f.conn.transaction().unwrap();
        for (metered, stop, expired) in [
            (true, None, false),
            (true, Some(333), false),
            (true, Some(10000), false),
            (false, None, false),
            (false, None, true),
        ] {
            let deadline = if metered {
                None
            } else {
                Some(
                    std::time::Instant::now()
                        + std::time::Duration::from_secs(if expired { 0 } else { 120 }),
                )
            };
            let mut baseline = None;
            for (enabled, use_owned_gate) in [(false, false), (true, false), (true, true)] {
                let owned = sc::capture_allocation::OwnedBudget::new(0);
                let m = Meter::new(stop);
                let sql = content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let result = audit_with_capture_impl(
                    &tx,
                    deadline,
                    None,
                    if metered { Some(&m) } else { None },
                    enabled,
                    use_owned_gate.then_some(&owned),
                );
                assert_eq!(owned.snapshot().allocation_attempts, 0);
                assert_eq!(owned.snapshot().rejected_layouts, 0);
                if let Ok((_, capacity)) = &result {
                    assert!(capacity.is_none(), "Some/期限不创建捕获");
                }
                let output = result
                    .as_ref()
                    .map(|(state, _)| fingerprint(state))
                    .map_err(Clone::clone);
                let observed = (
                    output,
                    content_store::sql_reads_on_thread() - sql,
                    sc::frame_calls_on_thread() - frames,
                    m.fingerprint(),
                    m.records.get(),
                    m.checks.get(),
                );
                if let Some(ref expected) = baseline {
                    assert_eq!(&observed, expected);
                } else {
                    baseline = Some(observed);
                }
            }
            let observation = baseline.unwrap();
            assert_eq!(observation.0.is_ok(), stop.is_none() && !expired);
            if metered && stop.is_none() {
                assert!(observation.4 > 0);
            }
            println!("CAPTURE_GATE metered={metered} stop={stop:?} expired={expired} sql={} f={} records={} checks={} trace={}",observation.1,observation.2,observation.4,observation.5,observation.3);
        }
    }
}
