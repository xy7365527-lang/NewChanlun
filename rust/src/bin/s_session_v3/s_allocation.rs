//! #1394 TB-03-B：S 写前审计的互斥私有 TestOnly 模式。
//! Owned 沿同一局部分配账；CapturedNative 使用普通容器，不提供整堆分配统计。
//! Ordinary/Owned启动、身份/摘要、preflight、recover 仍走普通审计；显式Native启动另走同核完整捕获；不声明整堆、生产容量或完整准入。
//! S7/Q21、公开回复与现有 complete-audit-coverage 拒绝门均保持原口径。

use super::{q_allocation, state};
use rusqlite::Transaction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Handle,
    Advance,
    Prepare,
}

/// S 私有启动模式；不是失败后的重判或新公开资源合同。Q/reader 不消费本类型。
#[derive(Clone)]
pub(super) enum Mode {
    Ordinary,
    Owned(q_allocation::Profile),
    CapturedNative,
}
impl From<Option<q_allocation::Profile>> for Mode {
    fn from(profile: Option<q_allocation::Profile>) -> Self {
        match profile {
            Some(profile) => Self::Owned(profile),
            None => Self::Ordinary,
        }
    }
}
impl Mode {
    pub(super) fn validate(&self) -> Result<(), String> {
        match self {
            Self::Ordinary => Ok(()),
            Self::Owned(profile) => profile.validate(),
            Self::CapturedNative => {
                #[cfg(feature = "s_incremental")]
                return Ok(());
                #[cfg(not(feature = "s_incremental"))]
                Err(
                    "SNativeCaptureTestOnlyUnsupported：此构建不支持 S_CAPTURE_NATIVE_TESTONLY"
                        .into(),
                )
            }
        }
    }
}

/// 每次读取当前事务；选定的臂失败直接传播。捕获在该阶段返回前释放，
/// AuditedState 仍按原生命周期存活，Owned 跨阶段/请求同账，Native 不宣称整堆统计。
/// Startup没有写前Phase计数或keeper；Owned启动保持原普通审计，不误切分配臂。
pub(super) fn audit_startup(
    tx: &Transaction<'_>,
    mode: &Mode,
) -> Result<state::AuditedState, String> {
    match mode {
        Mode::CapturedNative => state::audit_native_current(tx),
        Mode::Ordinary | Mode::Owned(_) => state::audit(tx),
    }
}

pub(super) fn audit(
    tx: &Transaction<'_>,
    mode: &Mode,
    phase: Phase,
) -> Result<state::AuditedState, String> {
    audit_impl(tx, mode, phase, None, None)
}
pub(super) fn audit_in_request(
    tx: &Transaction<'_>,
    mode: &Mode,
    phase: Phase,
    context: &mut super::request_audit::RequestAuditContext,
) -> Result<state::AuditedState, String> {
    audit_in_request_admitted(tx, mode, phase, context, None)
}

/// 显式Native父域只贯通已接线的捕获/绑定子域；无scope入口保持原选择。
pub(super) fn audit_in_request_admitted(
    tx: &Transaction<'_>,
    mode: &Mode,
    phase: Phase,
    context: &mut super::request_audit::RequestAuditContext,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<state::AuditedState, String> {
    let result = audit_impl(tx, mode, phase, Some(&mut *context), work);
    if result.is_err() {
        drop(context.take());
    }
    result
}
fn audit_impl(
    tx: &Transaction<'_>,
    mode: &Mode,
    phase: Phase,
    context: Option<&mut super::request_audit::RequestAuditContext>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<state::AuditedState, String> {
    if work.is_some() && !matches!(mode, Mode::CapturedNative) {
        return Err("SNativeAdmissionUnsupported：显式父域只适用于Native捕获臂".into());
    }
    let profile = match mode {
        Mode::Ordinary => return state::audit(tx),
        Mode::Owned(profile) => Some(profile),
        Mode::CapturedNative => None,
    };
    #[cfg(all(
        test,
        any(feature = "s_allocation_json_testonly", feature = "s_incremental")
    ))]
    let _refusal_owner = probe::before(tx, profile, phase)?;
    #[cfg(not(all(
        test,
        any(feature = "s_allocation_json_testonly", feature = "s_incremental")
    )))]
    let _ = (phase, profile);
    let audited = match mode {
        Mode::Owned(profile) => {
            let (audited, capture) = state::audit_bounded_with_allocation(tx, None, None, profile)?;
            drop(capture);
            audited
        }
        Mode::CapturedNative => match context {
            Some(context) => state::audit_native_in_request_admitted(tx, context, work)?,
            None => state::audit_native_current(tx)?,
        },
        Mode::Ordinary => unreachable!(),
    };
    #[cfg(all(
        test,
        any(feature = "s_allocation_json_testonly", feature = "s_incremental")
    ))]
    probe::captured(tx, profile, phase, &audited)?;
    Ok(audited)
}

#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
const TESTONLY_MARKER: &str = "S_ALLOCATION_JSON_TESTONLY";
#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
const NATIVE_MARKER: &str = "S_CAPTURE_NATIVE_TESTONLY";
#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
const INPUTS: [&str; 4] = [
    TESTONLY_MARKER,
    "AUDIT_PROTOTYPE_OWNED_PAYLOAD_BYTES",
    "AUDIT_PROTOTYPE_OVERLAY_SLOTS",
    "AUDIT_PROTOTYPE_OVERLAY_BYTES",
];

/// 仅 serve/advance 的模式入口；启动和恢复 pending 之前一次冻结。
/// #1640（ADR 0042 D1 / 计划 §1.1）：`s_incremental` 生产构建无运行时开关，编译期固定为
/// CapturedNative，不读取任何私有选择输入；env 选择只编入默认构建与 testonly 对照臂。
/// 默认构建所有输入缺省时返回 Ordinary；部分/混合输入、非 UTF-8、坏值与无 feature 激活都拒绝。
pub(super) fn load_environment_mode() -> Result<Mode, String> {
    #[cfg(all(feature = "s_incremental", not(feature = "s_capture_native_testonly")))]
    return incremental_mode();
    #[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
    parse_selection_environment(read_selection_input)
}

/// #1640：生产增量构建的唯一模式；不读环境，不回落 Ordinary/Owned。
#[cfg(all(feature = "s_incremental", not(feature = "s_capture_native_testonly")))]
fn incremental_mode() -> Result<Mode, String> {
    let mode = Mode::CapturedNative;
    mode.validate()?;
    Ok(mode)
}

#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
fn read_selection_input(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(format!("SAllocationTestOnlyInvalid：{name} 必须是 UTF-8"))
        }
    }
}

// #1640 §2.3 第 1 层：生产增量构建不得同时带分配 JSON 实验钩子。
#[cfg(all(
    feature = "s_incremental",
    feature = "s_allocation_json_testonly",
    not(feature = "s_capture_native_testonly"),
    not(test)
))]
compile_error!("s_incremental 生产构建不得启用 s_allocation_json_testonly（#1640 / ADR 0042）");

// #1640：生产增量构建不编入 env 选择臂；默认构建的 Ordinary 入口与 testonly 对照臂仍走这里。
#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
fn parse_selection_environment(
    mut read: impl FnMut(&str) -> Result<Option<String>, String>,
) -> Result<Mode, String> {
    let native = read(NATIVE_MARKER)?;
    let values = INPUTS.map(&mut read);
    let values: Vec<Option<String>> = values.into_iter().collect::<Result<_, _>>()?;
    if let Some(native) = native {
        if native != "1" || values.iter().any(Option::is_some) {
            return Err("SAllocationTestOnlyInvalid：S_CAPTURE_NATIVE_TESTONLY 必须为1且不得混用任意 Owned 私有输入".into());
        }
        let mode = Mode::CapturedNative;
        mode.validate()?;
        return Ok(mode);
    }
    if values.iter().all(Option::is_none) {
        return Ok(Mode::Ordinary);
    }
    if values[0].as_deref() != Some("1") || values.iter().any(Option::is_none) {
        return Err("SAllocationTestOnlyInvalid：必须显式 S_ALLOCATION_JSON_TESTONLY=1 且同时给出三个 AUDIT_PROTOTYPE 分配值，无默认值".into());
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    return Err("prototype_audit_allocation_incomplete：此构建不支持 S TestOnly 分配实验".into());
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        let number = |index: usize| -> Result<usize, String> {
            let value = values[index].as_deref().unwrap();
            newchan_rust::session_protocol::positive(value, INPUTS[index])
                .and_then(|n| {
                    usize::try_from(n).map_err(|_| format!("{} 超出 usize 域", INPUTS[index]))
                })
                .map_err(|e| format!("SAllocationTestOnlyInvalid：{e}"))
        };
        let profile = q_allocation::Profile {
            budget: newchan_rust::s_content::capture_allocation::OwnedBudget::new(number(1)?),
            overlay_slots: number(2)?,
            overlay_bytes: number(3)?,
        };
        profile.validate()?;
        Ok(Mode::Owned(profile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
    #[test]
    fn explicit_testonly_activation_has_no_defaults_and_rejects_partial_or_bad_inputs() {
        assert!(matches!(
            parse_selection_environment(|_| Ok(None)).unwrap(),
            Mode::Ordinary
        ));
        for missing in 0..4 {
            let result = parse_selection_environment(|name| {
                if name == NATIVE_MARKER {
                    return Ok(None);
                }
                let index = INPUTS.iter().position(|key| *key == name).unwrap();
                Ok((index != missing).then(|| "1".into()))
            });
            assert!(result.is_err(), "missing {missing}");
        }
        assert!(parse_selection_environment(|name| {
            if name == NATIVE_MARKER {
                return Ok(None);
            }
            Ok(Some(
                if name == TESTONLY_MARKER { "true" } else { "1" }.into(),
            ))
        })
        .is_err());
        assert!(parse_selection_environment(|_| Err("non-UTF8".into())).is_err());
        for bad in ["0", "-1", "01", "1.0", "", "184467440737095516160"] {
            assert!(
                parse_selection_environment(|name| {
                    if name == NATIVE_MARKER {
                        return Ok(None);
                    }
                    Ok(Some(if name == TESTONLY_MARKER { "1" } else { bad }.into()))
                })
                .is_err(),
                "{bad}"
            );
        }
        let complete =
            parse_selection_environment(|name| Ok((name != NATIVE_MARKER).then(|| "1".into())));
        #[cfg(feature = "s_allocation_json_testonly")]
        match complete.unwrap() {
            Mode::Owned(profile) => assert_eq!(profile.budget.snapshot().limit_bytes, 1),
            _ => panic!("explicit Owned selection"),
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        assert!(complete
            .err()
            .unwrap()
            .contains("prototype_audit_allocation_incomplete"));
    }
    #[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
    #[test]
    fn native_selection_is_explicit_mutually_exclusive_and_feature_gated() {
        for old in INPUTS {
            assert!(
                parse_selection_environment(|name| {
                    Ok((name == NATIVE_MARKER || name == old).then(|| "1".into()))
                })
                .is_err(),
                "mixed {old}"
            );
        }
        for bad in ["", "0", "01", "true", "-1"] {
            assert!(
                parse_selection_environment(|name| {
                    Ok((name == NATIVE_MARKER).then(|| bad.into()))
                })
                .is_err(),
                "native {bad}"
            );
        }
        assert!(parse_selection_environment(|name| {
            if name == NATIVE_MARKER {
                Err("native non-UTF8".into())
            } else {
                Ok(None)
            }
        })
        .is_err());
        let native =
            parse_selection_environment(|name| Ok((name == NATIVE_MARKER).then(|| "1".into())));
        #[cfg(feature = "s_incremental")]
        assert!(matches!(native.unwrap(), Mode::CapturedNative));
        #[cfg(not(feature = "s_incremental"))]
        assert!(native
            .err()
            .unwrap()
            .starts_with("SNativeCaptureTestOnlyUnsupported："));
        // 从Owned失败不会切Native，混合完整Owned档也在feature/额度核验前拒绝。
        assert!(parse_selection_environment(|_| Ok(Some("1".into()))).is_err());
    }
    /// #1640 D1：生产增量构建无运行时开关——编译期固定 CapturedNative，入口不编入任何 env 选择臂。
    #[cfg(all(feature = "s_incremental", not(feature = "s_capture_native_testonly")))]
    #[test]
    fn incremental_build_mode_is_compile_time_fixed_captured_native() {
        assert!(matches!(incremental_mode().unwrap(), Mode::CapturedNative));
        assert!(matches!(
            load_environment_mode().unwrap(),
            Mode::CapturedNative
        ));
    }
}

/// 仅测试观测，固定最多三条；记录和额外 SQL 不计入分配覆盖或服务资源声明。
/// 每条成功记录只在本次选定审计 helper 实际现场捕获成功后写入，
/// 不靠地址推断捕获次数，也不改变公开回复。
#[cfg(all(
    test,
    any(feature = "s_allocation_json_testonly", feature = "s_incremental")
))]
pub(super) mod probe {
    use super::*;
    use newchan_rust::s_content::capture_allocation::CaptureSlab;
    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    pub(in super::super) enum Fault {
        #[cfg_attr(
            not(feature = "s_allocation_json_testonly"),
            allow(
                dead_code,
                reason = "#1608：仅 s_allocation_json_testonly 的 driver.rs 测试构造"
            )
        )]
        Budget,
        SqlPrepare,
        SqlStep,
    }
    #[derive(Clone, Copy, Debug)]
    pub(in super::super) struct Observation {
        pub phase: Phase,
        pub raw: usize,
        pub clocks: i64,
        pub has_begin: bool,
        pub live_before: usize,
        pub capture_return_ordinal: usize,
    }
    #[derive(Default)]
    struct Probe {
        enabled: bool,
        fault: Option<(Phase, Fault)>,
        attempts: [usize; 3],
        live_before: [usize; 3],
        captures: [Option<Observation>; 3],
        returned: usize,
    }
    thread_local! { static PROBE: RefCell<Probe> = RefCell::new(Probe::default()); }
    fn index(phase: Phase) -> usize {
        match phase {
            Phase::Handle => 0,
            Phase::Advance => 1,
            Phase::Prepare => 2,
        }
    }
    pub(in super::super) fn start(fault: Option<(Phase, Fault)>) {
        PROBE.with(|p| {
            *p.borrow_mut() = Probe {
                enabled: true,
                fault,
                ..Default::default()
            }
        });
    }
    pub(in super::super) fn finish() -> ([usize; 3], [Option<Observation>; 3]) {
        PROBE.with(|p| {
            let p = p.replace(Probe::default());
            (p.attempts, p.captures)
        })
    }
    pub(super) fn before(
        tx: &Transaction<'_>,
        profile: Option<&q_allocation::Profile>,
        phase: Phase,
    ) -> Result<Option<CaptureSlab>, String> {
        let fault = PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if !p.enabled {
                return None;
            }
            p.attempts[index(phase)] += 1;
            p.live_before[index(phase)] =
                profile.map_or(0, |p| p.budget.snapshot().live_requested_layout_bytes);
            p.fault
                .filter(|(target, _)| *target == phase)
                .map(|(_, fault)| fault)
        });
        match fault {
            Some(Fault::Budget) => {
                // 真实 OwnedBudget 分配占满余量，后续实际申请被账本拒绝；非伪造 Err。
                let profile = profile.ok_or("TestOnly：Budget 故障仅用于 Owned 模式")?;
                let s = profile.budget.snapshot();
                CaptureSlab::new(
                    0,
                    s.limit_bytes - s.live_requested_layout_bytes,
                    &profile.budget,
                )
                .map(Some)
                .map_err(|e| e.to_string())
            }
            Some(Fault::SqlPrepare) => {
                tx.execute_batch("ALTER TABLE s_content_nodes RENAME TO s_testonly_hidden_nodes")
                    .map_err(|e| e.to_string())?;
                Ok(None)
            }
            Some(Fault::SqlStep) => {
                tx.progress_handler(1, Some(|| true))
                    .map_err(|e| e.to_string())?;
                Ok(None)
            }
            None => Ok(None),
        }
    }
    pub(super) fn captured(
        tx: &Transaction<'_>,
        profile: Option<&q_allocation::Profile>,
        phase: Phase,
        audited: &state::AuditedState,
    ) -> Result<(), String> {
        if !PROBE.with(|p| p.borrow().enabled) {
            return Ok(());
        }
        match profile {
            Some(profile) => {
                assert!(audited
                    .budget
                    .as_ref()
                    .unwrap()
                    .same_ledger(&profile.budget));
                assert!(audited.raw.budget().unwrap().same_ledger(&profile.budget));
            }
            None => {
                assert!(audited.budget.is_none());
                assert!(audited.raw.budget().is_none());
            }
        }
        let clocks = tx
            .query_row("SELECT count(*) FROM s_clock_events", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let has_begin = super::super::meta_get_opt(tx, "advance_state")?.as_deref() != Some("idle");
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            p.returned += 1;
            p.captures[index(phase)] = Some(Observation {
                phase,
                raw: audited.raw.len(),
                clocks,
                has_begin,
                live_before: p.live_before[index(phase)],
                capture_return_ordinal: p.returned,
            });
        });
        Ok(())
    }
}
