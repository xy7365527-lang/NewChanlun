//! 退出决策生成器（§9 closePred）+ 持仓声部台账 `HeldVoice`（共享单源）。
//!
//! ## 存在论位置（消双源）
//!
//! 退出生成器有**两个消费路径**，必须共享同一逻辑（否则 bit-exact 漂移）：
//! - **回测路径**（[`super::super::backtest::runner`] `plan_and_fill_mtm`）：内部模拟台账
//!   （cash/units/voice_qty）+ 延迟成交队列驱动，equity 由模拟撮合算。
//! - **生产路径**（[`super::super::nautilus::strategy::ThetaCore`]）：持仓真相源 = Nautilus
//!   portfolio，equity 由 venue MtM，Close 订单交 venue 撮合（无内部延迟队列）。
//!
//! 两路共享本模块的 [`HeldVoice`]（缠论语义台账：方向/止损/入场快照）+ [`exit_decision_for`]
//! （§9 closePred → exit=true 决策）+ [`record_held_voice`]（开仓后记台账）。**逻辑单源**——
//! 差异仅在「持仓真相从哪读」（runner 从模拟 units，ThetaCore 从 portfolio snap），退出**判定**
//! 逻辑零分叉。
//!
//! ## 契约锚 `Origin.SubVoiceOpenClose.closePred`（line 552-562）
//!
//! X_{v,t} = ¬ParentValid ∨ χ^{σ_p}（反向信号）∨ Stop ∨ RiskClose。

use super::exec::{close_pred, reverse_signal, stop_hit, CloseTriggers, FillSide};
use super::risk::{global_risk_close, risk_mode, structural_stop, RiskModeInput, StopSide};
use super::voice::{voice_side, VoiceSide};
use super::VoiceDecision;
use super::super::types::{Bar, Tick};

/// 持仓声部台账项（退出决策生成器的状态，对齐 §9 closePred 读出所需的入场快照）。
///
/// 退出判定（§9 closePred：Stop ∨ 反向 BSP ∨ RiskClose）逐 bar 读它：
/// - `side`：持仓方向（`VoiceSide::Long`/`Short`）——决定止损触及方向 + 反向信号方向。
/// - `stop`：结构止损价（`risk::structural_stop` 产出，与 build_open_order 同一函数零偏差）。
/// - `decision`：入场决策快照——退出触发时复用它构造 `exit=true` 决策喂 plan_orders
///   （复用 depth/level/bsp/stop_in/root_side，使 build_exit_order 全平当前 q）。
/// - `exit_pending`：退出已触发但 Close 订单尚在延迟队列（fill 未到）的标记——避免在触发 bar
///   到 fill bar 之间**重复入队**同一退出（spec:50 延迟成交期间不重复触发，关闭谓词一次触发
///   一次平仓；台账在 Close 成交清台账后才允许新触发）。
#[derive(Debug, Clone, Copy)]
pub struct HeldVoice {
    pub side: VoiceSide,
    pub stop: Tick,
    pub decision: VoiceDecision,
    pub exit_pending: bool,
}

/// 记入持仓台账（开仓订单成交后调用）：算止损价 + 存入场快照。
///
/// 止损价用 `risk::structural_stop`（与 `build_open_order` 内同一函数，零口径偏差）。
/// 方向 Flat / 无结构止损 ⟹ 不记台账（无止损则退出生成器的 Stop 项无依据，诚实跳过——
/// 此情形 build_open_order 也返回 None 不开仓，故不应到达；防御性跳过）。
pub fn record_held_voice(held: &mut [Option<HeldVoice>], d: &VoiceDecision) {
    let side = voice_side(d.root_side, d.depth);
    let stop_side = match side {
        VoiceSide::Long => StopSide::Long,
        VoiceSide::Short => StopSide::Short,
        VoiceSide::Flat => return,
    };
    let stop = match structural_stop(stop_side, &d.bsp, &d.stop_in) {
        Some(s) => s,
        None => return, // 无结构止损（不应到达——build_open_order 已 None）
    };
    if let Some(slot) = held.get_mut(d.depth as usize) {
        *slot = Some(HeldVoice { side, stop, decision: *d, exit_pending: false });
    }
}

/// **退出决策生成器（§9 closePred → exit=true 决策，本轮真根因修复核心）**。
///
/// 对持仓声部 `hv` 在当前 bar `bar`（索引 `i`）检查关闭谓词 X_{v,t}（对齐
/// `Origin.SubVoiceOpenClose.closePred`），触发则构造**全平**退出决策（`exit=true`，复用入场
/// 决策快照），喂 plan_orders 产 Close 订单。返回 `None` 当 X=false（不关闭，持仓延续）。
///
/// **四析取项的 root 声部读出**（contract anchor `Origin.SubVoiceOpenClose.closePred` line 552-562）：
/// - `parent_invalid`（¬ParentValid）：**root 声部无父 ⟹ 恒 false**（v0 recognize 只产 depth=0
///   独立根；子声部的父失效判定属嵌套树扩展，v0 未触发——诚实有效域标注）。
/// - `reverse_signal`（χ^{σ_p}）：当前 bar 的开仓 decisions（`groups[i]`）中是否有**反向**方向的
///   决策（持多遇卖侧根 / 持空遇买侧根）。反向 BSP 经 recognize 产成反向开仓决策落在某 exec_index，
///   该 exec_index = i 时即当前 bar 出现反向信号 ⟹ 触发先平（对齐 risk.rs `root_dir_next` case2）。
/// - `stop`（Stop）：当前 bar 价格触及 `hv.stop`（`exec::stop_hit`，复用 spec:52 触及语义）。
/// - `risk_close`（RiskClose）：`risk::global_risk_close(μ_t)`。v0 可计算 μ_t 的 **Insolvent 子集**
///   （equity≤0）——maint_margin/buffer/liq_flag 是账户/场所层输入（§11），v0 未建模，故
///   GlobalRiskClose 在 v0 退化为「权益耗尽」判据（标 L0 有效域：μ_t 全五态需账户层输入，
///   v0 只 discharge equity≤0 这一可计算分量，不臆造 maint_margin）。
///
/// **关闭优先于开启**（line 592）：退出生成器在每 bar **先于**开仓处理（exit_orders_at 在开仓前
/// apply），且退出 Close 订单的 ConflictKey.exit_first=0（spec:54 退出先于开仓）——双重保证关闭赢。
pub fn exit_decision_for(
    hv: &HeldVoice,
    depth: usize,
    bar: &Bar,
    i: usize,
    groups: &[Vec<&VoiceDecision>],
    equity_now: f64,
) -> Option<VoiceDecision> {
    // Stop（line 559）：当前 bar 触及止损价 hv.stop。平仓方向 = 持仓反向（平多=Sell，平空=Buy）。
    let exit_side = match hv.side {
        VoiceSide::Long => FillSide::Sell,
        VoiceSide::Short => FillSide::Buy,
        VoiceSide::Flat => return None, // 不应发生（台账只记 Long/Short）
    };
    let stop = stop_hit(bar, hv.stop, exit_side);

    // 反向信号 χ^{σ_p}（line 596-601）：当前 bar 的开仓 decisions 含反向方向根决策 ⟹ 触发。
    // 用 reverse_signal 判每个当前 bar 决策的 bsp 是否与持仓反向（持多遇卖 / 持空遇买）。
    let reverse = groups
        .get(i)
        .map(|ds| ds.iter().any(|d| reverse_signal(hv.side, &d.bsp)))
        .unwrap_or(false);

    // RiskClose（line 561）：GlobalRiskClose（μ_t ∈ {Insolvent, Liquidation}）。
    // v0 可计算 Insolvent（equity≤0）——maint_margin/buffer/liq_flag 账户层输入未建模，
    // 用占位（maint_margin=0, buffer=0, liq_flag=false）使 μ_t 退化到 equity≤0 ⟹ Insolvent 子集。
    let mode = risk_mode(&RiskModeInput {
        equity: equity_now,
        maint_margin: 0.0,
        buffer1: 0.0,
        buffer2: 0.0,
        liq_flag: false,
    });
    let risk_close = global_risk_close(mode);

    let triggers = CloseTriggers {
        parent_invalid: false, // root 声部无父（恒 false，诚实有效域）
        reverse_signal: reverse,
        stop,
        risk_close,
    };
    if !close_pred(&triggers) {
        return None; // X=false：不关闭，持仓延续
    }

    // X=true：构造全平退出决策（exit=true，复用入场快照）。signal_index = 触发 bar i
    // （build_exit_order 用它经 fill_bar_index 算延迟成交 bar；与本函数外 exit_orders_at[fi] 对齐）。
    let mut exit_d = hv.decision;
    exit_d.exit = true;
    exit_d.enter_ok = false; // 退出态，非进场
    exit_d.signal_index = i;
    exit_d.depth = depth as u32;
    Some(exit_d)
}
