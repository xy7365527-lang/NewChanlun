//! #1306 对拍 harness v1：批量 fill loop 的逐 bar 只读观测 sink（`#[cfg(test)]`）。
//!
//! 回放（stream）与批量（fill loop）「共核不共外围」——要逐位对拍三面（信号/状态/账本），
//! 必须先把批量侧**内部**的逐 bar 状态（sep_legs/registry 长度/风控门/risk seeds/TW 谓词/
//! 候选过滤口径/p_t/equity_nav/级别账本步进）外化出来。本模块是那层外化的载体：
//!
//! - `start()` 激活 sink（thread_local，`#[cfg(test)]` 才编译 ⟹ 生产路径零代码零开销）；
//! - fill loop 在决策点调用 `push_decision`/`set_level_step`/`set_registry_len`/`set_final_registry`
//!   （sink 未激活 ⟹ 全部 no-op，仅一次 `active()` 布尔检查）；
//! - `take()` 取回捕获，供 [`theta_pi_diff`](super::theta_pi_diff) 对拍。
//!
//! 与仓库既有 `#[cfg(test)] thread_local` override 惯例（`VOICE_EXEC_OVERRIDE` /
//! `NEST_CERT_GATE_OVERRIDE` / `OPSEM_DUMP_DIR_OVERRIDE`）同型：注入点测试专用，不改生产行为。

use std::cell::RefCell;

use super::super::strategy::coverage::{KThetaRiskGate, SepLeg};
use super::super::strategy::level_ledger::LevelLedgerStep;
use super::super::strategy::persistent::PersistentRegistry;

/// 批量侧单个可交易决策 bar 的只读观测。
#[derive(Debug, Clone)]
pub(super) struct BatchBarObs {
    /// bar 序号（fill loop 的 `i`，与 stream 侧 `bar_idx` 同坐标）。
    pub i: usize,
    /// 决策前真实净持仓 `p_t`（units，有符号手数）。
    pub p_t: f64,
    /// 决策前权益 NAV（`cash + units·px`，`base_units` 的分子）。
    pub equity_nav: f64,
    /// 本步 `StepTrace.sep_legs`（状态面左端）。
    pub sep_legs: Vec<SepLeg>,
    /// 风控门（stream 侧恒 `KThetaRiskGate::open()`；批量侧经 `k_theta_risk_gate`）。
    pub gate: KThetaRiskGate,
    /// risk-close seeds 数量（stream 侧恒 0）。
    pub n_risk_seeds: usize,
    /// TW 谓词 P3/P4 是否在本步产出事件（stream 侧恒 false）。
    pub tw_event: bool,
    /// TW 谓词 P2 关闭的重叠腿数（stream 侧恒 0）。
    pub overlay_closes: usize,
    /// 候选过滤前 `step_gamma.len()`（原始候选数）。
    pub n_gamma_raw: usize,
    /// 候选过滤后 `step_gamma_trade.len()`（χ/nest/entry_stop_recheck 之后）。
    pub n_gamma_trade: usize,
    /// 级别账本镜像本步产出（`level_ledger=None` ⟹ None）。
    pub level_step: Option<LevelLedgerStep>,
    /// 本步 ⑤ 合并后持久注册表元素数（状态面右端的长度读数）。
    pub registry_len: usize,
}

/// 批量侧整窗捕获（逐 bar 观测 + 终态注册表）。
#[derive(Debug, Clone, Default)]
pub(super) struct BatchCapture {
    pub bars: Vec<BatchBarObs>,
    /// 窗口末持久注册表全量（stream 侧对应 `ThetaPiStream::registry()`）。
    pub final_registry: Option<PersistentRegistry>,
}

thread_local! {
    static SINK: RefCell<Option<BatchCapture>> = const { RefCell::new(None) };
}

/// 激活捕获（幂等；已激活时清空重来）。
pub(super) fn start() {
    SINK.with(|s| *s.borrow_mut() = Some(BatchCapture::default()));
}

/// 取回捕获并停用（未激活 ⟹ None）。
pub(super) fn take() -> Option<BatchCapture> {
    SINK.with(|s| s.borrow_mut().take())
}

fn active() -> bool {
    SINK.with(|s| s.borrow().is_some())
}

/// 决策点观测（pi_theta_step 返回后调用；`sep_legs` 只读，激活时克隆）。
#[allow(clippy::too_many_arguments)]
pub(super) fn push_decision(
    i: usize,
    sep_legs: &[SepLeg],
    gate: KThetaRiskGate,
    n_risk_seeds: usize,
    tw_event: bool,
    overlay_closes: usize,
    n_gamma_raw: usize,
    n_gamma_trade: usize,
    p_t: f64,
    equity_nav: f64,
) {
    if !active() {
        return;
    }
    SINK.with(|s| {
        if let Some(cap) = s.borrow_mut().as_mut() {
            cap.bars.push(BatchBarObs {
                i,
                p_t,
                equity_nav,
                sep_legs: sep_legs.to_vec(),
                gate,
                n_risk_seeds,
                tw_event,
                overlay_closes,
                n_gamma_raw,
                n_gamma_trade,
                level_step: None,
                registry_len: 0,
            });
        }
    });
}

/// 级别账本步进观测（`ll.step` 后调用；覆盖最近一条 obs 的 `level_step`）。
pub(super) fn set_level_step(lstep: LevelLedgerStep) {
    if !active() {
        return;
    }
    SINK.with(|s| {
        if let Some(cap) = s.borrow_mut().as_mut() {
            if let Some(last) = cap.bars.last_mut() {
                last.level_step = Some(lstep);
            }
        }
    });
}

/// 持久注册表合并后长度观测（⑤ 段末调用）。
pub(super) fn set_registry_len(len: usize) {
    if !active() {
        return;
    }
    SINK.with(|s| {
        if let Some(cap) = s.borrow_mut().as_mut() {
            if let Some(last) = cap.bars.last_mut() {
                last.registry_len = len;
            }
        }
    });
}

/// 窗口末持久注册表全量快照（fill loop 结束前调用）。
pub(super) fn set_final_registry(registry: &PersistentRegistry) {
    if !active() {
        return;
    }
    SINK.with(|s| {
        if let Some(cap) = s.borrow_mut().as_mut() {
            cap.final_registry = Some(registry.clone());
        }
    });
}
