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
    // root 声部无父 ⟹ parent_invalid=false（恒假占位的委托形态，现行语义逐字保留）。
    exit_decision_for_nested(hv, depth, bar, i, groups, equity_now, false)
}

/// **级联关闭版退出决策生成器**（关⑤方案 A §3.5：`parent_invalid` 恒假占位**实义化**）。
///
/// 与 [`exit_decision_for`] 的唯一差异：`CloseTriggers.parent_invalid`（exit.rs:121）从恒
/// `false` 改为**实参**——对 depth>0 声部，调用方按 [`parent_invalid_at`]
/// （`held[depth-1].is_none() ∨ held[depth-1].exit_pending`）计算；depth=0 根无父 ⟹
/// 调用方恒传 `false`（现行语义保留）。§9 closePred 四析取（X = ¬ParentValid ∨ χ^{σ_p} ∨
/// Stop ∨ RiskClose，`Origin.SubVoiceOpenClose.closePred` line 552-562）自此四项皆实。
pub fn exit_decision_for_nested(
    hv: &HeldVoice,
    depth: usize,
    bar: &Bar,
    i: usize,
    groups: &[Vec<&VoiceDecision>],
    equity_now: f64,
    parent_invalid: bool,
) -> Option<VoiceDecision> {
    // 门关委托（reverse_cert=None ⟹ 不求值任何证书谓词，短路语义逐 bit 保留——bit-exact 回归锁）。
    exit_decision_impl(hv, depth, bar, i, groups, equity_now, parent_invalid, None)
}

/// **★#76 证书门版退出决策生成器**（出场门真链切换，SPEC #73 A 线第三票）。
///
/// 与 [`exit_decision_for_nested`] 的唯一差异：第 8 参 `reverse_cert`——注入的**反向证书准入
/// 查询**闭包（生产侧 = #75 同一 `NestChainGate`/身份桥/`n_delta` 判定；miss ⟹ false，
/// 诚实不准出，禁 fallback v0——Xzd 回退只服务进场，出场反向项无 Xzd 对应物）。
/// 反向项 χ^{σ_p} 的消费对象自此从裸 BspBits 升格为 typed 真链反查；四析取结构不动
/// （`Origin.SubVoiceOpenClose.closePred` line 552-562，零新析取项）。
pub fn exit_decision_for_nested_cert(
    hv: &HeldVoice,
    depth: usize,
    bar: &Bar,
    i: usize,
    groups: &[Vec<&VoiceDecision>],
    equity_now: f64,
    parent_invalid: bool,
    reverse_cert: &mut dyn FnMut(&VoiceDecision) -> bool,
) -> Option<VoiceDecision> {
    exit_decision_impl(hv, depth, bar, i, groups, equity_now, parent_invalid, Some(reverse_cert))
}

/// v0 反向证书基例（对照读出角色，#76 起判定不消费）：对反向开仓决策自身方向
/// δ′ = `voice_side(root_side, depth)` 读证书基例 Conf^{δ′}_e（复用
/// [`super::interp::nest_confirm`]，不 fork 第二套证书判据；单级末端 Conf，
/// Flat ⟹ false）。#76 后 runner 侧仅双读落账 NEST_GATE_EXIT cross 对照差，
/// 不进判定、禁作 fallback。
pub fn reverse_nest_cert_base(d: &VoiceDecision) -> bool {
    let dir = voice_side(d.root_side, d.depth);
    super::interp::nest_confirm(d.level, d.signal_index, &d.bsp, dir)
}

/// 退出判定共享实现（§9 closePred 四析取；`reverse_cert`：`None` ⟹ 裸 bits 现行语义
/// （不求值证书谓词），`Some(q)` ⟹ 反向项升格 `reverse_signal && q(d)`）。
#[allow(clippy::too_many_arguments)]
fn exit_decision_impl(
    hv: &HeldVoice,
    depth: usize,
    bar: &Bar,
    i: usize,
    groups: &[Vec<&VoiceDecision>],
    equity_now: f64,
    parent_invalid: bool,
    mut reverse_cert: Option<&mut dyn FnMut(&VoiceDecision) -> bool>,
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
    // ★#76：reverse_cert=Some(q) ⟹ 反向项升格为 typed 真链反查（reverse_signal && q(d)，
    // 短路纪律：同向候选零查询调用）；None ⟹ 裸 bits 现行语义，不求值任何证书谓词。
    let reverse = groups
        .get(i)
        .map(|ds| {
            ds.iter().any(|d| {
                reverse_signal(hv.side, &d.bsp)
                    && match reverse_cert.as_mut() {
                        None => true,
                        Some(q) => q(d),
                    }
            })
        })
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
        parent_invalid, // ★关⑤实义化：depth>0 由调用方实算；depth=0 恒 false（根无父）
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

/// 父失效判据（关⑤ §3.5，`¬ParentValid_{v,t}` 的单脊柱实例）：
/// depth>0 声部的**父槽空仓 ∨ 父 exit_pending**；depth=0 根无父 ⟹ 恒 `false`。
///
/// `exit_pending` 视同失效：父退出已触发、Close 尚在延迟队列——子声部的父背景已否决
/// （不等 fill 到位再判，否则 fill 前窗口期子腿裸存）。越界防御：父槽不存在 ⟹ true
/// （fail-closed，不假设父有效）。
pub fn parent_invalid_at(held: &[Option<HeldVoice>], depth: usize) -> bool {
    if depth == 0 {
        return false; // 根无父（现行语义保留）
    }
    match held.get(depth - 1) {
        Some(Some(h)) => h.exit_pending,
        _ => true, // 父槽空仓/越界 ⟹ 父失效（fail-closed）
    }
}

/// **cascade 发射**（关⑤ §3.5，M16 AncOK「父关则子关」+ §20 先平后开执行序）：
/// depth `trigger_depth` 的退出触发后，对所有 `j > trigger_depth` 且 `held[j].is_some()`
/// 的更深声部**强制**产退出决策（`exit=true`，复用入场快照），**最深优先**（j 降序），
/// 触发决策自身收尾。
///
/// - `exit_pending` 槽跳过（fill 前抑制重复触发，exit.rs:33-35 机制沿用）。
/// - 空槽/零手数由下游 `plan_orders`（q=0 不产 Close）自然过滤，此处不重复判（单一来源）。
/// - 最深优先使子腿现金先到位、父腿后平（双账本下子腿平仓收/付现金独立成腿，§4.3）。
pub fn cascade_exit_decisions(
    held: &[Option<HeldVoice>],
    trigger_depth: usize,
    trigger_exit: VoiceDecision,
    i: usize,
) -> Vec<VoiceDecision> {
    let mut out = Vec::new();
    for j in (trigger_depth + 1..held.len()).rev() {
        if let Some(hv) = &held[j] {
            if hv.exit_pending {
                continue; // fill 前抑制（一次触发一次平仓）
            }
            let mut d = hv.decision;
            d.exit = true;
            d.enter_ok = false;
            d.signal_index = i;
            d.depth = j as u32;
            out.push(d);
        }
    }
    out.push(trigger_exit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::risk::StopInput;
    use super::super::super::types::{BspBits, Center};

    fn bar_at(idx: usize, o: Tick, h: Tick, l: Tick, c: Tick) -> Bar {
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 100,
            untradable: false,
        }
    }

    /// depth 槽持仓声部（root_side=Long 根；side=voice_side(Long,depth) 偶 Long 奇 Short）。
    fn held_at(depth: u32, exit_pending: bool) -> HeldVoice {
        let side = voice_side(VoiceSide::Long, depth);
        HeldVoice {
            side,
            // 止损：Long=pivot_low 950 / Short=pivot_high 1100（测试 bar 默认不触及）。
            stop: if side == VoiceSide::Long { 950 } else { 1100 },
            decision: VoiceDecision {
                depth,
                root_side: VoiceSide::Long,
                exit: false,
                enter_ok: true,
                bsp: BspBits { buy1: true, ..Default::default() },
                signal_index: depth as usize, // 区分各槽快照
                stop_in: StopInput {
                    pivot_low: 950,
                    pivot_high: 1100,
                    center: Center { zd: 1000, zg: 1080, dd: 940, gg: 1090, start_index: 0, end_index: 5 },
                },
                entry: 1000,
                cost_per_unit: 0.0,
                level: 1,
            },
            exit_pending,
        }
    }

    fn trigger_of(hv: &HeldVoice, i: usize) -> VoiceDecision {
        let mut d = hv.decision;
        d.exit = true;
        d.enter_ok = false;
        d.signal_index = i;
        d
    }

    /// C1（最深优先 cascade）：held[0..3] 全活，depth 0 触发退出 ⟹ 产 3 决策，
    /// 序 [depth2, depth1, depth0]。
    #[test]
    fn cascade_parent_exit_closes_descendants_deepest_first() {
        let held = vec![Some(held_at(0, false)), Some(held_at(1, false)), Some(held_at(2, false))];
        let trigger = trigger_of(&held[0].unwrap(), 7);
        let out = cascade_exit_decisions(&held, 0, trigger, 7);
        assert_eq!(out.len(), 3, "两个更深声部强制退出 + 触发决策收尾");
        assert_eq!(out[0].depth, 2, "最深优先（j 降序）");
        assert_eq!(out[1].depth, 1);
        assert_eq!(out[2].depth, 0, "触发决策（父）收尾");
        assert!(out.iter().all(|d| d.exit && !d.enter_ok), "全部 exit=true 退出态");
        assert!(out.iter().all(|d| d.signal_index == 7), "同一触发 bar");
    }

    /// C2（parent_invalid 单项触发）：held[1] 活、held[0] 空 ⟹ depth1 的 X=true
    /// （stop/reverse/risk 全假对照；四析取自此皆实）。
    #[test]
    fn parent_invalid_fires_when_parent_slot_empty() {
        let h = held_at(1, false); // Short 腿（voice_side(Long,1)），stop=1100
        let bar = bar_at(3, 1000, 1010, 990, 1005); // high 1010 < 1100 不触止损
        let groups: Vec<Vec<&VoiceDecision>> = vec![]; // 无反向信号
        let equity = 1_000_000.0; // >0 ⟹ RiskClose 假
        // 父槽空 ⟹ parent_invalid_at=true（fail-closed）。
        let held = vec![None, Some(h)];
        assert!(parent_invalid_at(&held, 1), "父槽空仓 ⟹ 父失效");
        let out = exit_decision_for_nested(&h, 1, &bar, 3, &groups, equity, true);
        let d = out.expect("parent_invalid=true 单项 ⟹ X=true 产退出决策");
        assert!(d.exit && !d.enter_ok);
        assert_eq!(d.depth, 1);
        assert_eq!(d.signal_index, 3);
        // 对照：parent_invalid=false ⟹ 四项全假 ⟹ None（不误触）。
        assert!(
            exit_decision_for_nested(&h, 1, &bar, 3, &groups, equity, false).is_none(),
            "stop/reverse/risk 全假 + parent_invalid=false ⟹ X=false"
        );
    }

    /// C3（fill 前抑制）：父 exit_pending=true ⟹ 子经 parent_invalid 判失效；
    /// cascade 跳过已 pending 的更深槽（不重复产退出）。
    #[test]
    fn cascade_exit_pending_suppressed() {
        let held = vec![
            Some(held_at(0, true)),  // 父已 pending（退出在延迟队列）
            Some(held_at(1, true)),  // 子已被上轮 cascade 标 pending
            Some(held_at(2, false)), // 孙未触发
        ];
        // 父 pending ⟹ 子的 parent_invalid 判据为真（父背景已否决）。
        assert!(parent_invalid_at(&held, 1), "父 exit_pending ⟹ 子父失效");
        // 再从 depth 0 发射：已 pending 的 depth1 跳过（不重复产退出），depth2 强制，父收尾。
        let trigger = trigger_of(&held[0].unwrap(), 5);
        let out = cascade_exit_decisions(&held, 0, trigger, 5);
        assert_eq!(out.len(), 2, "pending 槽跳过：只孙 + 父");
        assert_eq!(out[0].depth, 2);
        assert_eq!(out[1].depth, 0);
    }

    /// C4（根回归锁）：仅 depth0 活 ⟹ 行为与现行逐字节同（parent_invalid=false 路径回归）。
    #[test]
    fn cascade_root_only_regression() {
        let h = held_at(0, false); // Long 根，stop=950
        let bar = bar_at(3, 960, 965, 900, 920); // low 900 ≤ 950 ⟹ 多头止损触及
        let groups: Vec<Vec<&VoiceDecision>> = vec![];
        let equity = 1_000_000.0;
        let held = vec![Some(h), None, None];
        assert!(!parent_invalid_at(&held, 0), "根无父 ⟹ 恒 false");
        let via_old = exit_decision_for(&h, 0, &bar, 3, &groups, equity);
        let via_new = exit_decision_for_nested(&h, 0, &bar, 3, &groups, equity, false);
        let (a, b) = (via_old.expect("止损触发"), via_new.expect("止损触发"));
        assert_eq!(a.depth, b.depth);
        assert_eq!(a.root_side, b.root_side);
        assert_eq!(a.exit, b.exit);
        assert_eq!(a.enter_ok, b.enter_ok);
        assert_eq!(a.bsp, b.bsp);
        assert_eq!(a.signal_index, b.signal_index);
        assert_eq!(a.entry, b.entry);
        assert_eq!(a.level, b.level);
        // 根触发 ⟹ cascade 无更深声部 ⟹ 仅触发决策自身。
        let out = cascade_exit_decisions(&held, 0, trigger_of(&h, 3), 3);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].depth, 0);
    }
}
