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

// ════════════════════════════════════════════════════════════════════════════
//  #148 T4：祖先耦合收口——AncOK 子树清仓（活动集层）
//
//  ★票面边界：祖先耦合实现在**活动集层结构不变量**，不进通道谓词序列（#148 票面）——
//  本节不引入显式通道，channel.rs 的 P1–P8 谓词语义与槽位排序原样冻结（#147 T3）。
//  父声部终结（如 CloseRoot 命中）时，其全部后代声部经 [`subtree_close`] 剪除（子树清仓），
//  一步后活动集恒满足 `Anc(v) ⊆ A_{t+1}`（[`anc_subset_of_active`]，M16 AncOK 声部层实例）。
//
//  ★#183 归一（#179 裁决：结构对应是硬要求，子树清仓接线进生产 π loop）：
//  本节函数即生产「父关则子关」的**唯一实装**——
//  - coverage.rs 环6 活动集一步更新 A_{t+1}=AncOK[(A_t∖𝒟_x^†)∪ℬ_x] 归一到
//    [`step_active_set_with_subtree_close`]（散装 `ancestor_close_by_id` 已下线）；
//  - runner（`plan_and_fill_mtm_dual`）与 nautilus（`ThetaCore::plan_for_bar`）的级联发射
//    归一到 [`subtree_close_exit_decisions`]（held 槽压缩链投影 → [`subtree_close`]；
//    散装 `cascade_exit_decisions` 已下线——deepest-first + 触发者收尾的发射序由
//    [`subtree_close`] 的代际降序同一兑现，depth 槽线性链是 parent_id 树的线性特例）。
//  归一即删除散装等价物（#183 no-patch 硬约束：不允许旧路径保留为 fallback）。
// ════════════════════════════════════════════════════════════════════════════

use super::interp::ActiveLeg;
use super::super::classifier::recursive_tower::ElementId;
use std::collections::{HashMap, HashSet};

/// 声部层活动集不变量 `∀v∈A, Anc(v)⊆A`（#148 验收2；M16 AncOK 的声部层实例）。
///
/// **祖先耦合实现在活动集层结构不变量，不进通道谓词序列（#148 票面）。**
///
/// 祖先链按 `parent_id` 结构映射上溯（spec §13 `p:C_ℓ→C_{ℓ+1}`）：`is_boundary_root=true`
/// 的腿是根锚（链终止，`parent_id` 不再要求在 A 内——interp.rs ActiveLeg 契约：边界根
/// `parent_id=None` 合法/Stale 根保留）；非根腿的 `parent_id` 必须解析到 A 内某腿并递归成立。
///
/// 防御性上界：链步数 > |A| ⟹ 判 false（fail-closed——parent_id 级别严格递增保证无环，
/// 环只能来自数据损坏，不假设不变量成立）。
pub fn anc_subset_of_active(active: &[ActiveLeg]) -> bool {
    let by_id: HashMap<ElementId, &ActiveLeg> =
        active.iter().map(|l| (l.id, l)).collect();
    active.iter().all(|leg| {
        let mut cur = leg;
        let mut steps = 0usize;
        loop {
            if cur.is_boundary_root {
                return true; // 根锚：链在 A 内完整终止
            }
            steps += 1;
            if steps > active.len() {
                return false; // 环/损坏 ⟹ fail-closed
            }
            match cur.parent_id.and_then(|p| by_id.get(&p)) {
                Some(parent) => cur = parent,
                None => return false, // 父不在 A ⟹ Anc(v)⊄A（孤儿）
            }
        }
    })
}

/// 声部层代际深度：v 沿 `parent_id` 在 `by_id`（A 的 id 索引）内可上溯的步数
/// （根锚/父缺失处终止；防御性上界 |A| 截断）。deepest-first 排序键。
fn generation_depth(leg: &ActiveLeg, by_id: &HashMap<ElementId, &ActiveLeg>, bound: usize) -> usize {
    let mut depth = 0usize;
    let mut cur = leg;
    while !cur.is_boundary_root && depth <= bound {
        match cur.parent_id.and_then(|p| by_id.get(&p)) {
            Some(parent) => {
                depth += 1;
                cur = parent;
            }
            None => break,
        }
    }
    depth
}

/// **子树清仓 `𝒟_x^† = {v∈A_t : ({v}∪Anc(v)) ∩ 𝒟_x ≠ ∅}`**（#148 验收1 核心）：
/// 父声部终结 ⟹ 其全部后代声部同刻纳入关闭集（M16 AncOK「父关则子关」在 parent_id 树上的
/// 传递闭包）。返回序 **deepest-first**（代际深者先平、种子父收尾——生产级联发射序，
/// §20 先平后开：子腿现金先到位；#183 归一后本序即生产唯一发射序）。
///
/// - `seeds` = 直接被裁决终结的声部（interp 规则2 的 𝒟_x，契约 `𝒟_x⊆A_t`；★#233 起含
///   coverage 翻向种子——翻向父腿旧世代终结**不保留于 A**，见下条数济判据）。
/// - 后代判据：v 自身或沿 `parent_id` 链（A 内解析）任一祖先的 id ∈ seeds。
/// - ★#233 **父不在 A 的数济判据**：链上溯断裂（`parent_id` 不在 by_id——翻向父旧世代
///   终结后未保留，coverage.rs `held_flip_terminated` 分派）时，断裂点 `parent_id ∈ seeds`
///   ⟹ **连坐**（父已被裁决终结，后代不得因父缺席而漏清）；`parent_id ∉ seeds` ⟹ 现状
///   不变（孤儿归声部层 AncOK 判，#148 验收2 路径不回退）。仅扩「父是种子但不在 A」一面；
///   父在 A 的既有路径（restore 复活被关父命中种子同清，#183 归一机制）零改。
/// - 不可变：不 mutate 输入，产新 Vec。
pub fn subtree_close(active: &[ActiveLeg], seeds: &[ActiveLeg]) -> Vec<ActiveLeg> {
    let by_id: HashMap<ElementId, &ActiveLeg> =
        active.iter().map(|l| (l.id, l)).collect();
    let seed_ids: HashSet<ElementId> = seeds.iter().map(|l| l.id).collect();
    let bound = active.len();
    let mut closed: Vec<(usize, ActiveLeg)> = active
        .iter()
        .filter(|leg| {
            // 自身或任一 A 内祖先命中种子 ⟹ 属被清子树。
            let mut cur = *leg;
            let mut steps = 0usize;
            loop {
                if seed_ids.contains(&cur.id) {
                    return true;
                }
                if cur.is_boundary_root || steps >= bound {
                    return false;
                }
                steps += 1;
                match cur.parent_id.and_then(|p| by_id.get(&p)) {
                    Some(parent) => cur = *parent,
                    // ★#233 数济：断裂点 parent_id ∈ seeds ⟹ 连坐（父已终结但未保留于 A）；
                    // ∉ seeds ⟹ false（现状：孤儿归声部层 AncOK，不回退 #148 验收2 路径）。
                    None => {
                        return cur.parent_id.is_some_and(|p| seed_ids.contains(&p));
                    }
                }
            }
        })
        .map(|l| (generation_depth(l, &by_id, bound), *l))
        .collect();
    // deepest-first（代际深度降序）；同深保持 A 内原序（stable sort）。
    closed.sort_by(|a, b| b.0.cmp(&a.0));
    closed.into_iter().map(|(_, l)| l).collect()
}

/// **活动集一步更新（子树清仓版）`A_{t+1} = AncOK[(A_t ∖ 𝒟_x^†) ∪ ℬ_x]`**（#148 收口）：
/// 先按 [`subtree_close`] 把关闭种子扩为全子树剪除，再并入新开腿 `opened`（ℬ_x，id 去重、
/// 已在存量者不重复入），最后施声部层 AncOK（祖先链在结果集内不可解析的开腿=孤儿，剪除
/// 不入 A——spec §13「子级腿存在 ⟹ 父容器存在」的 fail-closed 面）。
///
/// **L0 定理（构造内蕴）**：返回集恒满足 [`anc_subset_of_active`]——存量腿的祖先只可能因
/// 子树清仓整链移除（祖先被清 ⟹ 后代同在 𝒟_x^† 内），新开孤儿被终检剪除；单遍全链检查
/// 充分（若 v 的链断裂则其全部后代的链同样断裂，剪除单调，无需迭代到不动点）。
pub fn step_active_set_with_subtree_close(
    active: &[ActiveLeg],
    seeds: &[ActiveLeg],
    opened: &[ActiveLeg],
) -> Vec<ActiveLeg> {
    let closed_ids: HashSet<ElementId> =
        subtree_close(active, seeds).iter().map(|l| l.id).collect();
    // (A_t ∖ 𝒟_x^†) ∪ ℬ_x（id 去重，保序：存量在前、新开在后）。
    let mut raw: Vec<ActiveLeg> =
        active.iter().filter(|l| !closed_ids.contains(&l.id)).copied().collect();
    let mut raw_ids: HashSet<ElementId> = raw.iter().map(|l| l.id).collect();
    for &b in opened {
        if raw_ids.insert(b.id) {
            raw.push(b);
        }
    }
    // 声部层 AncOK：全链检查（单遍充分，剪除单调——见函数级 L0 注记）。
    let by_id: HashMap<ElementId, ActiveLeg> = raw.iter().map(|l| (l.id, *l)).collect();
    let bound = raw.len();
    raw.into_iter()
        .filter(|leg| {
            let mut cur = *leg;
            let mut steps = 0usize;
            loop {
                if cur.is_boundary_root {
                    return true;
                }
                steps += 1;
                if steps > bound {
                    return false;
                }
                match cur.parent_id.and_then(|p| by_id.get(&p)) {
                    Some(parent) => cur = *parent,
                    None => return false,
                }
            }
        })
        .collect()
}

/// **生产级联发射（#183 T4 归一，#179 裁决：结构对应是硬要求）**：depth 槽 `held` 台账
/// 投影压缩链 → [`subtree_close`] → 最深优先退出决策序列（触发者收尾）。
///
/// 生产「父关则子关」级联自此归一到 T4 镜像函数；散装 `cascade_exit_decisions`（depth 槽
/// 线性递推）已随归一下线（归一即删除散装等价物，#183 no-patch 硬约束）。
///
/// depth 槽线性链是 parent_id 树的线性特例：`held[j]` 的父 = 链中前一活槽。投影把非
/// `None` 且非 `exit_pending` 槽按 depth 升序成链（各槽 parent 指向前一槽、链首根锚），
/// [`subtree_close`] 在该投影上的子树闭包 ⟺ 旧 cascade 的「更深活槽全集」；deepest-first
/// 按代际深度降序 ⟺ depth 降序（单脊柱 depth 索引单槽、链内无平局）；触发者（链中种子，
/// 代际最浅）收尾——与旧 cascade 逐点等价（一致性见证
/// `t4_linear_chain_production_cascade_matches_subtree_close_deepest_first`）。
///
/// - `exit_pending` 槽不入链（fill 前抑制重复触发，等价旧 cascade 的 skip；exit.rs:33-35 机制）。
/// - 触发决策 `trigger_exit` 收尾（调用方经 [`exit_decision_for_nested`] 产出，与其余腿的
///   快照重构同构：exit=true ∧ enter_ok=false ∧ signal_index=i ∧ depth=槽位）。
/// - 调用契约：触发槽非 `None` 且非 pending（两处生产调用点——runner `plan_and_fill_mtm_dual`
///   与 nautilus `ThetaCore::plan_for_bar`——的外层循环已过滤，否则 expect fail-closed）。
pub fn subtree_close_exit_decisions(
    held: &[Option<HeldVoice>],
    trigger_depth: usize,
    trigger_exit: VoiceDecision,
    i: usize,
) -> Vec<VoiceDecision> {
    // 压缩链投影：非 None && !exit_pending 槽按 depth 升序成链；parent = 链中前一槽，链首根锚。
    // 投影字段仅 `id/parent_id/is_boundary_root` 参与子树判据；`level/dir/source_index/lambda/
    // op_parent` 同为占位（快照值填充，不读出、id 不流出本函数——占位声明补全，code-review 采纳）。
    let mut legs: Vec<ActiveLeg> = Vec::new();
    let mut prev_id: Option<ElementId> = None;
    for (depth, slot) in held.iter().enumerate() {
        let hv = match slot {
            Some(h) if !h.exit_pending => h,
            _ => continue, // 空槽/pending 槽不入链（fill 前抑制，等价旧 cascade skip）
        };
        let id = ElementId { level: hv.decision.level, ordinal: depth as u64 };
        legs.push(ActiveLeg {
            level: hv.decision.level,
            dir: hv.side,
            source_index: hv.decision.signal_index,
            lambda: hv.decision.signal_index,
            id,
            parent_id: prev_id,
            is_boundary_root: prev_id.is_none(),
            op_parent: prev_id,
        });
        prev_id = Some(id);
    }
    // 种子 = 触发腿（调用契约：触发槽非 None 非 pending ⟹ 必在链中）。
    let seed = legs
        .iter()
        .find(|l| l.id.ordinal == trigger_depth as u64)
        .copied()
        .expect("调用契约：触发槽非 None 非 pending（外层循环已过滤）⟹ 触发腿在投影链中");
    // 子树清仓（T4 镜像函数）：触发者的全部后代（= 链上更深活槽）同刻纳入关闭集，deepest-first。
    let closed = subtree_close(&legs, &[seed]);
    // 映射回退出决策：触发者用传入决策收尾，其余按持仓快照重构（与旧 cascade 逐字段同构）。
    closed
        .iter()
        .map(|l| {
            let depth = l.id.ordinal as usize;
            if depth == trigger_depth {
                return trigger_exit;
            }
            let hv = held[depth].expect("投影链腿来自非 None 槽");
            let mut d = hv.decision;
            d.exit = true;
            d.enter_ok = false;
            d.signal_index = i;
            d.depth = depth as u32;
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::channel::{decision_of, ChannelDecision, ChannelId, CHANNEL_M};
    use super::super::coverage::{Dir, GradeRel, Horizontal, OperationRole, Vertical};
    use super::super::interp::{interpret_with_close_triggers, ActiveLeg, Candidate, ExitType};
    use super::super::risk::StopInput;
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::super::types::{BspBits, Center};

    // ── #148 T4 构造器：活动集层 ActiveLeg 树 ────────────────────────────

    /// 树腿构造：parent=None ⟹ 边界根（is_boundary_root=true）；否则子腿。
    fn tleg(level: u32, ordinal: u64, dir: VoiceSide, parent: Option<ElementId>) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: ordinal as usize,
            lambda: ordinal as usize,
            id: ElementId { level, ordinal },
            parent_id: parent,
            is_boundary_root: parent.is_none(),
            op_parent: None,
        }
    }

    fn sell1_cand(level: u32) -> Candidate {
        Candidate {
            level,
            source_index: 99,
            bits: BspBits { sell1: true, ..Default::default() },
            dir: VoiceSide::Short,
            bsp_class: 1,
            role: OperationRole {
                h: Horizontal::First,
                v: Vertical::Ambient,
                delta: Dir::Minus,
                grade: GradeRel::SameLevel,
            },
            nest_confirmed: true,
            gamma_index: 0,
            force: None,
        }
    }

    /// 三层链树：root(L2,Long) ← child(L1,Short) ← grand(L0,Long)。
    fn chain3() -> (ActiveLeg, ActiveLeg, ActiveLeg) {
        let root = tleg(2, 1, VoiceSide::Long, None);
        let child = tleg(1, 10, VoiceSide::Short, Some(root.id));
        let grand = tleg(0, 100, VoiceSide::Long, Some(child.id));
        (root, child, grand)
    }

    // ── #148 T4 验收1：父声部命中 CloseRoot ⟹ 子树同刻清仓，无孤儿 ──────

    /// 验收1（端到端）：反向候选命中父声部（root）经 interpret 产 𝒟_x={root}，
    /// 子树清仓把全部后代（child/grand）同刻纳入关闭集，A_{t+1} 无孤儿声部存活。
    #[test]
    fn t4_parent_close_root_liquidates_full_subtree_no_orphans() {
        let (root, child, grand) = chain3();
        let active = vec![root, child, grand];
        // 反向候选只命中同级别（L2）的 root——interpret 规则2 的 𝒟_x 不含后代。
        let (buckets, _) = interpret_with_close_triggers(&[sell1_cand(2)], &active);
        assert_eq!(buckets.close.len(), 1, "前提：interpret 只关父声部");
        assert_eq!(buckets.close[0].id, root.id);
        // T4 收口：子树清仓扩到全部后代。
        let closed = subtree_close(&active, &buckets.close);
        let closed_ids: Vec<ElementId> = closed.iter().map(|l| l.id).collect();
        assert_eq!(closed.len(), 3, "父 + 全部后代同刻终结");
        assert!(closed_ids.contains(&child.id) && closed_ids.contains(&grand.id));
        // 最深优先（生产级联 deepest-first 语义），父收尾。
        assert_eq!(closed_ids, vec![grand.id, child.id, root.id]);
        // A_{t+1} 无孤儿：全清空且不变量成立。
        let next = step_active_set_with_subtree_close(&active, &buckets.close, &[]);
        assert!(next.is_empty(), "无孤儿声部存活");
        assert!(anc_subset_of_active(&next));
    }

    /// 验收1 补充：中位父（child）终结 ⟹ 只清其子树 {grand, child}，root 存活且不变量成立。
    #[test]
    fn t4_mid_parent_close_liquidates_only_its_subtree() {
        let (root, child, grand) = chain3();
        let active = vec![root, child, grand];
        let closed = subtree_close(&active, &[child]);
        assert_eq!(closed.iter().map(|l| l.id).collect::<Vec<_>>(), vec![grand.id, child.id]);
        let next = step_active_set_with_subtree_close(&active, &[child], &[]);
        assert_eq!(next.len(), 1, "root 不在子树内 ⟹ 存活");
        assert_eq!(next[0].id, root.id);
        assert!(anc_subset_of_active(&next));
    }

    /// ★#233 翻向父**不在 A** 的后代数济（#227 裁决「父翻向 = 父终结」的清仓数济）：
    /// 翻向父腿旧世代终结后**不保留于活动集**（方向盲对位废除，coverage.rs `held_flip_
    /// terminated` 分派）——其后代沿 parent_id 链上溯时链断裂（父 ∉ by_id）。断裂点
    /// parent_id ∈ seeds ⟹ **连坐**（父已被裁决终结，其后代不得因父缺席而漏清）；
    /// parent_id ∉ seeds ⟹ 现状不变（孤儿留给声部层 AncOK 剪，#148 验收2 路径不回退）。
    ///
    /// **RED（判据前）**：断裂即返回 false ⟹ 翻向父的子树漏清（父不死、子树存活，
    /// leg-2-98 勘察 §5「死法②被系统性摘除」的活动集层形态）。**GREEN（判据后）**：
    /// 种子缺席 A 仍连清全子树（deepest-first 序保留）。
    #[test]
    fn subtree_close_liquidates_descendants_of_seed_absent_from_active() {
        let (root, child, grand) = chain3();
        // 翻向父（root）旧世代终结 ⟹ 不在 active（不保留形态）；但其 id 在 seeds（翻向种子）。
        let active = vec![child, grand];
        let closed = subtree_close(&active, &[root]);
        let ids: Vec<ElementId> = closed.iter().map(|l| l.id).collect();
        assert_eq!(
            ids,
            vec![grand.id, child.id],
            "父不在 A 但 ∈ seeds ⟹ 子树连坐清仓（deepest-first）；实得 {ids:?}"
        );
        let next = step_active_set_with_subtree_close(&active, &[root], &[]);
        assert!(next.is_empty(), "子树全清 ⟹ A_{{t+1}} 空");
        assert!(anc_subset_of_active(&next));
        // 对照（现状语义不回退）：parent_id ∉ seeds ⟹ 断裂不清（孤儿归声部层 AncOK 判）。
        let closed_no_seed = subtree_close(&active, &[]);
        assert!(closed_no_seed.is_empty(), "无种子 ⟹ 不清（现状语义不变）");
    }

    // ── #148 T4 验收2：任意裁决序列回放后 ∀v∈A, Anc(v)⊆A ────────────────

    /// 验收2（不变量）：任意裁决序列（开/关种子交错，含孤儿开仓输入）回放，
    /// 每步后 ∀v∈A, Anc(v)⊆A 恒成立（孤儿开仓被声部层 AncOK 剪除，不入 A）。
    #[test]
    fn t4_active_set_invariant_holds_under_arbitrary_verdict_replay() {
        let (root, child, grand) = chain3();
        let root2 = tleg(3, 2, VoiceSide::Short, None);
        let child2 = tleg(2, 20, VoiceSide::Long, Some(root2.id));
        // 孤儿腿：父 id 不存在于任何活动集（须被剪除，不得入 A）。
        let orphan = tleg(0, 999, VoiceSide::Long, Some(ElementId { level: 5, ordinal: 777 }));
        // 裁决序列：(关闭种子, 开启集) 逐步回放。
        let steps: Vec<(Vec<ActiveLeg>, Vec<ActiveLeg>)> = vec![
            (vec![], vec![root]),                 // 开根
            (vec![], vec![child, grand]),         // 开子/孙
            (vec![], vec![root2, orphan]),        // 开第二根 + 孤儿（孤儿须剪）
            (vec![child], vec![child2]),          // 关 child 子树 + 开 child2
            (vec![root], vec![]),                 // 关 root 子树
            (vec![root2], vec![orphan]),          // 关 root2 子树（child2 级联）+ 再喂孤儿
        ];
        let mut active: Vec<ActiveLeg> = vec![];
        for (i, (seeds, opened)) in steps.iter().enumerate() {
            active = step_active_set_with_subtree_close(&active, seeds, opened);
            assert!(
                anc_subset_of_active(&active),
                "step {i}: 不变量 ∀v∈A, Anc(v)⊆A 破裂：{active:?}"
            );
            assert!(
                active.iter().all(|l| l.id != orphan.id),
                "step {i}: 孤儿声部不得入活动集"
            );
        }
        assert!(active.is_empty(), "全部子树清仓后 A=∅（root2 关 ⟹ child2 级联）");
    }

    // ── #148 T4 验收3：子树清仓不占通道谓词槽位（P1–P8 冻结） ────────────

    /// 验收3：T4 不进通道谓词序列——CHANNEL_M 仍为 8，通道裁决映射与 #147 T3 冻结版
    /// 逐条相同（不存在「子树清仓」通道/裁决变体）。
    /// （#282：C5 `OpenShortDiff` 随 P5 槽删除——`decision_of(Cj(5))` 不可达，本表不含 C5 行。）
    #[test]
    fn t4_subtree_close_takes_no_channel_predicate_slot() {
        assert_eq!(CHANNEL_M, 8, "#148 不增通道谓词槽位");
        let frozen: [(ChannelId, ChannelDecision); 8] = [
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
            (ChannelId::Cj(1), ChannelDecision::Exit(ExitType::RiskExit)),
            (ChannelId::Cj(2), ChannelDecision::Exit(ExitType::CloseRoot)),
            (ChannelId::Cj(3), ChannelDecision::Exit(ExitType::ReduceCore)),
            (ChannelId::Cj(4), ChannelDecision::Exit(ExitType::CloseShortDiff)),
            (ChannelId::Cj(6), ChannelDecision::Open),
            (ChannelId::Cj(7), ChannelDecision::Record),
            (ChannelId::Cj(8), ChannelDecision::AddPosition),
        ];
        for (cid, want) in frozen {
            assert_eq!(decision_of(cid), want, "#147 T3 通道裁决映射被 #148 私改");
        }
    }

    // ── #148 T4：生产级联（#183 归一）与 subtree_close 的一致性见证 ─────────

    /// 生产级联 [`subtree_close_exit_decisions`]（held 槽压缩链投影 → subtree_close）与
    /// subtree_close 在树侧同一 deepest-first 序（触发者收尾）——#183 归一后生产「父关则子关」
    /// 直接经镜像函数发射的一致性见证（depth 槽线性链是 parent_id 树的线性特例）。
    #[test]
    fn t4_linear_chain_production_cascade_matches_subtree_close_deepest_first() {
        let (root, child, grand) = chain3();
        let active = vec![root, child, grand];
        let closed = subtree_close(&active, &[root]);
        // 树侧序：孙、子、根（代际降序）。
        let tree_generations: Vec<u32> = closed.iter().map(|l| 2 - l.level).collect();
        // 生产级联侧（held 槽压缩链，depth0 触发）：[2, 1, 0]。
        let held = vec![Some(held_at(0, false)), Some(held_at(1, false)), Some(held_at(2, false))];
        let out = subtree_close_exit_decisions(&held, 0, trigger_of(&held[0].unwrap(), 3), 3);
        let cascade_depths: Vec<u32> = out.iter().map(|d| d.depth).collect();
        assert_eq!(tree_generations, cascade_depths, "同一 deepest-first 序（线性特例一致）");
    }

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

    /// C1（最深优先级联，#183 归一到 subtree_close_exit_decisions）：held[0..3] 全活，
    /// depth 0 触发退出 ⟹ 产 3 决策，序 [depth2, depth1, depth0]。
    #[test]
    fn subtree_close_parent_exit_closes_descendants_deepest_first() {
        let held = vec![Some(held_at(0, false)), Some(held_at(1, false)), Some(held_at(2, false))];
        let trigger = trigger_of(&held[0].unwrap(), 7);
        let out = subtree_close_exit_decisions(&held, 0, trigger, 7);
        assert_eq!(out.len(), 3, "两个更深声部强制退出 + 触发决策收尾");
        assert_eq!(out[0].depth, 2, "最深优先（代际降序 ⟺ j 降序）");
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
    /// 级联发射（#183 归一）跳过已 pending 的更深槽（不重复产退出）。
    #[test]
    fn subtree_close_exit_pending_suppressed() {
        // 判据面：父 pending ⟹ 子的 parent_invalid 判据为真（父背景已否决）。
        let held_pending_parent = vec![
            Some(held_at(0, true)),  // 父已 pending（退出在延迟队列）
            Some(held_at(1, true)),  // 子已被上轮级联标 pending
            Some(held_at(2, false)), // 孙未触发
        ];
        assert!(parent_invalid_at(&held_pending_parent, 1), "父 exit_pending ⟹ 子父失效");
        // 发射面（生产调用契约：触发槽经外层循环过滤必非 pending）：已 pending 的 depth1
        // 不入投影链（跳过，不重复产退出），depth2 强制，触发者 depth0 收尾。
        let held = vec![
            Some(held_at(0, false)), // 触发者（生产循环已过滤 pending）
            Some(held_at(1, true)),  // 子已 pending ⟹ 跳过
            Some(held_at(2, false)), // 孙未触发
        ];
        let trigger = trigger_of(&held[0].unwrap(), 5);
        let out = subtree_close_exit_decisions(&held, 0, trigger, 5);
        assert_eq!(out.len(), 2, "pending 槽跳过：只孙 + 父");
        assert_eq!(out[0].depth, 2);
        assert_eq!(out[1].depth, 0);
    }

    /// C4（根回归锁）：仅 depth0 活 ⟹ 行为与现行逐字节同（parent_invalid=false 路径回归）。
    #[test]
    fn subtree_close_root_only_regression() {
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
        // 根触发 ⟹ 级联无更深声部 ⟹ 仅触发决策自身。
        let out = subtree_close_exit_decisions(&held, 0, trigger_of(&h, 3), 3);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].depth, 0);
    }
}
