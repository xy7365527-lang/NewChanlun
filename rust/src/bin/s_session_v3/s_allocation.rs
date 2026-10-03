//! #1394 TB-03-B：S 写前审计的私有 TestOnly 分配接线。
//! 只覆盖 handle / advance / prepare 的局部审计与已接入 OwnedBudget 的业务工作集。
//! 启动、身份/摘要、preflight、recover 仍走普通审计；不声明整堆、生产容量或完整准入。
//! S7/Q21、公开回复与现有 complete-audit-coverage 拒绝门均保持原口径。

use super::{q_allocation, state};
use rusqlite::Transaction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Handle,
    Advance,
    Prepare,
}

/// 一次调用只读当前事务，Some 不回落普通审计。捕获在本阶段返回前释放；
/// AuditedState 由调用方按原生命周期持有，其与后续阶段重叠的分配仍计同一预算。
pub(super) fn audit(
    tx: &Transaction<'_>,
    allocation: Option<&q_allocation::Profile>,
    phase: Phase,
) -> Result<state::AuditedState, String> {
    let Some(profile) = allocation else {
        return state::audit(tx);
    };
    #[cfg(all(test, feature = "s_allocation_json_testonly"))]
    let _refusal_owner = probe::before(tx, profile, phase)?;
    #[cfg(not(all(test, feature = "s_allocation_json_testonly")))]
    let _ = phase;
    let (audited, capture) = state::audit_bounded_with_allocation(tx, None, None, profile)?;
    #[cfg(all(test, feature = "s_allocation_json_testonly"))]
    probe::captured(tx, profile, phase, &audited)?;
    drop(capture);
    Ok(audited)
}

const TESTONLY_MARKER: &str = "S_ALLOCATION_JSON_TESTONLY";
const INPUTS: [&str; 4] = [
    TESTONLY_MARKER,
    "AUDIT_PROTOTYPE_OWNED_PAYLOAD_BYTES",
    "AUDIT_PROTOTYPE_OVERLAY_SLOTS",
    "AUDIT_PROTOTYPE_OVERLAY_BYTES",
];

/// 仅 serve/advance 的私有实验入口；启动和恢复 pending 之前一次冻结。
/// 所有输入缺省时返回 None；任何部分输入、非 UTF-8、坏值和无 feature 激活都拒绝。
pub(super) fn load_testonly_environment() -> Result<Option<q_allocation::Profile>, String> {
    parse_testonly_environment(|name| match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(format!("SAllocationTestOnlyInvalid：{name} 必须是 UTF-8"))
        }
    })
}

fn parse_testonly_environment(
    mut read: impl FnMut(&str) -> Result<Option<String>, String>,
) -> Result<Option<q_allocation::Profile>, String> {
    let values = INPUTS.map(&mut read);
    let values: Vec<Option<String>> = values.into_iter().collect::<Result<_, _>>()?;
    if values.iter().all(Option::is_none) {
        return Ok(None);
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
        Ok(Some(profile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_testonly_activation_has_no_defaults_and_rejects_partial_or_bad_inputs() {
        assert!(parse_testonly_environment(|_| Ok(None)).unwrap().is_none());
        for missing in 0..4 {
            let result = parse_testonly_environment(|name| {
                let index = INPUTS.iter().position(|key| *key == name).unwrap();
                Ok((index != missing).then(|| "1".into()))
            });
            assert!(result.is_err(), "missing {missing}");
        }
        assert!(parse_testonly_environment(|name| {
            Ok(Some(
                if name == TESTONLY_MARKER { "true" } else { "1" }.into(),
            ))
        })
        .is_err());
        assert!(parse_testonly_environment(|_| Err("non-UTF8".into())).is_err());
        for bad in ["0", "-1", "01", "1.0", "", "184467440737095516160"] {
            assert!(
                parse_testonly_environment(|name| {
                    Ok(Some(if name == TESTONLY_MARKER { "1" } else { bad }.into()))
                })
                .is_err(),
                "{bad}"
            );
        }
        let complete = parse_testonly_environment(|_| Ok(Some("1".into())));
        #[cfg(feature = "s_allocation_json_testonly")]
        assert_eq!(complete.unwrap().unwrap().budget.snapshot().limit_bytes, 1);
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        assert!(complete
            .err()
            .unwrap()
            .contains("prototype_audit_allocation_incomplete"));
    }
}

/// 仅测试观测，固定最多三条；记录和额外 SQL 不计入分配覆盖或服务资源声明。
/// 每条成功记录只在本次 audit_bounded_with_allocation 实际返回新 capture 后写入，
/// 不靠地址推断捕获次数，也不改变公开回复。
#[cfg(all(test, feature = "s_allocation_json_testonly"))]
pub(super) mod probe {
    use super::*;
    use newchan_rust::s_content::capture_allocation::CaptureSlab;
    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    pub(in super::super) enum Fault {
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
        profile: &q_allocation::Profile,
        phase: Phase,
    ) -> Result<Option<CaptureSlab>, String> {
        let fault = PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if !p.enabled {
                return None;
            }
            p.attempts[index(phase)] += 1;
            p.live_before[index(phase)] = profile.budget.snapshot().live_requested_layout_bytes;
            p.fault
                .filter(|(target, _)| *target == phase)
                .map(|(_, fault)| fault)
        });
        match fault {
            Some(Fault::Budget) => {
                // 真实 OwnedBudget 分配占满余量，后续实际申请被账本拒绝；非伪造 Err。
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
                tx.progress_handler(1, Some(|| true));
                Ok(None)
            }
            None => Ok(None),
        }
    }
    pub(super) fn captured(
        tx: &Transaction<'_>,
        profile: &q_allocation::Profile,
        phase: Phase,
        audited: &state::AuditedState,
    ) -> Result<(), String> {
        if !PROBE.with(|p| p.borrow().enabled) {
            return Ok(());
        }
        // 证明返回的 owned 原始账与 Driver/Runtime 的预算确是同一账本。
        assert!(audited
            .budget
            .as_ref()
            .unwrap()
            .same_ledger(&profile.budget));
        assert!(audited.raw.budget().unwrap().same_ledger(&profile.budget));
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
