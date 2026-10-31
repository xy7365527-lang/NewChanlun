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
use newchan_rust::s_content as sc;
#[cfg(test)]
use newchan_rust::s_content::ContentRef;
use rusqlite::Transaction;

/// #1394 仅本模块可签发；不是公开事务或跨请求缓存许可。
pub(super) struct OrdinaryAuditSeal {
    _private: (),
}

/// 完整状态审计的返回值：全部来自同一只读快照与 SQLite 已持久原件。
pub(super) struct AuditedState {
    /// 同一快照内的真实原始账（`raw_events` 逐序回读）。
    pub raw: super::business_json::RawEvents,
    pub budget: Option<sc::capture_allocation::OwnedBudget>,
    historical: HistoricalOwner,
    #[cfg(feature = "s_incremental")]
    certification: Option<HistoricalCertification>,
}

/// 全部旧历史判据成功后的纯结果；保留控制头与 Native 前缀状态，不含当前事务或提交许可。
pub(super) struct ImmutableHistoricalAudit {
    /// 全局控制已通过且全部发布头已逐代消费后的原头事实。
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub control_facts: v2::ControlFacts,
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub raw_prefixes: Option<history::NativeRawPrefixes>,
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub raw_sources: Option<source_audit::NativeRawFacts>,
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub auxiliary_facts: Option<aux_index::NativeAuxFacts>,
    /// 当前 cold 的完整身份事件表；其静态排序 rank 尚不构成追加资格。
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    pub object_facts: Option<object_index::HistorySweep>,
    /// 逐代已发布闭包（manifest、实际根、批次身份与快照/Delta 根）。
    pub cuts: sc::CodecVec<history::PublishedCut>,
    /// 当前已发布代的对象索引（独立出生/撤回行）。
    pub objects: sc::CodecVec<object_index::Entry>,
    /// 当前已发布代的辅助索引（见证/关系/观察）。
    pub auxiliary: aux_index::Index,
}
enum HistoricalOwner {
    Unique(ImmutableHistoricalAudit),
    #[cfg_attr(
        not(feature = "s_incremental"),
        allow(dead_code, reason = "#1488 再合 #1608：非 s_incremental 构建不触达")
    )]
    Shared(super::native_shared::NativeRc<ImmutableHistoricalAudit>),
}
impl std::ops::Deref for AuditedState {
    type Target = ImmutableHistoricalAudit;
    fn deref(&self) -> &Self::Target {
        match &self.historical {
            HistoricalOwner::Unique(value) => value,
            HistoricalOwner::Shared(value) => value,
        }
    }
}

/// #1394：第七资源门仍计跨请求审计源保留字节。
/// 直接audit/请求局部上下文自身不跨请求保留。Driver的Native keeper另外按实际源成本核此门。
/// 此常量仅供无keeper路径的已有测试，不是Driver的运行时读数。
#[cfg(test)]
pub(super) const AUDIT_SOURCE_RETAINED_BYTES: usize = 0;

pub(super) fn check_audit_source_retention(cap: usize) -> Result<(), String> {
    check_audit_source_retention_bytes(cap, 0)
}
pub(super) fn check_audit_source_retention_bytes(
    cap: usize,
    retained: usize,
) -> Result<(), String> {
    if cap == 0 {
        return Err("S3WriterResourcesInvalid：audit_cache_source_bytes 必须为显式正预算".into());
    }
    if retained > cap {
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
/// #1394：S 独立 Native TestOnly 臂。只从本次事务现场捕获，不接收外部 capture，
/// 不创建 OwnedBudget；捕获随本次调用释放，返回的普通 AuditedState 保持调用方原生命周期。
/// 三处写前及显式CapturedNative startup选择此臂；Ordinary/Owned startup、identity/preflight/recover保持普通。
/// 全节点捕获先于control/raw，坏库同时多错时首报顺序可能不同；读取错误不回退。
pub(super) fn audit_native_current(tx: &Transaction<'_>) -> Result<AuditedState, String> {
    audit_native_current_measured(tx, None, None)
}

pub(super) fn audit_native_current_measured(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn audit_budget::Budget>,
) -> Result<AuditedState, String> {
    #[cfg(feature = "s_incremental")]
    {
        let measure = audit_budget::NativeMeasure { budget, deadline };
        let observed = budget.is_some() || deadline.is_some();
        let capture = content_store::capture_for_request_prototype_measured(
            tx,
            observed.then_some(&measure as &dyn sc::CaptureReadObserver),
        )
        .map(|(capture, _)| capture)
        .map_err(err)?
        .enable_native_frame_reuse()
        .map_err(err)?;
        let (audited, _) = audit_core_fresh_with_policy(
            tx,
            deadline,
            None,
            budget,
            false,
            None,
            Some(&capture),
            None,
            RetentionReport::Skip,
            None,
            None,
            if observed {
                history::SourcePolicy::NativeMeasured
            } else {
                history::SourcePolicy::Ordinary
            },
        )?;
        drop(capture);
        Ok(audited)
    }
    #[cfg(not(feature = "s_incremental"))]
    {
        let _ = (tx, deadline, budget);
        Err("SNativeCaptureTestOnlyUnsupported：此构建不支持 S_CAPTURE_NATIVE_TESTONLY".into())
    }
}

/// 每阶段仍现场读取完整节点、控制与raw；同一handler的完整原件绑定成功才复用capture与纯历史。
pub(super) fn audit_native_in_request(
    tx: &Transaction<'_>,
    context: &mut super::request_audit::RequestAuditContext,
) -> Result<AuditedState, String> {
    audit_native_in_request_admitted(tx, context, None)
}

/// #1394 显式父域：已接捕获、控制/raw、history/source及索引；整体资源资格仍需独立验收。
pub(super) fn audit_native_in_request_admitted(
    tx: &Transaction<'_>,
    context: &mut super::request_audit::RequestAuditContext,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
) -> Result<AuditedState, String> {
    audit_native_in_request_measured(tx, context, work, None, None)
}

pub(super) fn audit_native_in_request_measured(
    tx: &Transaction<'_>,
    context: &mut super::request_audit::RequestAuditContext,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn audit_budget::Budget>,
) -> Result<AuditedState, String> {
    #[cfg(feature = "s_incremental")]
    {
        let measure = audit_budget::NativeMeasure { budget, deadline };
        let observed = budget.is_some() || deadline.is_some();
        let observer = observed.then_some(&measure as &dyn sc::CaptureReadObserver);
        let policy = if observed {
            history::SourcePolicy::NativeMeasured
        } else {
            history::SourcePolicy::Ordinary
        };
        // take保证任何读取/校验错误都不留下可继续使用的旧slot。
        let prior = context.take();
        let source_enabled = context.source_cache_enabled();
        let mut source_prior = context.take_source_pool();
        let result = (|| {
            let capture_fresh = || {
                let (capture, count, source_same) = if source_enabled {
                    content_store::capture_with_original_subset_for_source_cache_admitted_measured(
                        tx,
                        source_prior
                            .as_ref()
                            .map(|pool| (&pool.originals, pool.nodes)),
                        work,
                        observer,
                    )
                    .map_err(err)?
                } else {
                    let (capture, count) =
                        content_store::capture_for_request_prototype_admitted_measured(
                            tx, work, observer,
                        )
                        .map_err(err)?;
                    (capture, count, false)
                };
                let capture = capture
                    .enable_native_frame_reuse_admitted(work)
                    .map_err(err)?;
                if source_same {
                    if let Some(pool) = source_prior.as_ref() {
                        // source_same只减少无谓尝试；封闭API仍自行逐tuple比较完整原件。
                        capture
                            .seed_codec_roots_admitted(&pool.originals, work)
                            .map_err(err)?;
                    }
                }
                Ok::<_, String>((capture, count, source_same))
            };
            let (capture, node_count, prior, same_nodes, source_same) = match prior {
                Some(prior) => {
                    let same_nodes =
                        content_store::current_originals_equal_for_request_with_scratch_admitted_measured(
                            tx,
                            &prior.capture,
                            prior.node_count,
                            context.original_scan_scratch(),
                            work,
                            observer,
                        )
                        .map_err(err)?;
                    let (capture, count, source_same) = if same_nodes {
                        (prior.capture, prior.node_count, false)
                    } else {
                        // 合法材料变化才在同一事务第二次全扫描建新capture；任何Err都已直接返回。
                        let (capture, count, source_same) = capture_fresh()?;
                        // 同handler合法追加也只复用纯codec根；完整history结果仍走fresh审计。
                        capture
                            .seed_codec_roots_admitted(&prior.capture, work)
                            .map_err(err)?;
                        (capture, count, source_same)
                    };
                    (
                        capture,
                        count,
                        Some((prior.binding, prior.result, prior.source_pending)),
                        same_nodes,
                        source_same,
                    )
                }
                None => {
                    let (capture, count, source_same) = capture_fresh()?;
                    (capture, count, None, false, source_same)
                }
            };
            let control = control_audit::prepare_s3_control_history_admitted(
                tx,
                None,
                work,
                budget.and_then(|b| b.before_write()),
            )?;
            let raw = super::business_json::RawEvents::from_json_owner(
                crate::read_raw_events_until_admitted_measured(tx, deadline, work, budget)?,
            )?;
            if let Some((prior_binding, prior_result, source_pending)) = prior {
                #[cfg(test)]
                context.before_binding(tx);
                let binding = super::request_audit::HistoricalDependencyBinding::read_admitted(
                    tx,
                    &prior_result,
                    work,
                )?;
                if binding == prior_binding && same_nodes {
                    history::revalidate_current_for_reuse_measured(
                        tx,
                        raw.view(),
                        control,
                        &capture,
                        &prior_result.cuts,
                        work,
                        deadline,
                        budget,
                        policy,
                    )?;
                    #[cfg(test)]
                    trace_event("request_history_hit", None);
                    let result = prior_result;
                    context.retain(
                        binding,
                        capture,
                        node_count,
                        result.clone(),
                        true,
                        source_pending,
                    );
                    return Ok(AuditedState {
                        budget: None,
                        raw,
                        historical: HistoricalOwner::Shared(result),
                        certification: None,
                    });
                }
            }
            #[cfg(test)]
            trace_event("request_history_cold", None);
            let mut source_trial = if source_enabled {
                Some(super::cross_request_source_cache::Trial::new_admitted(
                    tx,
                    source_prior.as_ref(),
                    source_same,
                    work,
                )?)
            } else {
                None
            };
            let (audited, _) = audit_core_fresh_with_policy(
                tx,
                deadline,
                None,
                budget,
                false,
                None,
                Some(&capture),
                Some((control, raw)),
                RetentionReport::Skip,
                source_trial.as_mut(),
                work,
                policy,
            )?;
            let HistoricalOwner::Unique(result) = audited.historical else {
                unreachable!()
            };
            let result =
                super::native_shared::NativeRc::try_new_shallow(result, work).map_err(err)?;
            let binding = super::request_audit::HistoricalDependencyBinding::read_admitted(
                tx, &result, work,
            )?;
            let frontier = result
                .cuts
                .last()
                .map_or(-1, |cut| cut.metadata.input_frontier);
            let source_pending = source_trial
                .map(|trial| trial.seal_admitted(tx, frontier, work))
                .transpose()?;
            drop(source_prior.take());
            context.retain(
                binding,
                capture,
                node_count,
                result.clone(),
                false,
                source_pending,
            );
            Ok(AuditedState {
                budget: audited.budget,
                raw: audited.raw,
                historical: HistoricalOwner::Shared(result),
                certification: None,
            })
        })();
        // #1488 wide16-0058：冷根首跑失败后按旧口径无票重跑；首跑只借用、未改动旧来源池，
        // 交还keeper供重跑复用（否则冷根额度不足即静默丢失跨请求复用）。其余错误仍随本次丢弃。
        // 覆盖边界（review-0060 F3）：交还只覆盖本函数内层返回Err。若本函数已Ok（池已在Ok路径
        // drop），而外层codec_work().run尾检、s_allocation的probe::captured或cold_root_probe::
        // inject_failure（后两者仅cfg(test)）报Err，池已消费、重跑从冷开始——与修复前同口径，非回归。
        if result.is_err() {
            context.return_source_pool_for_cold_rerun(source_prior.take());
        }
        drop(source_prior);
        result
    }
    #[cfg(not(feature = "s_incremental"))]
    {
        let _ = (tx, context, work, deadline, budget);
        Err("SNativeCaptureTestOnlyUnsupported：此构建不支持 S_CAPTURE_NATIVE_TESTONLY".into())
    }
}

/// #1394 隔离测试入口：每次新capture与完整状态audit，仅精确绑定的旧来源Counts复用。
/// keeper不接Driver/serve，预算不来自公共S7；任一错误均清除旧池及部分新证明。
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn audit_native_with_source_keeper_for_testonly(
    tx: &Transaction<'_>,
    keeper: &mut super::cross_request_source_cache::Keeper,
) -> Result<AuditedState, String> {
    let prior = keeper.take();
    let result = (|| {
        let (capture, nodes, same_nodes) =
            content_store::capture_with_original_subset_for_source_cache(
                tx,
                prior.as_ref().map(|pool| (&pool.originals, pool.nodes)),
            )
            .map_err(err)?;
        let capture = capture.enable_native_frame_reuse().map_err(err)?;
        if same_nodes {
            if let Some(pool) = prior.as_ref() {
                capture
                    .seed_codec_roots_admitted(&pool.originals, None)
                    .map_err(err)?;
            }
        }
        let mut trial =
            super::cross_request_source_cache::Trial::new(tx, prior.as_ref(), same_nodes)?;
        let (audited, _) = audit_core_fresh_inner(
            tx,
            None,
            None,
            None,
            false,
            None,
            Some(&capture),
            None,
            RetentionReport::Skip,
            Some(&mut trial),
            None,
        )?;
        let frontier = audited
            .cuts
            .last()
            .map_or(-1, |cut| cut.metadata.input_frontier);
        let (next, hits, cold) =
            trial.complete(tx, capture, nodes, frontier, keeper.retention_cap())?;
        Ok::<_, String>((audited, next, hits, cold))
    })();
    match result {
        Ok((audited, next, hits, cold)) => {
            if let Err(error) = keeper.finish(prior, next, hits, cold) {
                keeper.clear();
                return Err(error);
            }
            Ok(audited)
        }
        Err(error) => {
            drop(prior);
            keeper.clear();
            Err(error)
        }
    }
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
#[cfg(feature = "s_allocation_json_testonly")]
fn with_q_source<T>(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    profile: &super::q_allocation::Profile,
    f: impl FnOnce(&sc::AuditCapture) -> Result<T, String>,
) -> Result<(T, sc::AuditCapture), String> {
    profile.validate()?;
    history::check_deadline(deadline)?;
    let slab = content_store::capture_slab_for_prototype(
        tx,
        &profile.budget,
        &content_store::CaptureCancellation::until(deadline),
    )
    .map_err(err)?;
    let source = sc::AuditCapture::from_slab_for_allocation_prototype(slab);
    let result = history::with_overlay_prototype(
        history::OverlayPrototype {
            slots: profile.overlay_slots,
            bytes: profile.overlay_bytes,
            budget: profile.budget.clone(),
            peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
        },
        || f(&source),
    )?;
    Ok((result, source))
}
pub(crate) fn audit_bounded_with_allocation(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max: Option<u64>,
    profile: &super::q_allocation::Profile,
) -> Result<(AuditedState, sc::AuditCapture), String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    return with_q_source(tx, deadline, profile, |source| {
        audit_core(
            tx,
            deadline,
            max,
            None,
            false,
            Some(&profile.budget),
            Some(source),
            RetentionReport::Skip,
        )
        .map(|(s, _)| s)
    });
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    {
        let _ = (tx, deadline, max, profile);
        Err("prototype_audit_allocation_incomplete：Q分配原型未启用".into())
    }
}
pub(crate) fn audit_with_access_and_allocation(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max: Option<u64>,
    job: &mut dyn sc::access::Access,
    profile: &super::q_allocation::Profile,
) -> Result<(AuditedState, audit_budget::Receipt, sc::AuditCapture), String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        let hard = job.read_deadline().map_err(err)?;
        let deadline = match (deadline, hard) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let meter = audit_budget::AuditJob::with_allocation(job, Some(&profile.budget));
        let (state, source) = with_q_source(tx, deadline, profile, |source| {
            audit_core(
                tx,
                deadline,
                max,
                Some(&meter),
                false,
                Some(&profile.budget),
                Some(source),
                RetentionReport::Skip,
            )
            .map(|(s, _)| s)
        })?;
        audit_budget::Budget::precheck(&meter).map_err(err)?;
        Ok((state, meter.try_finish()?, source))
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    {
        let _ = (tx, deadline, max, job, profile);
        Err("prototype_audit_allocation_incomplete：Q分配原型未启用".into())
    }
}
pub(super) fn audit_with_budget(
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
    audit_core(
        tx,
        deadline,
        max_render_bytes,
        budget,
        enable_prototype_capture,
        #[cfg(test)]
        owned_budget,
        #[cfg(not(test))]
        None,
        None,
        RetentionReport::Collect,
    )
}
// #1394：具名捕获诊断才生成普通 JSON；正式 Owned/native 审计不构造后再丢弃。
#[derive(Clone, Copy)]
enum RetentionReport {
    #[cfg_attr(
        all(not(test), not(feature = "s_incremental")),
        allow(
            dead_code,
            reason = "#1608：默认 feature 的非测试构建不触达；测试与 s_incremental 构建使用"
        )
    )]
    Skip,
    Collect,
}
fn audit_core(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
    owned_budget: Option<&sc::capture_allocation::OwnedBudget>,
    external_capture: Option<&sc::AuditCapture>,
    retention_report: RetentionReport,
) -> Result<(AuditedState, Option<Value>), String> {
    audit_core_fresh(
        tx,
        deadline,
        max_render_bytes,
        budget,
        enable_prototype_capture,
        owned_budget,
        external_capture,
        None,
        retention_report,
    )
}
fn audit_core_fresh(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
    owned_budget: Option<&sc::capture_allocation::OwnedBudget>,
    external_capture: Option<&sc::AuditCapture>,
    fresh: Option<(v2::ControlHistory, super::business_json::RawEvents)>,
    retention_report: RetentionReport,
) -> Result<(AuditedState, Option<Value>), String> {
    audit_core_fresh_inner(
        tx,
        deadline,
        max_render_bytes,
        budget,
        enable_prototype_capture,
        owned_budget,
        external_capture,
        fresh,
        retention_report,
        #[cfg(feature = "s_incremental")]
        None,
        None,
    )
}

fn audit_core_fresh_inner(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
    owned_budget: Option<&sc::capture_allocation::OwnedBudget>,
    external_capture: Option<&sc::AuditCapture>,
    fresh: Option<(v2::ControlHistory, super::business_json::RawEvents)>,
    retention_report: RetentionReport,
    #[cfg(feature = "s_incremental")] source_cache: Option<
        &mut super::cross_request_source_cache::Trial<'_>,
    >,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
) -> Result<(AuditedState, Option<Value>), String> {
    audit_core_fresh_with_policy(
        tx,
        deadline,
        max_render_bytes,
        budget,
        enable_prototype_capture,
        owned_budget,
        external_capture,
        fresh,
        retention_report,
        #[cfg(feature = "s_incremental")]
        source_cache,
        work,
        history::SourcePolicy::Ordinary,
    )
}

fn audit_core_fresh_with_policy(
    tx: &Transaction<'_>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn audit_budget::Budget>,
    enable_prototype_capture: bool,
    owned_budget: Option<&sc::capture_allocation::OwnedBudget>,
    external_capture: Option<&sc::AuditCapture>,
    fresh: Option<(v2::ControlHistory, super::business_json::RawEvents)>,
    retention_report: RetentionReport,
    #[cfg(feature = "s_incremental")] mut source_cache: Option<
        &mut super::cross_request_source_cache::Trial<'_>,
    >,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
    policy: history::SourcePolicy,
) -> Result<(AuditedState, Option<Value>), String> {
    // 外部不能带入旧捕获；只有本次fresh audit在None/None内可新建自有读集。
    #[cfg(test)]
    let capture_started = std::time::Instant::now();
    #[cfg(test)]
    let constructed_capture = if enable_prototype_capture && budget.is_none() && deadline.is_none()
    {
        Some(match owned_budget {
            Some(b) => sc::AuditCapture::from_slab_for_allocation_prototype(
                content_store::capture_slab_for_prototype(
                    tx,
                    b,
                    &content_store::CaptureCancellation::never(),
                )
                .map_err(err)?,
            ),
            None => {
                let capture = content_store::capture_for_prototype(tx).map_err(err)?;
                #[cfg(feature = "s_incremental")]
                let capture = if max_render_bytes.is_none() {
                    capture.enable_native_frame_reuse().map_err(err)?
                } else {
                    capture
                };
                capture
            }
        })
    } else {
        None
    };
    #[cfg(not(test))]
    let constructed_capture: Option<sc::AuditCapture> = {
        let _ = enable_prototype_capture;
        None
    };
    let capture = external_capture.or(constructed_capture.as_ref());
    let ordinary_materials_qualified = budget.is_none()
        && deadline.is_none()
        && max_render_bytes.is_none()
        && !enable_prototype_capture
        && owned_budget.is_none()
        && capture.is_none()
        && external_capture.is_none()
        && work.is_none()
        && fresh.is_none()
        && !cfg!(feature = "s_incremental");
    #[cfg(test)]
    let capture_ns = capture_started.elapsed().as_nanos();
    #[cfg(all(test, feature = "s_allocation_sqlite_testonly"))]
    if capture.is_some() {
        shared_sqlite_probe::arm_after_owned_capture();
    }
    #[cfg(test)]
    trace_event("control", None);
    history::check_deadline(deadline)?;
    if let Some(b) = budget {
        b.precheck().map_err(err)?;
    }
    // 1. 控制历史：共享 schema/profile/catalog 绑定与逐代发布索引闭合（唯一判据在 control_audit）。
    #[cfg(feature = "s_allocation_json_testonly")]
    let control_owner = owned_budget.filter(|_| {
        capture
            .as_ref()
            .is_some_and(|c| c.is_allocation_prototype())
    });
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    let control_owner = None;
    let (control, raw) = match fresh {
        Some(fresh) => fresh,
        None => {
            let control = control_audit::prepare_s3_control_history_admitted(
                tx,
                control_owner,
                work,
                budget.and_then(|b| b.before_write()),
            )?;
            #[cfg(test)]
            trace_event("raw", None);
            // 2. 真实原始账：逐序回读并做身份/修订链/内容hash/receipt 自检。
            #[cfg(feature = "s_allocation_json_testonly")]
            let raw = match owned_budget.filter(|_| {
                capture
                    .as_ref()
                    .is_some_and(|c| c.is_allocation_prototype())
            }) {
                Some(b) => super::business_json::read_owned(tx, deadline, b)?,
                None => super::business_json::RawEvents::from_json_owner(
                    crate::read_raw_events_until_admitted_measured(
                        tx,
                        deadline,
                        work,
                        if policy == history::SourcePolicy::NativeMeasured {
                            budget
                        } else {
                            None
                        },
                    )?,
                )?,
            };
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            let raw = super::business_json::RawEvents::from_json_owner(
                crate::read_raw_events_until_admitted_measured(
                    tx,
                    deadline,
                    work,
                    if policy == history::SourcePolicy::NativeMeasured {
                        budget
                    } else {
                        None
                    },
                )?,
            )?;
            (control, raw)
        }
    };
    #[cfg(test)]
    trace_event("history", None);
    // 3. 逐代发布闭包：消费每代控制头并核内容闭包、原始/生效账与独立变化集合（唯一判据在 history）。
    #[cfg(feature = "s_incremental")]
    // 同 capture、同一次 cold audit 的私有字段交接，不进入公共资源或 Owned 布局。
    let mut native_records = capture
        .filter(|c| {
            !c.is_allocation_prototype()
                && owned_budget.is_none()
                && (policy == history::SourcePolicy::NativeMeasured
                    || (budget.is_none() && deadline.is_none()))
                && max_render_bytes.is_none()
        })
        .and_then(|capture| history::NativeRecordFields::new_admitted(capture, work));
    #[cfg(not(feature = "s_incremental"))]
    let mut native_records: Option<history::NativeRecordFields<'_>> = None;
    let mut materials = ordinary_materials_qualified.then(|| {
        super::ordinary_materials::OrdinaryAuditMaterials::new(OrdinaryAuditSeal { _private: () })
    });
    let verified_history = history::read_verified_public_history(
        tx,
        raw.view(),
        control,
        deadline,
        max_render_bytes,
        budget,
        capture,
        native_records.as_mut(),
        work,
        materials.as_mut(),
        policy,
    )?;
    let history::IndexedHistory {
        cuts,
        objects,
        object_sweep,
        ..
    } = verified_history.history();
    #[cfg(test)]
    trace_event("catalog_metadata", None);
    // 4. 从未运行的目录行：空初态（generation=0）要求全部行都仍是未运行。
    verify_unrun_catalog_admitted(tx, cuts.is_empty(), control_owner, work)?;
    // 5. 来源审计与目录证据只接线 OHLC 档；非 OHLC 明确拒绝，不跳过也不宣布永久缩小范围。
    let _profile_id = audit_profile_id_admitted(tx, control_owner, work)?;
    // 只读源 + 内存派生层：overlay 随本函数内的 store 一起销毁，不是 SQLite 写入通道。
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    if policy == history::SourcePolicy::NativeMeasured || budget.is_none() {
        reader.enable_audit_same_material();
    }
    let mut reader = Some(reader);
    let mut material_source = materials
        .as_mut()
        .map(|pool| pool.source(reader.take().unwrap()));
    let source: &mut dyn sc::NodeSource = match material_source.as_mut() {
        Some(source) => source,
        None => reader.as_mut().unwrap(),
    };
    let mut store = match capture.as_ref() {
        Some(c) => history::HashReader::captured_for_audit(c, max_render_bytes)
            .map_err(err)?
            .with_audit_gate(budget, deadline),
        None => history::HashReader::metered(source, deadline, max_render_bytes, budget),
    };
    store = store
        .with_source_policy(policy)
        .with_native_scope(work)
        .map_err(err)?;
    // 7a. 当前代对象索引已在同次history中完整加载并绑定原件；继续用它核逐代变化。
    #[cfg(feature = "s_incremental")]
    let mut native_sources = capture
        .filter(|c| {
            !c.is_allocation_prototype()
                && owned_budget.is_none()
                && (policy == history::SourcePolicy::NativeMeasured
                    || (budget.is_none() && deadline.is_none()))
                && max_render_bytes.is_none()
        })
        .map(|c| {
            source_audit::NativeSourceAudit::new_with_records_measured(
                c,
                raw.view(),
                native_records.take(),
                work,
                budget,
                deadline,
                policy,
            )
        })
        .transpose()?;
    #[cfg(not(feature = "s_incremental"))]
    let mut native_sources: Option<source_audit::NativeSourceAudit<'_>> = None;
    #[cfg(feature = "s_incremental")]
    let allow_ordinary = source_cache.is_none();
    #[cfg(not(feature = "s_incremental"))]
    let allow_ordinary = true;
    let mut ordinary_source = if allow_ordinary {
        verified_history.ordinary_source_session()
    } else {
        None
    };
    let mut previous = -1i64;
    for cut in cuts {
        let mut audit_cut =
            |store: &mut history::HashReader<'_>,
             proofs: Option<&mut source_audit::OrdinaryProofs>| {
                audit_one_cut_sources(
                    cut,
                    raw.view(),
                    store,
                    &objects,
                    object_sweep.as_ref(),
                    &mut native_sources,
                    proofs,
                    previous,
                    deadline,
                    budget,
                    #[cfg(feature = "s_incremental")]
                    source_cache.as_deref_mut(),
                    #[cfg(feature = "s_incremental")]
                    None,
                    #[cfg(feature = "s_incremental")]
                    false,
                )
            };
        if let Some(session) = ordinary_source.as_mut() {
            session.audit_cut(
                tx,
                cut,
                deadline,
                max_render_bytes,
                budget,
                |store, proofs| audit_cut(store, Some(proofs)),
            )?;
        } else if verified_history.qualified() {
            verified_history.audit_cut(tx, cut, deadline, max_render_bytes, budget, |store| {
                audit_cut(store, None)
            })?;
        } else {
            audit_cut(&mut store, None)?;
        }
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
    let capacity = capture
        .filter(|_| matches!(retention_report, RetentionReport::Collect))
        .map(|c| {
            #[cfg(test)]
            RETENTION_REPORT_CALLS.with(|calls| calls.set(calls.get() + 1));
            let value = c.retention();
            #[cfg(test)]
            let mut value = value;
            #[cfg(test)]
            {
                value["capture_ns"] = json!(capture_ns.to_string());
            }
            value
        });
    // 当前 Capture memo 的 Raw 借用到此结束，输出可转移已审原始账。
    drop(ordinary_source);
    let raw_sources = native_sources.map(source_audit::NativeSourceAudit::into_raw_facts);
    let history::IndexedHistory {
        control_facts,
        raw_prefixes,
        auxiliary_facts,
        cuts,
        objects,
        auxiliary,
        object_sweep,
    } = verified_history.into_history();
    Ok((
        AuditedState {
            budget: raw.budget().cloned(),
            raw,
            #[cfg(feature = "s_incremental")]
            certification: None,
            historical: HistoricalOwner::Unique(ImmutableHistoricalAudit {
                control_facts,
                raw_prefixes,
                raw_sources,
                auxiliary_facts,
                object_facts: object_sweep,
                cuts,
                objects,
                auxiliary,
            }),
        },
        capacity,
    ))
}

/// 与历史 manifest 核配套的单代来源、Counts、变化集合及 catalog 判据。
fn audit_one_cut_sources(
    cut: &history::PublishedCut,
    raw: super::business_json::RawEventsView<'_>,
    store: &mut history::HashReader<'_>,
    objects: &[object_index::Entry],
    object_sweep: Option<&object_index::HistorySweep>,
    native_sources: &mut Option<source_audit::NativeSourceAudit<'_>>,
    proofs: Option<&mut source_audit::OrdinaryProofs>,
    previous: i64,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn audit_budget::Budget>,
    #[cfg(feature = "s_incremental")] mut source_cache: Option<
        &mut super::cross_request_source_cache::Trial<'_>,
    >,
    #[cfg(feature = "s_incremental")] owner_source: Option<&super::audit_owner::OwnerSourceCheck>,
    #[cfg(feature = "s_incremental")] certified: bool,
) -> Result<(), String> {
    #[cfg(test)]
    trace_event("generated_sources", Some(cut.metadata.generation));
    history::check_deadline(deadline)?;
    let prefix = raw.prefix(cut.metadata.input_frontier as usize + 1)?;
    // 5'. 真实源与生成对象配方逐代对拍；本层不重跑分类。
    #[cfg(feature = "s_incremental")]
    let cached_counts = match owner_source {
        Some(source) if certified => Some(source.counts_for_cut(
            cut.metadata.view(),
            cut.roots.generated_state.view(),
            prefix.len(),
            deadline,
        )?),
        Some(source) => Some(source.counts_for(cut.roots.generated_state.view(), prefix.len())?),
        None if certified => return Err(err("历史认证在线缺同请求来源收据")),
        None => source_cache
            .as_deref_mut()
            .and_then(|cache| cache.lookup(cut)),
    };
    #[cfg(not(feature = "s_incremental"))]
    let cached_counts: Option<source_audit::Counts> = None;
    if proofs.is_some() && (cached_counts.is_some() || native_sources.is_some()) {
        return Err(err(
            "ordinary来源证明不得与cached_counts/owner_source/Native复用并用",
        ));
    }
    let counts = if let Some(proofs) = proofs {
        source_audit::audit_generated_with_proofs(
            store,
            cut.roots.generated_state.view(),
            prefix,
            &cut.metadata.profile_id,
            &cut.metadata.rule_revision,
            deadline,
            budget,
            proofs,
        )?
    } else if let Some(counts) = cached_counts {
        if let Some(native) = native_sources.as_mut() {
            native.advance_raw_facts(prefix.len())?;
        }
        counts
    } else {
        match native_sources.as_mut() {
            Some(native) => native.audit(
                &cut.roots.generated_state,
                prefix.len(),
                &cut.metadata.profile_id,
                &cut.metadata.rule_revision,
            )?,
            None => source_audit::audit_generated_for_audit(
                store,
                &cut.roots.generated_state,
                prefix,
                &cut.metadata.profile_id,
                &cut.metadata.rule_revision,
                deadline,
                budget,
            )?,
        }
    };
    #[cfg(feature = "s_incremental")]
    if let Some(cache) = source_cache.as_deref_mut() {
        cache.record_admitted(cut, &counts, store.native_work_scope())?;
    }
    let allocation = super::audit_store::CodecRead::allocation_budget(store);
    let (withdrawals, replaces) = match &object_sweep {
        Some(sweep) => sweep.changes_admitted(store, objects, cut.metadata.generation)?,
        None => object_index::changes_at_admitted(
            store,
            objects,
            cut.metadata.generation,
            allocation.as_ref(),
        )?,
    };
    #[cfg(test)]
    AUDIT_TRACE.with(|trace| {
        if let Some(t) = trace.borrow_mut().as_mut() {
            t.events.push(
                json!({"phase":"source_business_result","generation":cut.metadata.generation,
            "counts":format!("{:?}",counts),"withdrawals":withdrawals.view(),"replaces":replaces.view()}),
            );
        }
    });
    #[cfg(test)]
    trace_event("catalog_cut", Some(cut.metadata.generation));
    // 6'. Delta 目录证据与 CC-056 变化集合必须与本代真实来源、replaces 一致。
    catalog_evidence::verify_cut_for_audit(
        store,
        cut.metadata.view(),
        &cut.roots.generated_state,
        &cut.delta,
        prefix,
        previous,
        &counts,
        withdrawals.view(),
        replaces.view(),
        deadline,
        budget,
        native_sources.as_ref().map(|_| (previous + 1) as usize),
    )?;
    Ok(())
}

#[cfg(feature = "s_incremental")]
struct HistoricalSourceFold<'a> {
    raw: super::business_json::RawEventsView<'a>,
    source: Option<source_audit::NativeSourceAudit<'a>>,
    generation: i64,
    previous: i64,
    deadline: Option<std::time::Instant>,
    budget: Option<&'a dyn audit_budget::Budget>,
    owner_source: Option<&'a super::audit_owner::OwnerSourceCheck>,
    certified_online: bool,
}

#[cfg(feature = "s_incremental")]
impl history::CutSourceVerifier for HistoricalSourceFold<'_> {
    fn verify(
        &mut self,
        cut: &history::PublishedCut,
        store: &mut history::HashReader<'_>,
        objects: &[object_index::Entry],
        sweep: Option<&object_index::HistorySweep>,
    ) -> Result<(), String> {
        if cut.metadata.generation != self.generation + 1 {
            return Err(err("历史认证代号不连续"));
        }
        audit_one_cut_sources(
            cut,
            self.raw,
            store,
            objects,
            sweep,
            &mut self.source,
            None,
            self.previous,
            self.deadline,
            self.budget,
            None,
            self.owner_source,
            self.certified_online,
        )?;
        history::check_deadline(self.deadline)?;
        self.generation = cut.metadata.generation;
        self.previous = cut.metadata.input_frontier;
        #[cfg(test)]
        trace_event("h1_certified_cut", Some(self.generation));
        Ok(())
    }
}

#[cfg(feature = "s_incremental")]
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "#1481：历史认证组件，正式入口待 #1645/#1491")
)]
pub(super) struct HistoricalCertificate {
    audited: AuditedState,
}

/// 历史认证标记。`qualified` 为 None 时是 36T 来源收据认证组件（无 P6 台账）；
/// 正式三处准入切换（#1645/#1491 门内）时该档随旧入口一起退役，不并存为两档。
#[cfg(feature = "s_incremental")]
struct HistoricalCertification {
    qualified: Option<QualifiedCertification>,
}

/// #1481 39P：H1 消费 P6 完整资格的来源。资格只经 `Runtime` 的私有点查接口取得，
/// H1 不持 Ledger 引用。内容库与二进制身份由安装台账的调用方声明；规则取库 meta。
#[cfg(feature = "s_incremental")]
pub(super) struct QualifiedSource<'a> {
    pub runtime: &'a super::prepared::Runtime,
    pub content_identity: String,
    pub binary_commit: String,
}

/// 冷起步交给资格组件的来源与父账：持久载荷与临时缓冲都从这份父账持票出生（40B B2）。
#[cfg(feature = "s_incremental")]
pub(super) struct QualifiedSeed<'a> {
    pub source: QualifiedSource<'a>,
    pub budget: &'a super::audit_owner::native::Budget,
}

/// 本写者身份的持久副本；全部字符串按精确容量在父账许可内复制。
#[cfg(feature = "s_incremental")]
struct QualifiedIdentity {
    rule_revision: String,
    content_identity: String,
    binary_commit: String,
}
#[cfg(feature = "s_incremental")]
impl super::audit_retention::Retained for QualifiedIdentity {
    fn heap_bytes(&self) -> usize {
        super::audit_retention::sum([
            self.rule_revision.capacity(),
            self.content_identity.capacity(),
            self.binary_commit.capacity(),
        ])
    }
}
#[cfg(feature = "s_incremental")]
impl super::audit_retention::Retained for super::audit_ledger::Qualification {
    fn heap_bytes(&self) -> usize {
        super::audit_retention::sum([
            self.roots.capacity(),
            self.fold_digest.capacity(),
            self.verifier_revision.capacity(),
            self.rule_revision.capacity(),
            self.content_identity.capacity(),
            self.binary_commit.capacity(),
        ])
    }
}

/// H1 对 P6 完整资格的消费前沿，随完整候选跨事务保留，载荷持父账许可。
/// `Empty`：历史尚无已发布代，首代台账提交后才取得首份资格。
/// `Qualified(q)`：已按 `q` 逐代点查到 `q.through`，恒等于已消费的最高代。
#[cfg(feature = "s_incremental")]
pub(super) enum CertifiedFrontier {
    Empty,
    Qualified(super::audit_owner::native::Reserved<super::audit_ledger::Qualification>),
}

#[cfg(feature = "s_incremental")]
pub(super) struct QualifiedCertification {
    identity: super::audit_owner::native::Reserved<QualifiedIdentity>,
    frontier: CertifiedFrontier,
}

#[cfg(feature = "s_incremental")]
impl super::audit_retention::Retained for QualifiedCertification {
    fn heap_bytes(&self) -> usize {
        use super::audit_retention::Retained as _;
        let frontier = match &self.frontier {
            CertifiedFrontier::Empty => 0,
            CertifiedFrontier::Qualified(q) => q.heap_bytes(),
        };
        self.identity.heap_bytes().saturating_add(frontier)
    }
}
#[cfg(feature = "s_incremental")]
impl super::audit_retention::Retained for HistoricalCertification {
    fn heap_bytes(&self) -> usize {
        self.qualified
            .as_ref()
            .map_or(0, super::audit_retention::Retained::heap_bytes)
    }
}

#[cfg(feature = "s_incremental")]
impl CertifiedFrontier {
    fn generation(&self) -> i64 {
        match self {
            Self::Empty => 0,
            Self::Qualified(q) => q.through,
        }
    }
    fn qualification(&self) -> Option<&super::audit_ledger::Qualification> {
        match self {
            Self::Empty => None,
            Self::Qualified(q) => Some(&**q),
        }
    }
}

#[cfg(feature = "s_incremental")]
impl QualifiedCertification {
    /// 冷：从空前沿按代消费全部已发布 cut，只用台账当前已提交的一份资格。
    /// 身份与资格字符串在父账预留后再按精确容量复制；四根比较缓冲临时持票。
    fn fold(
        seed: QualifiedSeed<'_>,
        rule_revision: &str,
        cuts: &[history::PublishedCut],
    ) -> Result<Self, String> {
        let QualifiedSeed { source, budget } = seed;
        let identity_bytes = super::audit_retention::sum([
            rule_revision.len(),
            source.content_identity.len(),
            source.binary_commit.len(),
        ]);
        let permit = budget
            .reserve(identity_bytes)
            .map_err(|_| err("资格组件身份载荷父预算不足"))?;
        let identity = QualifiedIdentity {
            rule_revision: exact_copy(rule_revision),
            content_identity: exact_copy(&source.content_identity),
            binary_commit: exact_copy(&source.binary_commit),
        };
        let identity = finish_reserved(permit, identity)?;
        let runtime = source.runtime;
        let frontier = if cuts.is_empty() {
            CertifiedFrontier::Empty
        } else {
            let transition = Self::transition_bound(&identity);
            let permit = budget
                .reserve(transition)
                .map_err(|_| err("资格组件冷起步父预算不足"))?;
            let q = Self::read_qualification(runtime, &identity, permit)?;
            let mut generation = 0;
            for cut in cuts {
                generation = Self::step(runtime, &q, generation, cut, budget)?;
            }
            if generation != q.through {
                return Err(err("完整台账水位超前于已发布历史"));
            }
            CertifiedFrontier::Qualified(q)
        };
        Ok(Self { identity, frontier })
    }

    /// 本次交接的持久上界：新资格全部字符串；临时四根缓冲另在 `step` 内持票。
    fn transition_bound(identity: &QualifiedIdentity) -> usize {
        super::audit_ledger::Qualification::retained_bound(
            identity.rule_revision.len(),
            identity.content_identity.len(),
            identity.binary_commit.len(),
        )
    }

    /// 在线：内容与台账都已提交后，用费用终检前已持的许可从上一代前沿消费新一代。
    /// 返回保留账变化：新资格入账、旧资格与许可余量释放。
    fn advance(
        &mut self,
        runtime: &super::prepared::Runtime,
        cut: &history::PublishedCut,
        permit: super::audit_owner::native::Permit,
        budget: &super::audit_owner::native::Budget,
    ) -> Result<super::audit_retention::RetainedDelta, String> {
        let reserved = permit.bytes();
        let q = Self::read_qualification(runtime, &self.identity, permit)?;
        let generation = Self::step(runtime, &q, self.frontier.generation(), cut, budget)?;
        if generation != q.through {
            return Err(err("已提交资格水位不是刚发布的一代"));
        }
        let mut delta = super::audit_retention::RetainedDelta::default();
        delta.release_bytes(reserved);
        delta.add(&*q);
        if let Some(old) = self.frontier.qualification() {
            delta.release(old);
        }
        self.frontier = CertifiedFrontier::Qualified(q);
        Ok(delta)
    }

    /// 缺资格、部分账、Pending/Bound/Stale 均在此 Err；身份与核验判据版本逐字段核。
    /// `permit` 须已覆盖 `transition_bound`；超出上界的台账行在分配前被 P6 拒绝。
    fn read_qualification(
        runtime: &super::prepared::Runtime,
        identity: &QualifiedIdentity,
        permit: super::audit_owner::native::Permit,
    ) -> Result<super::audit_owner::native::Reserved<super::audit_ledger::Qualification>, String>
    {
        let q = runtime.audit_qualification_bounded(permit.bytes())?;
        if q.rule_revision != identity.rule_revision
            || q.content_identity != identity.content_identity
            || q.binary_commit != identity.binary_commit
        {
            return Err(err("完整资格的规则/内容库/二进制身份与本写者不符"));
        }
        if q.verifier_revision != sc::closure_delta::VERIFIER_REVISION {
            return Err(err("完整资格的核验判据版本与本二进制不符"));
        }
        finish_reserved(permit, q)
    }

    /// 单代 Δ 步：四根按模式作为认证单位逐一点查，末代四根须与资格四根逐字节相同。
    /// 未提交的一代（超过 `through`）不成为 cert；任一单位不在已认证集即拒绝，不回落完整核验。
    fn step(
        runtime: &super::prepared::Runtime,
        q: &super::audit_ledger::Qualification,
        generation: i64,
        cut: &history::PublishedCut,
        budget: &super::audit_owner::native::Budget,
    ) -> Result<i64, String> {
        use super::audit_ledger::{Flavor, Mode, Unit};
        let g = cut.metadata.generation;
        if g != generation + 1 {
            return Err(err("历史资格代号不连续"));
        }
        if g > q.through {
            return Err(err("已发布代尚无已提交的完整资格"));
        }
        if cut.metadata.rule_revision.as_str() != q.rule_revision {
            return Err(err("已发布 cut 的规则与完整资格不符"));
        }
        let roots = [
            (cut.metadata.scope.view(), Mode::Public),
            (cut.roots.generated_state.view(), Mode::Public),
            (cut.roots.publication_template.view(), Mode::Template),
            (cut.roots.delta_template.view(), Mode::Template),
        ];
        for (root, mode) in roots {
            let unit = Unit {
                digest: root.digest().map_err(err)?,
                flavor: Flavor::Value,
                mode,
            };
            if !runtime.contains_qualified_unit(q, &unit)? {
                return Err(err(format!("第 {g} 代根单位不在已认证单位集")));
            }
        }
        if g == q.through {
            // 比较缓冲临时持票：输出串 ≤ ROOTS_BOUND，逐字段转义的短暂副本再计一份。
            let bound = super::audit_ledger::Qualification::ROOTS_BOUND;
            let _temporary = budget
                .reserve(bound.saturating_mul(2))
                .map_err(|_| err("四根比较缓冲父预算不足"))?;
            let mut actual = String::with_capacity(bound);
            super::audit_ledger::roots_json_into(&mut actual, roots.map(|(root, _)| root))?;
            if actual.capacity() > bound {
                return Err(err("四根比较缓冲超过预留上界"));
            }
            if actual != q.roots {
                return Err(err("末代四根与完整资格四根不符"));
            }
        }
        #[cfg(test)]
        trace_event("h1_qualified_cut", Some(g));
        Ok(g)
    }

    #[cfg(test)]
    pub(super) fn frontier(&self) -> &CertifiedFrontier {
        &self.frontier
    }
}

#[cfg(feature = "s_incremental")]
fn exact_copy(text: &str) -> String {
    let mut owned = String::with_capacity(text.len());
    owned.push_str(text);
    owned
}
/// 实际容量不得超过许可；超过是分配器口径与预留不符，按失效处理而不是扩票。
#[cfg(feature = "s_incremental")]
fn finish_reserved<T: super::audit_retention::Retained>(
    permit: super::audit_owner::native::Permit,
    value: T,
) -> Result<super::audit_owner::native::Reserved<T>, String> {
    let actual = super::audit_retention::Retained::heap_bytes(&value);
    if actual > permit.bytes() {
        return Err(err("持久载荷超过分配前许可"));
    }
    Ok(permit.finish(actual).hold(value))
}

#[cfg(feature = "s_incremental")]
impl AuditedState {
    /// 内容与台账均已提交后由 owner 调用；非资格组件为空操作，资格组件失败即 stale。
    /// `permit` 是费用终检前为本次交接预留的许可（`qualified_transition_bound`）。
    pub(super) fn advance_qualified(
        &mut self,
        runtime: &super::prepared::Runtime,
        permit: Option<super::audit_owner::native::Permit>,
        budget: &super::audit_owner::native::Budget,
    ) -> Result<super::audit_retention::RetainedDelta, String> {
        let Some(qualified) = self
            .certification
            .as_mut()
            .and_then(|c| c.qualified.as_mut())
        else {
            return Ok(Default::default());
        };
        let permit = permit.ok_or_else(|| err("资格推进缺费用终检前的许可"))?;
        let cut = match &self.historical {
            HistoricalOwner::Unique(v) => v.cuts.last(),
            HistoricalOwner::Shared(v) => v.cuts.last(),
        }
        .ok_or_else(|| err("资格推进缺刚发布的一代"))?;
        qualified.advance(runtime, cut, permit, budget)
    }

    /// 资格组件下一次交接的持久上界；候选在费用终检前据此持票并入账。
    pub(super) fn qualified_transition_bound(&self) -> Option<usize> {
        self.certification
            .as_ref()
            .and_then(|c| c.qualified.as_ref())
            .map(|q| QualifiedCertification::transition_bound(&q.identity))
    }

    /// stage 改写台账前须核的上一代资格；Empty 前沿由 stage 的代号连续检查覆盖。
    pub(super) fn prior_qualification(&self) -> Option<&super::audit_ledger::Qualification> {
        self.certification
            .as_ref()
            .and_then(|c| c.qualified.as_ref())
            .and_then(|q| q.frontier.qualification())
    }

    /// 资格组件（身份 + 前沿）当前入账的保留字节。
    #[cfg(test)]
    pub(super) fn certification_retained(&self) -> usize {
        super::audit_retention::Retained::heap_bytes(&self.certification)
    }

    #[cfg(test)]
    pub(super) fn qualified_frontier(&self) -> Option<&CertifiedFrontier> {
        self.certification
            .as_ref()
            .and_then(|c| c.qualified.as_ref())
            .map(QualifiedCertification::frontier)
    }
}

#[cfg(feature = "s_incremental")]
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "#1481：历史认证组件，正式入口待 #1645/#1491")
)]
impl HistoricalCertificate {
    pub(super) fn cold(
        tx: &Transaction<'_>,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn audit_budget::Budget>,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
    ) -> Result<Self, String> {
        Self::cold_captured(tx, deadline, budget, work, None).map(|(cert, _, _)| cert)
    }

    pub(super) fn cold_for_request(
        tx: &Transaction<'_>,
        deadline: Option<std::time::Instant>,
        work: &sc::capture_allocation::scratch::WorkScope,
        context: &mut super::request_audit::RequestAuditContext,
        qualified: Option<QualifiedSeed<'_>>,
    ) -> Result<AuditedState, String> {
        drop(context.take());
        let (cert, capture, node_count) =
            Self::cold_captured(tx, deadline, None, Some(work), qualified)?;
        let AuditedState {
            raw,
            budget,
            historical,
            certification,
        } = cert.audited;
        let HistoricalOwner::Unique(result) = historical else {
            unreachable!()
        };
        let result =
            super::native_shared::NativeRc::try_new_shallow(result, Some(work)).map_err(err)?;
        let binding = super::request_audit::HistoricalDependencyBinding::read_admitted(
            tx,
            &result,
            Some(work),
        )?;
        history::check_deadline(deadline)?;
        context.retain(binding, capture, node_count, result.clone(), false, None);
        Ok(AuditedState {
            raw,
            budget,
            historical: HistoricalOwner::Shared(result),
            certification,
        })
    }

    fn cold_captured(
        tx: &Transaction<'_>,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn audit_budget::Budget>,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
        qualified: Option<QualifiedSeed<'_>>,
    ) -> Result<(Self, sc::AuditCapture, usize), String> {
        history::check_deadline(deadline)?;
        if let Some(budget) = budget {
            budget.precheck().map_err(err)?;
        }
        let measure = audit_budget::NativeMeasure { budget, deadline };
        let (capture, node_count) = content_store::capture_for_request_prototype_admitted_measured(
            tx,
            work,
            Some(&measure),
        )
        .map_err(err)?;
        let capture = capture
            .enable_native_frame_reuse_admitted(work)
            .map_err(err)?;
        let control = control_audit::prepare_s3_control_history_admitted(
            tx,
            None,
            work,
            budget.and_then(|b| b.before_write()),
        )?;
        let raw = super::business_json::RawEvents::from_json_owner(
            crate::read_raw_events_until_admitted_measured(tx, deadline, work, budget)?,
        )?;
        audit_profile_id_admitted(tx, None, work)?;
        let mut sources = HistoricalSourceFold {
            raw: raw.view(),
            source: Some(source_audit::NativeSourceAudit::new_with_records_measured(
                &capture,
                raw.view(),
                None,
                work,
                budget,
                deadline,
                history::SourcePolicy::NativeMeasured,
            )?),
            generation: 0,
            previous: -1,
            deadline,
            budget,
            owner_source: None,
            certified_online: false,
        };
        let indexed = history::fold_certified_history(
            tx,
            raw.view(),
            control,
            &capture,
            work,
            deadline,
            budget,
            &mut sources,
        )?;
        verify_unrun_catalog_admitted(tx, indexed.cuts.is_empty(), None, work)?;
        let mut store = history::HashReader::captured_for_audit(&capture, None)
            .map_err(err)?
            .with_audit_gate(budget, deadline)
            .with_source_policy(history::SourcePolicy::NativeMeasured)
            .with_native_scope(work)
            .map_err(err)?;
        if let Some(last) = indexed.cuts.last() {
            let axes = field(&mut store, &last.roots.generated_state, "catalog_axes")?;
            catalog_evidence::verify_current_for_audit(tx, &mut store, &axes)?;
        }
        history::check_deadline(deadline)?;
        let raw_sources = sources
            .source
            .take()
            .map(source_audit::NativeSourceAudit::into_raw_facts);
        drop(sources);
        let history::IndexedHistory {
            control_facts,
            raw_prefixes,
            auxiliary_facts,
            cuts,
            objects,
            auxiliary,
            object_sweep,
        } = indexed;
        drop(store);
        // 冷起步：原件逐代核过后，才从空前沿用同一 Δ 步消费台账已提交的完整资格。
        let qualified = qualified
            .map(|seed| -> Result<_, String> {
                // 规则串只借用本行文本，复制前已按其长度在父账持票。
                let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
                    tx,
                    c"SELECT value FROM meta WHERE key='rule_revision'",
                )
                .map_err(err)?;
                let folded = {
                    let mut row = stmt
                        .next()
                        .map_err(err)?
                        .ok_or_else(|| err("库缺 rule_revision"))?;
                    QualifiedCertification::fold(seed, row.text(0).map_err(err)?, &cuts)
                };
                stmt.finalize().map_err(err)?;
                folded
            })
            .transpose()?;
        Ok((
            Self {
                audited: AuditedState {
                    budget: raw.budget().cloned(),
                    raw,
                    certification: Some(HistoricalCertification { qualified }),
                    historical: HistoricalOwner::Unique(ImmutableHistoricalAudit {
                        control_facts,
                        raw_prefixes,
                        raw_sources,
                        auxiliary_facts,
                        object_facts: object_sweep,
                        cuts,
                        objects,
                        auxiliary,
                    }),
                },
            },
            capture,
            node_count,
        ))
    }

    pub(super) fn historical(&self) -> &ImmutableHistoricalAudit {
        &self.audited
    }
}

// #1394：仅ignored成本诊断开启的当前线程阶段记录，不进入生产代码或资源判据。
#[cfg(test)]
#[derive(Default, Clone, Copy, serde::Serialize)]
struct WorkCounts {
    sql_returned_rows: [u64; 4],
    sql_returned_bytes: [u64; 4],
    local_sha256_calls: u64,
    local_sha256_bytes: u64,
    raw_payload_hashes: u64,
    raw_source_hashes: u64,
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
    static RETENTION_REPORT_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(crate) fn trace_event(phase: &'static str, generation: Option<i64>) {
    super::admission_private::s4_read_trace::audit_event(phase, generation);
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace
                .events
                .push(json!({"phase":phase,"generation":generation,
                "elapsed_ns":trace.start.elapsed().as_nanos().to_string(),"work":trace.work}));
        }
    });
}
/// #1640 D2 出生锁：完整分支（`generate_impl(None)`）一次调用及其入口标签。
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn trace_birth(entry: &'static str, generation: i64) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace
                .events
                .push(json!({"phase":"generate_full_birth","entry":entry,"generation":generation}));
        }
    });
}
/// #1488 S4（主控 D4/D6）：整代六阶段起点标记；`path` 记 delta/full_lease/full 车道。
/// 只追加累计快照，阶段读量由 [`stage_rows`] 对相邻标记做差。
#[cfg(test)]
pub(super) fn trace_stage(stage: &'static str, path: &'static str, generation: i64) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace
                .events
                .push(json!({"phase":"s4_stage","stage":stage,"path":path,
                "generation":generation,"elapsed_ns":trace.start.elapsed().as_nanos() as u64,
                "work":trace.work,"store_node_reads":content_store::sql_reads_on_thread(),
                "frame_calls":sc::frame_calls_on_thread()}));
        }
    });
}
/// S4 六阶段名；`stage_rows` 只汇总这些段，其余标记（publish_end）只作终点。
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
pub(super) const S4_STAGES: [&str; 6] = [
    "generate",
    "object_plan",
    "prepare_source_audit",
    "assemble_manifest",
    "stage_indexes",
    "publish_borrowed",
];
/// 把一次 traced 窗口的 s4_stage 标记折成每阶段 {sql_node_rows,…,elapsed_ns}；同名多段累加。
/// sql_*[3] 为 sqlite trace 实测节点行（需 sql_read_probe），store_node_reads 为
/// content_store 按 digest 取节点的语句次数。
#[cfg(test)]
#[cfg_attr(
    all(test, not(feature = "s_incremental")),
    allow(dead_code, reason = "#1488 再合 #1608：s_session 测试 构建不触达")
)]
pub(super) fn stage_rows(events: &[Value]) -> Value {
    let marks: Vec<&Value> = events.iter().filter(|e| e["phase"] == "s4_stage").collect();
    let num = |v: &Value| v.as_u64().unwrap_or(0);
    let mut out = serde_json::Map::new();
    let mut paths = std::collections::BTreeSet::new();
    for name in S4_STAGES {
        out.insert(
            name.into(),
            json!({"sql_node_rows":0,"sql_node_bytes":0,"sql_rows_total":0,
                "store_node_reads":0,"frame_calls":0,"elapsed_ns":0,"segments":0}),
        );
    }
    for pair in marks.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let name = a["stage"].as_str().unwrap_or("");
        paths.insert(a["path"].as_str().unwrap_or("").to_string());
        let Some(row) = out.get_mut(name) else {
            continue;
        };
        let diff = |key: &str| num(&b[key]).saturating_sub(num(&a[key]));
        let work =
            |key: &str, i: usize| num(&b["work"][key][i]).saturating_sub(num(&a["work"][key][i]));
        let total: u64 = (0..4).map(|i| work("sql_returned_rows", i)).sum();
        for (key, add) in [
            ("sql_node_rows", work("sql_returned_rows", 3)),
            ("sql_node_bytes", work("sql_returned_bytes", 3)),
            ("sql_rows_total", total),
            ("store_node_reads", diff("store_node_reads")),
            ("frame_calls", diff("frame_calls")),
            ("elapsed_ns", diff("elapsed_ns")),
            ("segments", 1),
        ] {
            row[key] = json!(num(&row[key]) + add);
        }
    }
    json!({"stages":out,"paths":paths,"marks":marks.len()})
}
#[cfg(test)]
pub(super) fn trace_request_clock(bytes: usize, elapsed: std::time::Duration) {
    AUDIT_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace
                .events
                .push(json!({"phase":"request_clock_full_check","bytes":bytes,
                "clock_hashes":1,"duration_ns":elapsed.as_nanos().to_string()}));
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

/// 与共享控制层相同的 SQL 文本所有者；返回值在本次完整审计活期内持账。
#[cfg_attr(
    not(all(test, feature = "s_allocation_json_testonly")),
    allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
)]
fn audit_profile_id(
    tx: &Transaction<'_>,
    allocation: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<super::business_json::TextValue, String> {
    audit_profile_id_admitted(tx, allocation, None)
}
fn audit_profile_id_admitted(
    tx: &Transaction<'_>,
    allocation: Option<&sc::capture_allocation::OwnedBudget>,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
) -> Result<super::business_json::TextValue, String> {
    let profile_id = v2::control_meta_text_admitted(tx, "profile_id", allocation, work)?
        .ok_or_else(|| err("缺冻结字段 profile_id"))?;
    if profile_id != tb02a::PROFILE {
        return Err(err(format!(
            "非OHLC档（profile=`{profile_id}`）：完整状态审计的来源/目录证据只接线 `{}`，其他档仍待接线，拒绝按此口径放行",
            tb02a::PROFILE
        )));
    }
    Ok(profile_id)
}

/// 静态列/冻结原件由共享控制审计核验；本层补核从未运行的动态列。
#[cfg_attr(
    not(all(test, feature = "s_allocation_json_testonly")),
    allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
)]
fn verify_unrun_catalog(
    tx: &Transaction<'_>,
    initial: bool,
    allocation: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<(), String> {
    verify_unrun_catalog_admitted(tx, initial, allocation, None)
}
fn verify_unrun_catalog_admitted(
    tx: &Transaction<'_>,
    initial: bool,
    allocation: Option<&sc::capture_allocation::OwnedBudget>,
    work: Option<&sc::capture_allocation::scratch::WorkScope>,
) -> Result<(), String> {
    use super::business_json::TextValue;
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        tx,
        c"SELECT catalog_id,impl_status,proof_status,run_status,evidence_json FROM catalog",
    )
    .map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        // 与旧 r.get::<_,String> 同列、同序、同类型核验；复制落到调用方真实预算。
        // checked FFI的通用type错误省略列名，此固定SQL仍保留原r.get诊断字节。
        let text = |row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>, column: usize| {
            const NAMES: [&str; 5] = [
                "catalog_id",
                "impl_status",
                "proof_status",
                "run_status",
                "evidence_json",
            ];
            let actual = row.column_type(column).map_err(err)?;
            if actual != rusqlite::types::Type::Text {
                return Err(err(rusqlite::Error::InvalidColumnType(
                    column,
                    NAMES[column].into(),
                    actual,
                )));
            }
            TextValue::from_sql_admitted(row, column, allocation, work).map_err(err)
        };
        let id = text(&mut row, 0)?;
        let implemented = text(&mut row, 1)?;
        let proof = text(&mut row, 2)?;
        let run = text(&mut row, 3)?;
        let evidence = text(&mut row, 4)?;
        validate_unrun_row(&id, &implemented, &proof, &run, &evidence, initial)?;
    }
    stmt.finalize().map_err(err)
}

pub(super) fn validate_unrun_row(
    id: &str,
    implemented: &str,
    proof: &str,
    run: &str,
    evidence: &str,
    initial: bool,
) -> Result<(), String> {
    if (initial || !catalog_evidence::AXIS_IDS.contains(&id))
        && (implemented != "not_implemented"
            || proof != "not_proved"
            || run != "not_run"
            || evidence != "{}")
    {
        return Err(err(format!("未运行目录{id}带无来源运行状态")));
    }
    Ok(())
}

fn err(e: impl std::fmt::Display) -> String {
    format!("S3StateInvalid：{e}")
}

fn field<'r>(
    s: &mut (impl super::audit_store::CodecStore + ?Sized),
    r: impl Into<sc::RefView<'r>>,
    key: &str,
) -> Result<super::audit_store::RefValue, String> {
    s.lookup_field_ref(r.into(), key).map_err(err)
}

// #1394：入口路由锁。默认/通用 path 会失败，绿判据必须来自真实 borrowed 方法。
#[cfg(test)]
pub(super) mod borrowed_field_probe {
    use super::super::audit_store::{CodecRead, CodecStore, RefValue};
    use super::*;
    use sc::{CodecError, NodeDigest, NodeRead, NodeSink, NodeSource};

    pub(crate) struct Probe {
        pub(crate) reference: ContentRef,
        pub(crate) fields: usize,
    }
    impl Probe {
        pub(crate) fn new() -> Self {
            Self {
                reference: sc::write_value(&mut sc::MemoryNodes::new(), &sc::Value::Null)
                    .unwrap()
                    .content_ref(),
                fields: 0,
            }
        }
    }
    impl NodeSource for Probe {
        fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, CodecError> {
            panic!("borrowed字段入口不得退回generic read")
        }
    }
    impl NodeSink for Probe {
        fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), CodecError> {
            panic!("字段导航不得写入")
        }
    }
    impl CodecRead for Probe {
        fn lookup_ref(
            &mut self,
            _: sc::RefView<'_>,
            _: &[sc::path::PathStep],
        ) -> Result<RefValue, CodecError> {
            panic!("borrowed字段入口不得先造owned PathStep")
        }
        fn lookup_field_ref(
            &mut self,
            _: sc::RefView<'_>,
            key: &str,
        ) -> Result<RefValue, CodecError> {
            assert_eq!(key, "borrowed😀");
            self.fields += 1;
            Ok(RefValue::from(self.reference.clone()))
        }
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            panic!("borrowed字段入口不得擦除codec")
        }
    }
    impl CodecStore for Probe {
        fn generic_store(&mut self) -> &mut dyn sc::NodeStore {
            self
        }
    }
    #[test]
    fn state_field_uses_borrowed_entry_without_generic_retry() {
        let mut probe = Probe::new();
        let root = probe.reference.clone();
        assert_eq!(
            field(&mut probe, &root, "borrowed😀")
                .unwrap()
                .digest()
                .unwrap(),
            root.digest().unwrap()
        );
        assert_eq!(probe.fields, 1);
    }
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
                "batch_id":cut.batch_id,"snapshot":cut.snapshot,
                "delta":cut.delta})).collect::<Vec<_>>(),
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
                sc::verify_refview(&mut persisted, root.view(), sc::Mode::Public).unwrap();
            }
            for root in [&cut.roots.publication_template, &cut.roots.delta_template] {
                sc::verify_refview(&mut persisted, root.view(), sc::Mode::Template).unwrap();
            }
        }
        for object in &audited.objects {
            for root in [
                super::audit_store::RefValue::from(object.generated.content_ref()),
                super::audit_store::RefValue::from(object.internal.content_ref()),
                super::audit_store::RefValue::from(object.immutable.content_ref()),
            ] {
                sc::verify_refview(&mut persisted, root.view(), sc::Mode::Public).unwrap();
            }
            sc::verify_refview(&mut persisted, object.birth_wire.view(), sc::Mode::Public).unwrap();
        }
        for witness in &audited.auxiliary.witnesses {
            sc::verify_refview(&mut persisted, witness.generated.view(), sc::Mode::Public).unwrap();
            sc::verify_refview(&mut persisted, witness.wire.view(), sc::Mode::Public).unwrap();
        }
        for relation in &audited.auxiliary.relations {
            sc::verify_refview(&mut persisted, relation.generated.view(), sc::Mode::Public)
                .unwrap();
        }
        for observation in &audited.auxiliary.observations {
            sc::verify_refview(
                &mut persisted,
                observation.generated.view(),
                sc::Mode::Public,
            )
            .unwrap();
            sc::verify_refview(&mut persisted, observation.wire.view(), sc::Mode::Public).unwrap();
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
        source_audit::reset_native_witness_identity_stats();
        source_audit::reset_native_source_prefix_stats();
        #[cfg(feature = "s_allocation_json_testonly")]
        sc::capture_allocation::overlay::reset_first_overlay_refusal_for_testonly();
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
        let peaks = std::rc::Rc::new(std::cell::Cell::new((0usize, 0usize)));
        let overlay_config = std::env::var("AUDIT_PROTOTYPE_OVERLAY_SLOTS")
            .ok()
            .map(|slots| history::OverlayPrototype {
                slots: slots.parse().unwrap(),
                bytes: std::env::var("AUDIT_PROTOTYPE_OVERLAY_BYTES")
                    .unwrap()
                    .parse()
                    .unwrap(),
                budget: owned_budget.as_ref().unwrap().clone(),
                peaks: peaks.clone(),
            });
        let run = || {
            traced(true, || {
                audit_with_capture_impl(&tx, None, None, None, enabled, owned_budget.as_ref())
            })
        };
        let (result, events, work) = match overlay_config {
            Some(c) => history::with_overlay_prototype(c, run),
            None => run(),
        };
        let elapsed = started.elapsed().as_nanos();
        #[cfg(feature = "s_allocation_json_testonly")]
        let first_overlay_refusal =
            sc::capture_allocation::overlay::take_first_overlay_refusal_for_testonly();
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        let first_overlay_refusal: Option<()> = None;
        if let Err(error) = &result {
            let failure = json!({"native_witness_identity":source_audit::native_witness_identity_stats(),"native_source_prefix_hits_misses":source_audit::native_source_prefix_stats(),"whole_audit_allocation_admitted":false,"testonly_failure_receipt":true,"error":error,"first_overlay_refusal":first_overlay_refusal,"after_drop_meter":owned_budget.as_ref().map(|b|b.snapshot()),"last_layout":owned_budget.as_ref().map(|b|b.allocation_request_probe()),"f_calls":sc::frame_calls_on_thread()-f,"sql_node_reads":content_store::sql_reads_on_thread()-sql,"events":events,"work":work,"elapsed_ns":elapsed.to_string()});
            std::fs::write(
                std::env::var("AUDIT_PROTOTYPE_REPORT").unwrap(),
                canonical_json(&failure),
            )
            .unwrap();
        }
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
        let report = json!({"native_witness_identity":source_audit::native_witness_identity_stats(),"native_source_prefix_hits_misses":source_audit::native_source_prefix_stats(),"overlay_peak_used_slots":peaks.get().0,"overlay_peak_used_body_bytes":peaks.get().1,"whole_audit_allocation_admitted":false,"first_overlay_refusal":first_overlay_refusal,"capture_meter_after_drop":owned_budget.as_ref().map(|b|b.snapshot()),"elapsed_ns":elapsed.to_string(),"capture_ns":capture.as_ref().and_then(|v|v.get("capture_ns")).cloned(),"capture":capture,"sql_node_reads":content_store::sql_reads_on_thread()-sql,"f_calls":sc::frame_calls_on_thread()-f,"work":work,"events":events,"complete":complete,"complete_sha256":sha256_hex(canonical_json(&complete).as_bytes())});
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

// #1394: one exact fresh TestOnly process. The production audit and its SQL predicates
// are unchanged; the optional fault starts only after the real capture exists.
#[cfg(all(test, feature = "s_allocation_sqlite_testonly"))]
mod shared_sqlite_probe {
    use super::*;
    use sc::capture_allocation::{sqlite, OwnedBudget};
    thread_local! {
        static FAULT: std::cell::RefCell<Option<(sqlite::RefusalControl, OwnedBudget)>> = const { std::cell::RefCell::new(None) };
        static ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static ARM_LIVE: std::cell::Cell<Option<(usize, usize)>> = const { std::cell::Cell::new(None) };
    }
    pub(super) fn arm_after_owned_capture() {
        FAULT.with(|f| {
            if let Some((control, budget)) = f.borrow().as_ref() {
                let total = budget.snapshot().live_requested_layout_bytes;
                let sql = budget.ledger_health().sqlite_live_layout_bytes;
                ARM_LIVE.with(|a| a.set(Some((total, sql))));
                control.refuse_sqlite_allocations_for_testonly();
                ARMED.with(|a| a.set(true));
            }
        });
    }
    #[test]
    #[ignore]
    fn actual_current_material_shared_sqlite() {
        let db = std::env::var("AUDIT_PROTOTYPE_DB").unwrap();
        let limit: usize = std::env::var("AUDIT_PROTOTYPE_OWNED_PAYLOAD_BYTES")
            .unwrap()
            .parse()
            .unwrap();
        let budget = OwnedBudget::new(limit);
        // SAFETY: this exact test runs alone in a fresh process, before any SQLite open.
        let guard = unsafe { sqlite::TestOnlySqliteAllocator::install(&budget) }.unwrap();
        let control = guard.control();
        let inject = std::env::var_os("AUDIT_SQLITE_INJECT_REFUSAL").is_some();
        if inject {
            FAULT.with(|f| *f.borrow_mut() = Some((control.clone(), budget.clone())));
        }
        let mut conn =
            Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let tx = conn.transaction().unwrap();
        let peaks = std::rc::Rc::new(std::cell::Cell::new((0usize, 0usize)));
        let config = history::OverlayPrototype {
            slots: std::env::var("AUDIT_PROTOTYPE_OVERLAY_SLOTS")
                .unwrap()
                .parse()
                .unwrap(),
            bytes: std::env::var("AUDIT_PROTOTYPE_OVERLAY_BYTES")
                .unwrap()
                .parse()
                .unwrap(),
            budget: budget.clone(),
            peaks: peaks.clone(),
        };
        let sql = content_store::sql_reads_on_thread();
        let frames = sc::frame_calls_on_thread();
        let started = std::time::Instant::now();
        let (result, events, work) = history::with_overlay_prototype(config, || {
            traced(true, || {
                audit_with_capture_impl(&tx, None, None, None, true, Some(&budget))
            })
        });
        let elapsed = started.elapsed().as_nanos();
        let before_close = budget.snapshot();
        let health_before_close = budget.ledger_health();
        let request = budget.allocation_request_probe();
        let armed = ARMED.with(|a| a.get());
        FAULT.with(|f| {
            f.borrow_mut().take();
        });
        control.clear_refusal_for_testonly();
        let error = result.as_ref().err().cloned();
        let (complete, retention) = match result {
            Ok((state, retention)) => {
                let cuts = state
                    .cuts
                    .iter()
                    .map(|c| {
                        format!(
                            "{:?}|{:?}|{:?}|{:?}|{:?}",
                            c.metadata, c.roots, c.batch_id, c.snapshot, c.delta
                        )
                    })
                    .collect::<Vec<_>>();
                let complete = json!({"raw":state.raw,"cuts":cuts,"objects":format!("{:?}",state.objects),"auxiliary":format!("{:?}",state.auxiliary),"clock":v2::stored_clock(&tx).unwrap().value});
                (Some(complete), retention)
            }
            Err(_) => (None, None),
        };
        let frame_count = sc::frame_calls_on_thread() - frames;
        let node_sql_reads = content_store::sql_reads_on_thread() - sql;
        drop(tx);
        drop(conn);
        // SAFETY: all user statements, transaction, and connection are now dropped.
        unsafe { guard.finish() }.unwrap();
        let final_meter = budget.snapshot();
        let final_health = budget.ledger_health();
        let report = json!({
            "scope":"selected closed codec, SQLite requested Layout/header, actual raw JSON consumers and typed audit return owners; remaining temporary collections and outer diagnostics are not qualified",
            "uncovered":["temporary control/metadata/scalar materialization and collections","ordinary Q projection/fragment worksets","outer errors and diagnostic reports","allocator metadata/rounding and physical RSS/IO"],
            "whole_audit_allocation_admitted":false,"production_capture_enabled":false,
            "fault_injection":inject,"armed_after_real_owned_capture":armed,
            "live_at_fault_arm":ARM_LIVE.with(|a| a.get()),
            "error":error,"before_sql_close":before_close,"health_before_sql_close":health_before_close,
            "last_layout":request,"after_all_drop":final_meter,"health_after_all_drop":final_health,
            "elapsed_ns":elapsed.to_string(),"f_calls":frame_count,"sql_node_reads":node_sql_reads,
            "events":events,"work":work,"capture":retention,
            "overlay_peak_used_slots":peaks.get().0,"overlay_peak_used_body_bytes":peaks.get().1,
            "complete_sha256":complete.as_ref().map(|v|sha256_hex(canonical_json(v).as_bytes())),"complete":complete,
            "diagnostics_are_outside_payload_instrumentation":true
        });
        std::fs::write(
            std::env::var("AUDIT_PROTOTYPE_REPORT").unwrap(),
            canonical_json(&report),
        )
        .unwrap();
        assert!(!final_health.invariant_broken);
        assert_eq!(final_meter.live_requested_layout_bytes, 0);
        assert_eq!(final_meter.allocated_layouts, final_meter.released_layouts);
        if inject {
            assert!(armed, "fault must start after the actual owned capture");
            let (total, sql) = ARM_LIVE.with(|a| a.get()).unwrap();
            assert!(total > sql, "real Rust capture must still be charged");
            assert!(health_before_close.sqlite_rejected_layouts > 0);
            let error = error.expect("actual SQL allocation refusal must propagate audit Err");
            assert!(
                error.contains("memory") || error.contains("Memory"),
                "{error}"
            );
        } else {
            assert!(report["complete"].is_object(), "{error:?}");
        }
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
        prepares: std::cell::Cell<usize>,
        accepts: std::cell::Cell<usize>,
        stop: Option<usize>,
    }
    impl Meter {
        fn new(stop: Option<usize>) -> Self {
            Self {
                trace: Default::default(),
                checks: Default::default(),
                records: Default::default(),
                prepares: Default::default(),
                accepts: Default::default(),
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
            self.prepares.set(self.prepares.get() + 1);
            Ok(())
        }
        fn accept_prepared_frame(&self, f: &sc::job::CheckedFrame) -> Result<(), sc::CodecError> {
            self.event(b"accept");
            self.accepts.set(self.accepts.get() + 1);
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
    /// 正常未发布修订尾不得进入旧 cut；移交结果不得带 SQL/raw/capture 借用。
    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_owned_facts_keep_unpublished_raw_separate_after_transaction_end() {
        fn require_owned<T: 'static>(_: &T) {}
        let mut fixture = prepared::tests::Fixture::new();
        for (n, row) in [(1, ("a", 0, 6, 12, 3, 7)), (2, ("b", 10, 8, 16, 6, 12))] {
            let (event, token) = fixture.accept_begin(n, row, 1);
            fixture.publish(&event, &token).unwrap();
        }
        let (pending, token) = fixture.accept_begin(3, ("a", 0, 7, 13, 4, 8), 2);
        let tx = fixture.conn.transaction().unwrap();
        let ordinary = audit(&tx).unwrap();
        let native = audit_native_current(&tx).unwrap();
        assert_eq!(fingerprint(&ordinary), fingerprint(&native));
        tx.rollback().unwrap();
        require_owned(&native.control_facts);
        require_owned(&native.raw_prefixes);
        require_owned(&native.raw_sources);
        require_owned(&native.auxiliary_facts);
        require_owned(&native.object_facts);
        assert!(native.object_facts.is_some());
        assert!(native.auxiliary_facts.is_some());
        assert_eq!(native.control_facts.len(), 2);
        let prefix = native.raw_prefixes.as_ref().unwrap();
        assert_eq!(prefix.checked_len(), 3);
        assert_eq!(prefix.published_len(), 2);
        let source = native.raw_sources.as_ref().unwrap();
        assert_eq!(source.prefix_len(), 2);
        assert_eq!(source.hashed_rows(), 2);
        for cut in &native.cuts {
            native
                .control_facts
                .verify_header(
                    cut.metadata.generation,
                    &cut.batch_id,
                    Some(&cut.metadata.session_generation),
                    Some(&cut.metadata.clock_plan_hash),
                    Some(&cut.metadata.semantic_commit_ns),
                )
                .unwrap();
        }
        // 新发布走正常生产 API；旧纯结果保持原 F，没有引用可被新提交改写。
        fixture.publish(&pending, &token).unwrap();
        let tx = fixture.conn.transaction().unwrap();
        let current = audit_native_current(&tx).unwrap();
        assert_eq!(current.raw_prefixes.as_ref().unwrap().published_len(), 3);
        assert_eq!(native.raw_prefixes.as_ref().unwrap().published_len(), 2);
        assert_eq!(current.control_facts.len(), 3);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_relation_indexes_normal_multicut_full_state_parity() {
        let cases = [
            [
                ("a", 0, 6, 12, 3, 7, 1),
                ("b", 10, 8, 16, 6, 12, 1),
                ("c", 20, 8, 13, 4, 7, 1),
                ("b", 10, 8, 17, 6, 12, 2),
            ],
            [
                ("a", 0, 6, 12, 3, 7, 1),
                ("b", 10, 8, 16, 6, 12, 1),
                ("c", 20, 8, 13, 4, 7, 1),
                ("d", 30, 8, 17, 6, 12, 1),
            ],
            [
                ("a", 0, 6, 12, 3, 7, 1),
                ("b", 10, 8, 16, 6, 12, 1),
                ("b", 10, 8, 17, 6, 12, 2),
                ("b", 10, 8, 18, 6, 12, 3),
            ],
        ];
        let mut saw_later_dead_target = false;
        for (case, rows) in cases.into_iter().enumerate() {
            let mut fixture = prepared::tests::Fixture::new();
            for (n, (id, coord, open, high, low, close, revision)) in rows.into_iter().enumerate() {
                let (event, token) = fixture.accept_begin(
                    n as i64 + 1,
                    (id, coord, open, high, low, close),
                    revision,
                );
                fixture.publish(&event, &token).unwrap();
            }
            let tx = fixture.conn.transaction().unwrap();
            catalog_evidence::take_raw_visits();
            aux_index::reset_aux_audit_stats();
            let ordinary = audit(&tx).unwrap();
            let old_catalog = catalog_evidence::take_raw_visits();
            let old_aux = aux_index::aux_audit_stats();
            object_index::take_load_lookup_counts();
            aux_index::reset_aux_audit_stats();
            let (native, _) = audit_with_capture_impl(&tx, None, None, None, true, None).unwrap();
            let new_catalog = catalog_evidence::take_raw_visits();
            let new_aux = aux_index::aux_audit_stats();
            let (ids, lookups) = object_index::take_load_lookup_counts();
            assert_eq!(fingerprint(&ordinary), fingerprint(&native), "case={case}");
            assert_eq!(native.cuts.len(), 4);
            assert_eq!(old_catalog, 10);
            assert_eq!(new_catalog, 4);
            assert_eq!(old_aux[5], 10);
            assert_eq!(new_aux[5], 4);
            assert_eq!(old_aux[3], new_aux[3], "完整候选比较不能减少");
            assert_eq!(ids, native.objects.len());
            eprintln!(
                "对象 by-id 实际 capacity={}",
                object_index::load_id_capacity()
            );
            let mut old_target_comparisons = 0;
            let mut targets = 0;
            for object in &native.objects {
                if let Some(id) = object.life.superseded_by.as_deref() {
                    let ordinal = native.objects.iter().position(|e| e.id == id).unwrap();
                    old_target_comparisons += ordinal + 1;
                    targets += 1;
                    if let Some(death) = native.objects[ordinal].life.withdrawn_generation {
                        assert!(death > object.life.withdrawn_generation.unwrap());
                        saw_later_dead_target = true;
                    }
                }
            }
            assert_eq!(lookups, targets);
            assert!(old_target_comparisons >= lookups);
            eprintln!("冷审计 case={case} cuts=4 identities={ids} target_scan={old_target_comparisons} target_lookup={lookups} aux_raw={}->{}, catalog_raw={}->{}",
                old_aux[5], new_aux[5], old_catalog, new_catalog);

            // 同一真实原件逐 cut 与既有排序 oracle 对拍，另测回退与跳跃。
            let raw = native.raw.view();
            let mut index = super::super::business_json::RawPrefixIndex::new(raw);
            let mut visits = 0;
            for n in 0..=raw.len() {
                visits += index.advance(n).unwrap().raw_visits;
                let expected =
                    super::super::business_json::effective(raw.prefix(n).unwrap(), None).unwrap();
                let actual: Vec<_> = index
                    .ordered()
                    .map(|(_, ordinal)| raw.get(ordinal).unwrap())
                    .collect();
                assert!(
                    expected.iter().eq(actual.iter().copied()),
                    "case={case} prefix={n}"
                );
            }
            assert_eq!(visits, raw.len());
            for n in [1, 4, 2, 4] {
                index.advance(n).unwrap();
                let expected =
                    super::super::business_json::effective(raw.prefix(n).unwrap(), None).unwrap();
                assert!(expected.iter().eq(index
                    .ordered()
                    .map(|(_, ordinal)| raw.get(ordinal).unwrap())));
            }
        }
        assert!(saw_later_dead_target, "必须覆盖已撤回替代目标的完整身份史");
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn cold_relation_indexes_replay_message_keeps_raw_and_cuts() {
        let mut fixture = prepared::tests::Fixture::new();
        let mut first = None;
        for (n, row) in [
            (1, ("a", 0, 6, 12, 3, 7)),
            (2, ("b", 10, 8, 16, 6, 12)),
            (3, ("c", 20, 8, 13, 4, 7)),
        ] {
            let (event, token) = fixture.accept_begin(n, row, 1);
            fixture.publish(&event, &token).unwrap();
            if n == 1 {
                first = Some(event);
            }
        }
        let before = {
            let tx = fixture.conn.transaction().unwrap();
            fingerprint(&audit(&tx).unwrap())
        };
        let mut replay = first.unwrap();
        replay["message_id"] = json!("message-4");
        replay["payload"]["clock_event_id"] = json!("input-4");
        replay["payload_hash"] = json!(sha256_hex(canonical_json(&replay["payload"]).as_bytes()));
        let clock = v2::stored_clock(&fixture.conn).unwrap();
        {
            let tx = fixture.conn.transaction().unwrap();
            let control = control::S3Control::attach(&tx, &clock).unwrap();
            let view = control.inspect_ingest(replay.clone()).unwrap();
            let parsed = tb02a::parse_input(&replay["payload"]["raw_input"]).unwrap();
            let accepted = raw_input::accept_events(&tx, &parsed, Some("1")).unwrap();
            control.accept(&view, &accepted).unwrap();
            tx.commit().unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let ordinary = audit(&tx).unwrap();
        let (native, _) = audit_with_capture_impl(&tx, None, None, None, true, None).unwrap();
        assert_eq!(native.raw.len(), 3);
        assert_eq!(native.cuts.len(), 3);
        assert_eq!(before, fingerprint(&native));
        assert_eq!(fingerprint(&ordinary), fingerprint(&native));
        eprintln!("正常 replay：4消息引用3 raw，3 cuts 全字段/根保持不变");
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_frame_reuse_opt_in_respects_render_and_budget_gates() {
        let mut fixture = prepared::tests::Fixture::new();
        let (event, token) = fixture.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        fixture.publish(&event, &token).unwrap();
        let tx = fixture.conn.transaction().unwrap();
        let (native, native_receipt) =
            audit_with_capture_impl(&tx, None, None, None, true, None).unwrap();
        let (render_bounded, bounded_receipt) =
            audit_with_capture_impl(&tx, None, Some(u64::MAX), None, true, None).unwrap();
        assert_eq!(fingerprint(&native), fingerprint(&render_bounded));
        assert_eq!(
            native_receipt.as_ref().unwrap()["native_frame_reuse"]["enabled"],
            true
        );
        assert_eq!(
            bounded_receipt.as_ref().unwrap()["native_frame_reuse"]["enabled"],
            false
        );
        assert_eq!(
            bounded_receipt.as_ref().unwrap()["native_frame_reuse"]["same_material_hits"],
            0
        );

        // 即使已opt-in且帧已热，BudgetSome仍逐次走原read→prepare→F→accept门序。
        let capture = content_store::capture_for_prototype(&tx)
            .unwrap()
            .enable_native_frame_reuse()
            .unwrap();
        let digest = native.cuts[0].snapshot.view().digest().unwrap();
        let original = sc::NodeSource::checked_read(&mut capture.reader(), &digest)
            .unwrap()
            .unwrap()
            .into_parts();
        let before = capture.retention();
        let frames = sc::frame_calls_on_thread();
        let meter = Meter::new(None);
        let mut gated = history::HashReader::captured_for_audit(&capture, None)
            .unwrap()
            .with_audit_gate(Some(&meter), None);
        for _ in 0..2 {
            assert_eq!(
                sc::NodeSource::checked_read(&mut gated, &digest)
                    .unwrap()
                    .unwrap()
                    .into_parts(),
                original
            );
        }
        assert_eq!(sc::frame_calls_on_thread() - frames, 2);
        assert_eq!(meter.prepares.get(), 2);
        assert_eq!(meter.accepts.get(), 2);
        assert_eq!(meter.checks.get(), 4);
        let expected = Meter::new(None);
        for _ in 0..2 {
            expected.event(b"pre");
            expected.event(b"prepare");
            expected.event(b"accept");
            expected.event(&digest);
            expected.event(b"pre");
        }
        assert_eq!(meter.fingerprint(), expected.fingerprint());
        let after = capture.retention();
        assert_eq!(
            after["capture_reads"].as_u64().unwrap(),
            before["capture_reads"].as_u64().unwrap() + 2
        );
        assert_eq!(after["native_frame_reuse"], before["native_frame_reuse"]);
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

/// 仅测试真实audit入口，资格仍由同一完整body判据给出；外层显式配置同账overlay。
#[cfg(all(test, feature = "s_allocation_json_testonly"))]
#[allow(
    dead_code,
    reason = "#1608：仅测试真实 audit 入口，当前无调用方；保留不删"
)]
pub(super) fn audit_owned_capture_for_testonly(
    tx: &Transaction<'_>,
    budget: &sc::capture_allocation::OwnedBudget,
) -> Result<AuditedState, String> {
    audit_core(
        tx,
        None,
        None,
        None,
        true,
        Some(budget),
        None,
        RetentionReport::Skip,
    )
    .map(|(state, _)| state)
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod unrun_catalog_owner_tests {
    use super::*;
    use newchan_rust::s_content::capture_allocation::OwnedBudget;
    use rusqlite::Connection;

    #[test]
    fn audit_profile_sql_text_layout_missing_types_and_read_errors_release() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE meta(key TEXT PRIMARY KEY,value);")
            .unwrap();
        conn.execute(
            "INSERT INTO meta VALUES('profile_id',?1)",
            params![tb02a::PROFILE],
        )
        .unwrap();
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let tx = conn.transaction().unwrap();
        let normal = audit_profile_id(&tx, None).unwrap();
        let owned = audit_profile_id(&tx, Some(&budget)).unwrap();
        assert_eq!(normal.as_str(), owned.as_str());
        assert!(matches!(
            &owned,
            super::super::business_json::TextValue::Owned(_)
        ));
        let request = budget.allocation_request_probe().last_request_bytes;
        assert!(request > 0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, request);
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let short = OwnedBudget::new(request - 1);
        let failure = audit_profile_id(&tx, Some(&short)).unwrap_err();
        assert!(failure.contains(sc::capture_allocation::LIMIT), "{failure}");
        assert_eq!(short.allocation_request_probe().refused_bytes, request);
        assert_eq!(short.snapshot().allocation_attempts, 0);
        assert_eq!(short.snapshot().live_requested_layout_bytes, 0);
        for value in ["NULL", "7", "x'ff'", "CAST(x'ff' AS TEXT)", "'另一个档😀'"] {
            tx.execute_batch(&format!("UPDATE meta SET value={value}"))
                .unwrap();
            assert_eq!(
                audit_profile_id(&tx, None).unwrap_err(),
                audit_profile_id(&tx, Some(&budget)).unwrap_err()
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        tx.execute("DELETE FROM meta", []).unwrap();
        assert_eq!(
            audit_profile_id(&tx, Some(&budget)).unwrap_err(),
            err("缺冻结字段 profile_id")
        );
        tx.execute("DROP TABLE meta", []).unwrap();
        let normal = audit_profile_id(&tx, None).unwrap_err();
        assert!(normal.starts_with("读 meta.profile_id 失败："));
        assert_eq!(audit_profile_id(&tx, Some(&budget)).unwrap_err(), normal);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            budget.snapshot().released_layouts
        );
    }

    #[test]
    fn owned_formal_audit_skips_retention_dto_and_named_diagnostic_keeps_it() {
        let mut fixture = super::super::prepared::tests::Fixture::new();
        for n in 1..=3 {
            let id = format!("raw-{n}");
            let (event, token) =
                fixture.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
            fixture.publish(&event, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let profile = super::super::q_allocation::Profile {
            budget: OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        };
        let expected = audit(&tx).unwrap();
        let before = RETENTION_REPORT_CALLS.with(|calls| calls.get());
        let (actual, source) = audit_bounded_with_allocation(&tx, None, None, &profile).unwrap();
        assert_eq!(actual.cuts.len(), 3);
        assert_eq!(
            serde_json::to_value(&expected.raw).unwrap(),
            serde_json::to_value(&actual.raw).unwrap()
        );
        for (a, b) in expected.cuts.iter().zip(actual.cuts.iter()) {
            assert_eq!(
                a.roots.generated_state.digest().unwrap(),
                b.roots.generated_state.digest().unwrap()
            );
            assert_eq!(a.delta.digest().unwrap(), b.delta.digest().unwrap());
        }
        assert_eq!(RETENTION_REPORT_CALLS.with(|calls| calls.get()), before);
        drop(actual);
        drop(source);
        assert_eq!(profile.budget.snapshot().live_requested_layout_bytes, 0);
        let (_, diagnostic) = history::with_overlay_prototype(
            history::OverlayPrototype {
                slots: profile.overlay_slots,
                bytes: profile.overlay_bytes,
                budget: profile.budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || audit_with_capture_impl(&tx, None, None, None, true, Some(&profile.budget)),
        )
        .unwrap();
        assert!(diagnostic.unwrap().get("capture_ns").is_some());
        assert_eq!(RETENTION_REPORT_CALLS.with(|calls| calls.get()), before + 1);
        assert_eq!(profile.budget.snapshot().live_requested_layout_bytes, 0);
    }

    fn oracle(tx: &Transaction<'_>, initial: bool) -> Result<(), String> {
        let mut stmt = tx
            .prepare(
                "SELECT catalog_id,impl_status,proof_status,run_status,evidence_json FROM catalog",
            )
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
    #[test]
    fn owned_unrun_catalog_preserves_types_first_error_layout_refusal_and_zero_release() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE catalog(catalog_id,impl_status,proof_status,run_status,evidence_json); INSERT INTO catalog VALUES('非轴😀','not_implemented','not_proved','not_run','{}');").unwrap();
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        {
            let tx = conn.transaction().unwrap();
            for initial in [false, true] {
                assert_eq!(
                    oracle(&tx, initial),
                    verify_unrun_catalog(&tx, initial, None)
                );
                assert_eq!(
                    oracle(&tx, initial),
                    verify_unrun_catalog(&tx, initial, Some(&budget))
                );
            }
        }
        assert!(budget.snapshot().allocated_layouts > 0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        for column in [
            "catalog_id",
            "impl_status",
            "proof_status",
            "run_status",
            "evidence_json",
        ] {
            for value in ["NULL", "7", "x'ff'", "CAST(x'ff' AS TEXT)"] {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(&format!("UPDATE catalog SET {column}={value}"))
                    .unwrap();
                let expected = oracle(&tx, true).unwrap_err();
                assert_eq!(verify_unrun_catalog(&tx, true, None).unwrap_err(), expected);
                assert_eq!(
                    verify_unrun_catalog(&tx, true, Some(&budget)).unwrap_err(),
                    expected
                );
                tx.rollback().unwrap();
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            }
        }
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch("UPDATE catalog SET impl_status='implemented', proof_status='bad'; INSERT INTO catalog VALUES('后错',7,NULL,NULL,NULL);").unwrap();
            assert_eq!(
                verify_unrun_catalog(&tx, true, Some(&budget)),
                oracle(&tx, true)
            );
            tx.rollback().unwrap();
        }
        let probe_budget = OwnedBudget::new(50 * 1024 * 1024);
        let probe = super::super::business_json::TextValue::from_str("非轴😀", Some(&probe_budget))
            .unwrap();
        let request = probe_budget.allocation_request_probe().last_request_bytes;
        drop(probe);
        let short = OwnedBudget::new(request - 1);
        let tx = conn.transaction().unwrap();
        let rejected = verify_unrun_catalog(&tx, true, Some(&short)).unwrap_err();
        assert!(rejected.contains(sc::capture_allocation::LIMIT));
        assert_eq!(short.allocation_request_probe().refused_bytes, request);
        assert_eq!(short.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, sum, Retained};

    retained_struct!(ImmutableHistoricalAudit {
        control_facts,
        raw_prefixes,
        raw_sources,
        auxiliary_facts,
        object_facts,
        cuts,
        objects,
        auxiliary
    });
    impl Retained for AuditedState {
        fn heap_bytes(&self) -> usize {
            #[cfg(test)]
            trace_event("owner_retained_full_scan", None);
            let Self {
                raw,
                budget,
                historical,
                certification,
            } = self;
            if budget.is_some() {
                return usize::MAX;
            }
            // 40B B2：资格组件的身份与前沿字符串是跨请求保留载荷，与历史同账。
            sum([
                raw.heap_bytes(),
                match historical {
                    HistoricalOwner::Unique(v) => v.heap_bytes(),
                    HistoricalOwner::Shared(v) => v.heap_bytes(),
                },
                certification.heap_bytes(),
            ])
        }
    }
}

#[cfg(all(test, feature = "s_incremental"))]
thread_local! {
    pub(super) static H1_DROP_SOURCE_RECEIPT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(feature = "s_incremental")]
impl AuditedState {
    pub(super) fn append_owner_cut(
        self,
        tx: &Transaction<'_>,
        capture: &sc::AuditCapture,
        g: i64,
        batch: &str,
        ns: &str,
        object_rows: &[i64],
        aux_rows: &[Vec<i64>; 3],
        bound: &super::audit_owner::BoundOriginals,
        work: &sc::capture_allocation::scratch::WorkScope,
        native: &super::audit_owner::native::Budget,
        source_check: Option<&super::audit_owner::OwnerSourceCheck>,
    ) -> Result<(Self, super::audit_retention::RetainedDelta), super::audit_owner::native::Failure>
    {
        #[cfg(test)]
        let source_check = if H1_DROP_SOURCE_RECEIPT.with(|flag| flag.replace(false)) {
            None
        } else {
            source_check
        };
        let certified = self.certification.is_some();
        if certified && source_check.is_none() {
            return Err(err("历史认证在线缺同请求来源收据").into());
        }
        let mut retained = super::audit_retention::RetainedDelta::default();
        let historical = match self.historical {
            HistoricalOwner::Unique(v) => v,
            HistoricalOwner::Shared(v) => {
                let shell_bytes = v.allocation_bytes();
                let value = super::native_shared::NativeRc::try_unwrap(v)
                    .map_err(|_| err("owner 历史仍有外部共享引用"))?;
                // facts 只移动；try_unwrap 真正释放的是 Rc 外壳和两个计数。
                retained.release_bytes(shell_bytes);
                value
            }
        };
        let ImmutableHistoricalAudit {
            control_facts,
            raw_prefixes,
            raw_sources,
            auxiliary_facts,
            object_facts,
            cuts,
            objects,
            auxiliary,
        } = historical;
        let previous = cuts.last().map_or(-1, |c| c.metadata.input_frontier);
        let mut sources = HistoricalSourceFold {
            raw: self.raw.view(),
            source: Some(
                source_audit::NativeSourceAudit::restore(
                    capture,
                    self.raw.view(),
                    raw_sources.ok_or_else(|| err("缺 raw 来源事实"))?,
                    bound,
                )?
                .with_codec_work(work, native),
            ),
            generation: g - 1,
            previous,
            deadline: None,
            budget: None,
            owner_source: source_check,
            certified_online: certified,
        };
        let (updated, history_delta) = history::append_owner_cut(
            tx,
            self.raw.view(),
            capture,
            history::IndexedHistory {
                control_facts,
                raw_prefixes,
                auxiliary_facts,
                cuts,
                objects,
                auxiliary,
                object_sweep: object_facts,
            },
            g,
            batch,
            ns,
            object_rows,
            aux_rows,
            bound,
            work,
            native,
            &mut sources,
        )?;
        retained.merge(history_delta);
        let mut store = history::HashReader::captured_for_audit(capture, None)
            .map_err(err)?
            .with_codec_work(work, native);
        let cut = updated.cuts.last().ok_or_else(|| err("缺新 C_h"))?;
        let axes = field(&mut store, &cut.roots.generated_state, "catalog_axes")?;
        catalog_evidence::verify_current_for_audit(tx, &mut store, &axes)?;
        if certified {
            source_check
                .ok_or_else(|| err("历史认证在线缺同请求来源收据"))?
                .counts_for_cut(
                    cut.metadata.view(),
                    cut.roots.generated_state.view(),
                    usize::try_from(cut.metadata.input_frontier + 1).map_err(err)?,
                    None,
                )?;
        }
        #[cfg(test)]
        trace_event("owner_suffix_complete", Some(g));
        let raw_sources = sources
            .source
            .take()
            .map(|source| source.into_raw_facts_retained(&mut retained));
        drop(sources);
        Ok((
            Self {
                raw: self.raw,
                budget: self.budget,
                certification: self.certification,
                historical: HistoricalOwner::Unique(ImmutableHistoricalAudit {
                    control_facts: updated.control_facts,
                    raw_prefixes: updated.raw_prefixes,
                    raw_sources,
                    auxiliary_facts: updated.auxiliary_facts,
                    object_facts: updated.object_sweep,
                    cuts: updated.cuts,
                    objects: updated.objects,
                    auxiliary: updated.auxiliary,
                }),
            },
            retained,
        ))
    }
    pub(super) fn check_owner_raw(
        &mut self,
        capture: &sc::AuditCapture,
        bound: &super::audit_owner::BoundOriginals,
        native: &super::audit_owner::native::Budget,
    ) -> Result<usize, super::audit_owner::native::Failure> {
        let historical = match &mut self.historical {
            HistoricalOwner::Unique(v) => v,
            HistoricalOwner::Shared(v) => super::native_shared::NativeRc::get_mut(v)
                .ok_or_else(|| err("raw owner 仍有借用"))?,
        };
        historical
            .raw_prefixes
            .as_mut()
            .ok_or_else(|| err("缺 raw R/F 状态"))?
            .check_appended(capture, self.raw.view(), bound, native)
    }
}
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn trace_owner_effects(rows: usize, peak: usize, raw: usize, cuts: usize) {
    AUDIT_TRACE.with(|t| if let Some(t)=t.borrow_mut().as_mut() {
        t.events.push(json!({"phase":"owner_effects","actual_rows":rows,"effects_peak":peak,"raw":raw,"cuts":cuts}));
    });
}
/// #1488 0044 P3-3：请求收尾保留裁决点的生产读数（complete_retention_cap 与 construction 实际 retained）。
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn trace_owner_retention(cap: usize, raw: usize) {
    AUDIT_TRACE.with(|t| {
        if let Some(t) = t.borrow_mut().as_mut() {
            t.events
                .push(json!({"phase":"owner_retention","cap":cap,"raw":raw}));
        }
    });
}

#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn trace_sql_row(class: usize, bytes: usize) {
    AUDIT_TRACE.with(|t| {
        if let Some(t) = t.borrow_mut().as_mut() {
            t.work.sql_returned_rows[class] += 1;
            t.work.sql_returned_bytes[class] += bytes as u64;
        }
    });
}
#[cfg(test)]
pub(super) fn trace_raw_hashes(source: bool, count: u64) {
    AUDIT_TRACE.with(|t| {
        if let Some(t) = t.borrow_mut().as_mut() {
            if source {
                t.work.raw_source_hashes += count;
            } else {
                t.work.raw_payload_hashes += count;
            }
        }
    });
}

/// 本二进制业务JSON、raw receipt、aux stream 三处直接SHA256的实际调用/输入字节。
#[cfg(test)]
pub(super) fn trace_local_sha256(calls: u64, bytes: usize) {
    AUDIT_TRACE.with(|t| {
        if let Some(t) = t.borrow_mut().as_mut() {
            t.work.local_sha256_calls += calls;
            t.work.local_sha256_bytes += bytes as u64;
        }
    });
}

#[cfg(test)]
pub(super) fn overlay_ab_signature(s: &AuditedState) -> serde_json::Value {
    let digest = |bytes: &[u8]| crate::sha256_hex(bytes);
    let cuts=s.cuts.iter().map(|c|serde_json::json!({"metadata_debug":format!("{:?}",c.metadata),"roots_debug":format!("{:?}",c.roots),"batch_id":c.batch_id,"snapshot":c.snapshot,"delta":c.delta})).collect::<Vec<_>>();
    let components = serde_json::json!({"raw_sha256":digest(crate::canonical_json(&serde_json::json!(s.raw)).as_bytes()),"control_facts":s.control_facts.overlay_ab_control_signature(),"cuts":cuts,"objects_debug_sha256":digest(format!("{:?}",s.objects).as_bytes()),"auxiliary_debug_sha256":digest(format!("{:?}",s.auxiliary).as_bytes()),"budget_present":s.budget.is_some(),"auxiliary_budget_present":!s.auxiliary.overlay_ab_budget_absent(),"raw_prefixes_present":s.raw_prefixes.is_some(),"raw_sources_present":s.raw_sources.is_some(),"auxiliary_facts_present":s.auxiliary_facts.is_some(),"object_facts_present":s.object_facts.is_some()});
    // 该完整签名只接受普通返回布局；任何意外Native状态不以存在性摘要蒙混。
    assert!(
        s.auxiliary.overlay_ab_budget_absent()
            && s.budget.is_none()
            && s.raw_prefixes.is_none()
            && s.raw_sources.is_none()
            && s.auxiliary_facts.is_none()
            && s.object_facts.is_none()
    );
    serde_json::json!({"schema_revision":"ordinary-audit-complete-return/1","sha256":digest(crate::canonical_json(&components).as_bytes()),"components":components})
}

#[cfg(test)]
mod transaction_material_qualification {
    use super::*;
    struct Meter;
    impl audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn accept_prepared_frame(&self, _: &sc::job::CheckedFrame) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn node(&self, _: &sc::NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
            Ok(())
        }
        fn record(&self) -> Result<(), sc::CodecError> {
            Ok(())
        }
    }
    #[test]
    fn actual_entry_seven_exclusions_and_fresh_drop() {
        let mut f = prepared::tests::Fixture::new();
        let tx = f.conn.transaction().unwrap();
        let owned = sc::capture_allocation::OwnedBudget::new(1024 * 1024);
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let meter = Meter;
        for which in 0..8 {
            super::super::ordinary_materials::LAST_REPORT.with(|r| r.set(None));
            let fresh = if which == 7 {
                Some((
                    control_audit::prepare_s3_control_history(&tx).unwrap(),
                    super::super::business_json::RawEvents::from_json_owner(
                        read_raw_events_until_admitted(&tx, None, None).unwrap(),
                    )
                    .unwrap(),
                ))
            } else {
                None
            };
            let result = audit_core_fresh(
                &tx,
                if which == 1 {
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(60))
                } else {
                    None
                },
                if which == 2 { Some(1024 * 1024) } else { None },
                if which == 3 { Some(&meter) } else { None },
                which == 4,
                if which == 5 { Some(&owned) } else { None },
                if which == 6 { Some(&capture) } else { None },
                fresh,
                RetentionReport::Skip,
            );
            if which == 0 {
                result.unwrap();
            }
            // 资格门 `ordinary_materials_qualified` 含 `!cfg!(feature = "s_incremental")`
            // （61b4adec5b）：native 构建下第 0 例同样不得发普通原件，与运行顺序无关。
            super::super::ordinary_materials::LAST_REPORT.with(|r| {
                assert_eq!(
                    r.get().is_some(),
                    which == 0 && !cfg!(feature = "s_incremental"),
                    "qualification case {}",
                    which
                )
            });
        }
        use sc::capture_allocation::scratch::*;
        struct Parent;
        impl ScratchParent for Parent {
            fn reserve(&self, _: usize, _: ScratchCharge) -> Result<u64, ScratchFailure> {
                Ok(0)
            }
            fn release(
                &self,
                _: usize,
                _: ScratchCharge,
                origin: u64,
            ) -> Result<(), ScratchFailure> {
                if origin != 0 {
                    return Err(ScratchFailure::Ledger);
                }
                Ok(())
            }
            fn health(&self) -> Result<(), ScratchFailure> {
                Ok(())
            }
            fn invalidate(&self) {}
        }
        super::super::ordinary_materials::LAST_REPORT.with(|r| r.set(None));
        let _ = CodecWork::new(std::sync::Arc::new(Parent), 1024 * 1024)
            .unwrap()
            .run(|scope| {
                let _ = audit_core_fresh_inner(
                    &tx,
                    None,
                    None,
                    None,
                    false,
                    None,
                    None,
                    None,
                    RetentionReport::Skip,
                    #[cfg(feature = "s_incremental")]
                    None,
                    Some(scope),
                );
                Ok::<_, ()>(())
            });
        super::super::ordinary_materials::LAST_REPORT.with(|r| assert!(r.get().is_none()));
    }
}

/// #1481 对拍规范输出：逐字段完整语义载荷；不包含分配许可、容量或测试计数。
/// Optional Native facts按实际状态逐字段保留，不能用计数/fingerprint代替。
#[cfg(all(test, feature = "s_incremental"))]
impl ImmutableHistoricalAudit {
    pub(super) fn h1_canonical(&self) -> String {
        let Self {
            control_facts,
            raw_prefixes,
            raw_sources,
            auxiliary_facts,
            object_facts,
            cuts,
            objects,
            auxiliary,
        } = self;
        canonical_json(&json!({
            "schema":"immutable-historical-audit/1",
            "control_facts":control_facts.h1_canonical(),
            "raw_prefixes":raw_prefixes.as_ref().map(history::NativeRawPrefixes::h1_canonical),
            "raw_sources":raw_sources.as_ref().map(source_audit::NativeRawFacts::h1_canonical),
            "auxiliary_facts":auxiliary_facts.as_ref().map(aux_index::NativeAuxFacts::h1_canonical),
            "object_facts":object_facts.as_ref().map(object_index::HistorySweep::h1_canonical),
            "cuts":cuts.iter().map(|c| { let history::PublishedCut { metadata, roots, batch_id, snapshot, delta } = c;
                json!({"metadata":metadata,"roots":roots,"batch_id":batch_id,"snapshot":snapshot,"delta":delta}) }).collect::<Vec<_>>(),
            "objects":objects.iter().map(object_index::Entry::h1_canonical).collect::<Vec<_>>(),
            "auxiliary":auxiliary.h1_canonical(),
        }))
    }
}

#[cfg(all(test, feature = "s_incremental"))]
mod h1_certificate_tests {
    use super::*;

    fn compare(tx: &Transaction<'_>) {
        let expected = audit_native_current(tx).unwrap();
        let (result, events, _) =
            traced(false, || HistoricalCertificate::cold(tx, None, None, None));
        let actual = result.unwrap();
        assert_eq!(actual.historical().h1_canonical(), expected.h1_canonical());
        assert_eq!(
            serde_json::to_value(&actual.audited.raw).unwrap(),
            serde_json::to_value(&expected.raw).unwrap(),
        );
        let certified = events
            .iter()
            .filter(|e| e["phase"] == "h1_certified_cut")
            .map(|e| e["generation"].as_i64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            certified,
            (1..=expected.cuts.len() as i64).collect::<Vec<_>>()
        );
        for generation in certified {
            let next = events
                .iter()
                .position(|e| e["phase"] == "h1_certified_cut" && e["generation"] == generation)
                .unwrap();
            if generation < expected.cuts.len() as i64 {
                let following = events
                    .iter()
                    .position(|e| {
                        e["phase"] == "history_decode_manifest" && e["generation"] == generation + 1
                    })
                    .unwrap();
                assert!(next < following, "来源认证必须在读取下一代之前完成");
            }
        }
    }

    #[test]
    fn h1_cert_cold_empty_revision_winner_and_unpublished_tail() {
        let mut fixture = prepared::tests::Fixture::new();
        compare(&fixture.conn.transaction().unwrap());
        for (n, row, revision) in [
            (1, ("a", 20, 6, 12, 3, 7), 1),
            (2, ("b", 0, 8, 16, 6, 12), 1),
            (3, ("a", 20, 8, 13, 4, 7), 2),
        ] {
            let (envelope, token) = fixture.accept_begin(n, row, revision);
            compare(&fixture.conn.transaction().unwrap());
            fixture.publish(&envelope, &token).unwrap();
            compare(&fixture.conn.transaction().unwrap());
        }
        fixture.accept_begin(4, ("tail", 30, 8, 17, 6, 12), 1);
        let tx = fixture.conn.transaction().unwrap();
        compare(&tx);
        let cert = HistoricalCertificate::cold(&tx, None, None, None).unwrap();
        let prefixes = cert.historical().raw_prefixes.as_ref().unwrap();
        assert_eq!(prefixes.checked_len(), 4);
        assert_eq!(prefixes.published_len(), 3);
    }

    #[test]
    fn h1_cert_rejects_control_manifest_current_sql_and_missing_original() {
        let mut fixture = prepared::tests::Fixture::new();
        let (envelope, token) = fixture.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        fixture.publish(&envelope, &token).unwrap();
        for sql in [
            "UPDATE meta SET value='2' WHERE key='generation'",
            "UPDATE s_content_publications SET batch_id='wrong'",
            "UPDATE structure_deltas SET input_frontier=99",
            "UPDATE catalog SET run_status='not_run' WHERE catalog_id='CC-005'",
            "DELETE FROM s_content_nodes",
        ] {
            let tx = fixture.conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            assert!(audit_native_current(&tx).is_err(), "oracle: {sql}");
            assert!(
                HistoricalCertificate::cold(&tx, None, None, None).is_err(),
                "cert: {sql}"
            );
            tx.rollback().unwrap();
        }
    }

    #[test]
    fn h1_cert_deadline_and_scratch_failure_release() {
        let mut fixture = prepared::tests::Fixture::new();
        let (envelope, token) = fixture.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        fixture.publish(&envelope, &token).unwrap();
        let tx = fixture.conn.transaction().unwrap();
        let expired = std::time::Instant::now() - std::time::Duration::from_secs(1);
        assert!(HistoricalCertificate::cold(&tx, Some(expired), None, None).is_err());
        for limit in [1024, 64 * 1024 * 1024] {
            let native = super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
            let baseline = native.live();
            let result = native
                .codec_work()
                .unwrap()
                .run(|scope| HistoricalCertificate::cold(&tx, None, None, Some(scope)));
            if limit == 1024 {
                assert!(result.is_err());
            } else {
                let cert = result.unwrap();
                assert_eq!(cert.historical().cuts.len(), 1);
                drop(cert);
            }
            assert_eq!(native.live(), baseline);
            assert_eq!(native.scratch_observation().0, 0);
        }
    }
}
