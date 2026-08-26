//! 经济正条件逐信号分解（《经济正条件.pdf》第5节可捕获价差判据 L2 诊断）。
//!
//! 证明链 L0 见 `.chanlun/proofs/economic-positive-condition-chain.md`。本模块是其
//! **唯一可否证环节**（可捕获价差判据前件）的 L2 实装：逐信号确定性分解
//!
//! ```text
//! captured = Ab_rev − ηin − ηout − Ce/qe        (PDF p4-5 §5，对象=反转交易腿)
//! ```
//!
//! **664 号对象错配修复（codex 异质审计 diagnose 坐实，2026-06-30）**：缠论买卖点是
//! **反转交易**（底背驰买点 δ=+1 出现在下跌段末端，顶背驰卖点 δ=−1 出现在上涨段末端），
//! 交易方向 δ 与信号前触发/背驰段方向 ε **内在相反**。旧实装测「信号前触发段端点价差」
//! `Ab=εb(Pρb−Pλb)`（locate_lambda_bar 定位触发段起点）= **测错对象**：触发段几何方向反平行
//! 于 δ ⟹ δ·(Pρ−Pλ)<0 系统性产生 ΣAb<0（BTC L2 ΣAb=−1.8e4）。正确对象 = **post-signal
//! 反转交易腿**（信号确认后真正持有的那一段）。**禁止补丁**：改 eps 回 sign(Pρ−Pλ) 只恢复
//! Ab=|Pρ−Pλ|≥0 同义反复（命题S L0 恒真，零信息）= 声明膨胀（090）+ 掩盖错配（no-patch）。
//!
//! 反转交易腿分解（δ=+1 多 / δ=−1 空）：
//! - `Ab_rev = δ(P[ρ_rev] − P[λ_rev])`      反转交易腿理想价差（λ_rev=入场信号挂靠 pivot，
//!                                          ρ_rev=配对出场信号挂靠 pivot，端点 close）。
//! - `ηin    = max(0, δ(Pτin − P[λ_rev]))`  入场滞后损耗（确认 bar 成交价相对入场 pivot 的不利滑移，≥0）。
//! - `ηout   = max(0, δ(P[ρ_rev] − Pτout))` 出场损耗（实际出场价相对出场 pivot 的损耗，≥0）。
//! - `Ce/qe  = Pτ·fee_rate·2`               单位双边成本（与 `mu_estimator::marginal_return` 同口径）。
//!
//! **可否证（L2）**：若 Σ(ηin+ηout+Ce) ≥ ΣAb_rev（执行损耗吃光反转腿价差），则该信号集无可捕获 alpha，
//! 且分解定位「钱去哪了」（反转腿无价差 / 执行滞后 / 成本三者分离）。与 663 咬合：逐信号确定性分解，
//! 非统计功效检验。注意：与旧触发段实装不同，Ab_rev 在反转腿上可正可负——是测对了对象之后的真实价差，
//! 不是同义反复（命题S L0 恒真仅在顺笔/触发段成立）。
//!
//! **端点价缺口的严格解（不改 TradeRecord/Order）**：理想 pivot 端点价 P[λ_rev]/P[ρ_rev] 不在 fill
//! 配对链（`TradeRecord` 只有 entry/exit bar），但**信号收集路径**在确认点
//! 持有 BspPoint——其 `source_index`（bsp.rs:103，L0 原始 K 序的 pivot 端点位置）即信号挂靠的 pivot 端点
//! bar，取其 close 作 P[λ_rev]（入场信号）/P[ρ_rev]（配对出场信号）。故走 μ 路径无需透传 Order 端点价。
//!
//! **口径降级标注 `legacy_reverse_exit_diagnostic`（#137，G4 #134 移交处置）**：本模块的出场配对口径
//! = τ^reverse（下一个任意反向新确认信号出场，[`pair_signals`]）；入场口径 = 全部新确认 bsp 信号
//! （[`collect_signals`]，非生产 π 真开腿子集）。生产 μ 管线已在 G4（99bab5ad68）重接 typed exit
//! （`typed_ledger_from_bars`：P5/P6/P7 反向关腿/§13 结构剪枝/censored Hold）并整体删除 τ^reverse
//! 状态机——本模块**有意保留**旧口径而非重接：econ-663/664 谱系历史结论（可捕获价差分解、逐信号
//! μ̂ 诊断）的可比性依赖此口径，且 dx harness（`acc_classification_level_hole_dx`）的门诊断对象是
//! **信号集本身**（门前后计数/配对丢失），不是 fill 腿。这是诊断口径，不是生产出场口径；两口径的
//! μ̂ 不可直接互比（G4 commit：typed 口径样本量低约两个数量级）。旧注释中「同 `build_walk_forward_mu`」
//! 的同源声明自 G4 起失效，已随本标注移除。

use super::super::classifier::bsp::BspPoint;
use super::super::classifier::center::{center_from_segments, UnitRange};
use super::super::classifier::descend::RMove;
use super::super::classifier::divergence::compute_macd;
use super::super::classifier::nest::{is_sub, NestCertificate, NestInterval, NestRung};
use super::super::classifier::recursive_tower::LeveledMove;
use super::super::classifier::recursive_tower::{
    find_move_by_end_index, find_move_containing_index,
};
use super::super::classifier::signal::PanDivCert;
use super::super::config::ThetaConfig;
use super::super::strategy::coverage::Horizontal;
use super::super::strategy::interp::{assemble_gamma_with_tower, exit_type_of_classes, ExitType};
use super::super::strategy::voice::VoiceSide;
use super::super::types::{Bar, BspBits, Center, Side};
use super::data::Dataset;
use super::incremental::IncrementalClassifier;
use super::mu_estimator::{MuClass, PositionState};
use super::selector::{sigma_higher_at, z_of_candidate, ZExt};
use std::rc::Rc;

// ── #1175（A01）：五块职责拆分（纯移动零行为，消费面零改）──────────────────────────
// SpreadAttribution → `spread`；DescendLocator/DescendStop/下钻锚定 → `descend`；
// cand_delta 判据族 → `cand_delta`；XzdEvidence/小转大确认 → `xzd`；
// GatedPanDivCert/生产门 → `pan_div`（既有模块）。重导出保 `econ_positive::X`
// 原路径逐字不变；`decompose_capturable_spread` 入口与 gate/nest 判定留本文件门面。
mod cand_delta;
mod descend;
mod spread;
mod xzd;

pub use spread::SpreadAttribution;

pub(super) use super::pan_div::gate_pan_div_for_production;
pub(super) use cand_delta::BspCandType;
pub(super) use descend::DescendStop;
pub(super) use xzd::XzdEvidence;

// DescendLocator 仅被 mu_estimator/selector 的文档链（`super::econ_positive::DescendLocator`）
// 引用，仓内无代码消费者；保留 pub(super) 重导出以维持原可见性，故 allow unused_imports。
#[allow(unused_imports)]
pub(super) use descend::DescendLocator;

use cand_delta::{
    bsp_cand_type, cand_delta, cand_delta_base_gate, cand_delta_type2_completion,
    cand_delta_type3_retest,
};
use descend::descend_type1_anchor_depth;
use xzd::{
    l0_units_from_tower, xiaozhuanda_confirm, xzd_c3_new_center_breakout,
    xzd_c3_overlap_window_probe, xzd_force_exception, xzd_sub_last_zs_type3, xzd_type2_confirmed,
};

/// P7 正规出场口径：配对出场信号的缠论卖点（买点）类别（对齐 interp `ExitType` 的
/// CloseRoot/ReduceCore——#181 自 closed_loop/sell.rs SellDecision 收敛到 interp 单源）。
///
/// **定义依据**：出场信号 bsp_class bits 已编码卖点判据（EndpointSituation → endpoint_to_bsp）。
/// - `CloseRoot`：exit bits.sell1=true（第一类顶背驰，对应 interp `ExitType::CloseRoot`）。
/// - `ReduceCore`：exit bits.sell3=true ∧ sell1=false（第三类，interp `ExitType::ReduceCore`）。
/// - `Type2Missing`：exit bits.sell2=true（第二类卖点闭环 still-MISSING，诚实标注）。
/// - `Hold`：exit 是反向方向信号但无正规卖点 bits（bits 全0 或仅 buy/sell 位未命中正规类）。
///
/// 对空头入场（δ=−1），exit 是多头信号，用 buy1/buy3/buy2 镜像判据（买点类型对应买侧正规出场）。
///
/// **第二类闭环 still-MISSING**：第二类卖点闭环需次级别递归（见 descend.rs；原 closed_loop/sell.rs
/// §诚实边界同口径声明，#181 随 SellDecision 下线，本标注保留）。本枚举不臆造 Type2 实装
/// （no-patch）——遇 sell2/buy2 入 `Type2Missing`，记入统计但不改账本口径。
///
/// **账本边界**：本枚举只影响 econ 的 exit_decision 字段（分析统计口径），不触碰 TW 三阶段（GAP3/576）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitDecision {
    /// 第一类正规出场（顶背驰清仓）→ interp `ExitType::CloseRoot`。
    CloseRoot,
    /// 第三类正规出场（减核）→ interp `ExitType::ReduceCore`。
    ReduceCore,
    /// 第二类卖点闭环 still-MISSING（诚实边界，见枚举 docstring）。
    #[default]
    Type2Missing,
    /// 非正规卖点出场（exit bits 无明确正规类别，或 Hold 信号）。
    Hold,
}

/// 单信号的可捕获价差分解（PDF §5 五项 + captured = Ab−ηin−ηout−Ce/qe）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDecomp {
    /// 入场确认 bar（τin，F_i 可测）。
    pub entry_bar: usize,
    /// 出场 bar（τout，下一反向确认信号 / 末 bar censored）。
    pub exit_bar: usize,
    /// 触发买卖点所在 tower 级别（per-class 分桶键 (level,δ) 的 level 分量，MuClass.level 同口径）。
    pub level: u32,
    /// 方向 δ：+1 多 / −1 空（交易方向，反转交易腿；664 号 δ≠ε 笔方向）。
    pub delta: i8,
    /// Ab_rev = δ(P[ρ_rev] − P[λ_rev])：反转交易腿理想价差（λ_rev=入场信号 pivot，ρ_rev=出场信号 pivot；
    /// 664 号修复测错对象——非触发段价差，可正可负）。
    pub a_b: f64,
    /// x = δ(Pτin − P[λ_rev])：**signed** 入场滑移（664-Q3 codex）。x<0=有利（确认价比 pivot 更优），x>0=不利。
    /// ηin=max(0,x) 丢弃了 x<0 的有利滑移 ⟹ captured 系统性低估真实 PnL（min(x,0)≤0）。
    pub x_in: f64,
    /// y = δ(P[ρ_rev] − Pτout)：**signed** 出场滑移（664-Q3 codex）。y<0=有利，y>0=不利。
    pub y_out: f64,
    /// ηin = max(0, x_in)：入场滞后损耗（adverse-only，≥0）。
    pub eta_in: f64,
    /// ηout = max(0, y_out)：出场损耗（adverse-only，≥0）。
    pub eta_out: f64,
    /// actual_spread = δ(Pτout − Pτin) = Ab_rev − x_in − y_out：**真实成交价差**（无成本，664-Q3）。
    /// 两个确认 bar（实际成交点）close 的有向差——含有利+不利滑移全部，非 adverse-only。
    pub actual_spread: f64,
    /// Ce/qe：单位双边成本。
    pub ce_unit: f64,
    /// captured = Ab_rev − ηin − ηout − Ce/qe（adverse-only 保守压力测试，PDF §5；丢有利滑移）。
    pub captured: f64,
    /// actual_pnl = actual_spread − Ce/qe = δ(Pτout − Pτin) − Ce：**真实成交 PnL 代理**（664-Q3，含全部滑移）。
    pub actual_pnl: f64,
    /// σ_higher：入场确认时**上级方向态**（tower_i[lvl+1] 末走势端点价符号派生，666 号 σ_higher 实验）。
    /// +1=上级走势净涨（Trend Up 等价）/ −1=净跌 / 0=持平或无上级层。
    /// **可达性约束（不是补丁）**：上级层走势是 `RMove::Compose`（descend.rs:53），**无 direction 字段**。
    /// task #143 后 `LevelState.moves: Vec<MoveBlock>` 已携块方向（decompose.rs），但本实验冻结于端点
    /// 净差口径（666 号已跑数），且信号收集作用域读的是 tower（非 LevelState）。故保留取上级
    /// LeveledMove 的 `start_index/end_index`（L0 原始 K 序端点，覆盖该走势全跨度）close 净差符号——
    /// 与 Trend(Up)⟺端点净涨语义等价，是该作用域的严格可达解。用于分离「δ 顺上级 vs 逆上级」alpha 归因。
    pub sigma_higher: i8,
    /// bsp_class：买卖点类型位掩码（W4 类型透传）。bsp_disc(&p.bits) 同口径——
    /// bit0=buy1, bit1=buy2, bit2=buy3, bit3=sell1, bit4=sell2, bit5=sell3（可同时多位）。
    /// 完整保留一/二/三类+买卖侧信息，供类型分层 alpha 检验（避免混合池稀释掩盖单类型 alpha）。
    /// 买卖侧另见 `delta`（+1 买 / −1 空，交易方向）——本字段是 bit 级类型，delta 是聚合方向。
    pub bsp_class: u8,
    /// P7 正规出场口径：配对出场信号的缠论卖（买）点类别（接 interp `ExitType` CloseRoot/ReduceCore）。
    /// 从配对出场信号的 bsp_class 派生（bsp_class bits 已编码卖点判据结果）。
    /// `Type2Missing` = 第二类闭环 still-MISSING（见 [`ExitDecision`] docstring），诚实标注，不改账本口径。
    pub exit_decision: ExitDecision,
    /// z：完整 [`MuClass`] 全互斥分类键（b2 升 Z 分桶；含 level/δ/i_class/parent_dir/short_swing/position）。
    /// 分桶/dx 投影**一律用 `z.i_class`**（未压缩 6-bit），**禁止 `z.bsp_class()`**（会丢 2B/3B 重合，
    /// 违反 P4 codex 判决）。`bsp_class` 旧字段保留供旧口径对照（CSV/旧报告），不作分桶键。
    pub z: MuClass,
    /// P0-1：准入触发通道（[`NestTrigger`]，codex-f2 #1）——signal-provenance，供每桶 μ̂ 质量归因。
    pub(super) trigger: NestTrigger,
}

/// 买卖点身份判别 u8（seen-set diff 键，与 l3_delta_r_alpha::bsp_disc 同口径）。
fn bsp_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// 逐信号可捕获价差分解（L2）。信号收集 + τ^reverse 退出配对（`legacy_reverse_exit_diagnostic`，
/// 见模块头——G4 99bab5ad68 后生产 μ 已改 typed exit，本函数保留旧口径作 econ-663/664 诊断），
/// 取入场信号与配对出场信号的 pivot 端点（source_index）算 PDF §5 反转交易腿分解（664 号）。
///
/// 返回 `(Vec<SignalDecomp>, SpreadAttribution)`：逐信号分解 + 聚合归因。
///
/// **认识论 L2**：真实数据逐信号分解，可产否定性结果（spread_eaten=true ⟹ 信号集无 alpha）。
pub fn decompose_capturable_spread(
    data: &Dataset,
    config: &ThetaConfig,
) -> (Vec<SignalDecomp>, SpreadAttribution) {
    // C1（algo-opt-plan-20260702 泳道 C）：拆 collect_signals（O(bar²) 逐 bar 收集）+ pair_signals
    // （O(信号) 配对）。语义 bit-exact 旧实装——collect 再 pair 顺序调用 = 原单函数体，逐字未改。
    // 拆分动机：让 acc_classification_level_hole_dx 复用 collect 输出，省二次全量重收集（dx harness ~2x）。
    let tick = config.tick.tick_size;
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let signals = collect_signals(data, config);
    pair_signals(&signals, &data.bars, tick, fee_rate)
}

/// 一条收集信号（b2：原 7 元组升具名 struct——7 字段两处生产者需 bit-exact 同步，具名字段消除位置错配）。
///
/// `z` 携带完整 [`MuClass`]（升 Z 分桶键）；`bsp_class` 旧字段保留供旧口径对照。生产 [`collect_signals`]
/// 与诊断 dx 手写循环各产一份，`acc_classification_level_hole_dx` 尾部 `assert_eq!` 逐条对拍（含 z）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct RawSignal {
    /// 入场确认 bar τin。
    entry_bar: usize,
    /// 交易方向候选 δ_g（VoiceSide）。
    dir: VoiceSide,
    /// 信号挂靠 pivot 端点 source_index（λ_rev/ρ_rev 取价处）。
    pivot_bar: usize,
    /// 级别 ℓ。
    level: u32,
    /// 入场时上级方向态（666 号）。
    sigma_higher: i8,
    /// bsp_class：bsp_disc(&p.bits) 类型位掩码（W4，旧口径对照，非分桶键）。
    bsp_class: u8,
    /// z：完整 MuClass 全互斥分类键（b2）。用 z.i_class 分桶，非 z.bsp_class()。
    z: MuClass,
    /// P0-1：准入触发通道（[`NestTrigger`]，codex-f2 #1）——signal-provenance，供每桶 μ̂ 质量归因。
    trigger: NestTrigger,
}

/// decompose 收集半边（纯函数）：逐 bar 因果分类 + N^δ 多级门 → 信号集。
///
/// C1 从 [`decompose_capturable_spread`] 拆出；`acc_classification_level_hole_dx` 复用本函数输出对拍。
fn collect_signals(data: &Dataset, config: &ThetaConfig) -> Vec<RawSignal> {
    let bars = &data.bars;
    let n = bars.len();
    let tick = config.tick.tick_size;

    // ── MACD hist 预计算（DivCand 条件4 Weak 判据所需，W1 工位）。 ──
    // Θ_MACD：全序列一次性计算（O(n)），供 bsp_div_cand 查询 bar 区间面积。
    // ponytail: 预计算一次，信号收集循环 O(1) 查表，无 per-signal 重算。
    let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
    let macd_hist = compute_macd(&closes, &config.macd).hist;

    // ── 信号收集（legacy_reverse_exit_diagnostic 收集半边，见模块头；G4 前与旧 build_walk_forward_mu
    // 同源，G4 后生产入场已收敛为 π 真开腿子集）：逐 bar 因果分类，收全部新确认买卖点。 ──
    // 664 号：每条信号挂靠的 pivot 端点 = p.source_index（bsp.rs:103，L0 原始 K 序）。
    // 反转交易腿 λ_rev = 入场信号 pivot 端点；ρ_rev 在退出配对时取配对出场信号的 pivot 端点。
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // Q4：盘整背驰证书去重键 (lvl, source_index, side判别码)——证书跨 bar 复现，首见 bar 即 entry_bar。
    let mut seen_pan: std::collections::HashSet<(usize, usize, u8)> =
        std::collections::HashSet::new();
    // 每条信号：(entry_bar=确认 bar τin, dir=交易方向 δ, pivot_bar=信号挂靠 pivot 端点 source_index,
    // lvl=级别, sigma_higher=入场时上级方向态 666 号, bsp_class=bsp_disc(&p.bits) 类型位掩码 W4)。
    // W4 类型透传：bsp_class 是 buy1/2/3+sell1/2/3 的 u8 位掩码（bsp_disc 同口径，seen-set 键已在用），
    // 完整保留一/二/三类+买卖侧信息（可同时置多位，如 buy1+buy3）。纯增字段，不改配对/信号集/识别逻辑。
    let mut signals: Vec<RawSignal> = Vec::new();
    // C2（algo-opt-plan-20260702 泳道 C）：各级 bsp 上一 bar 的 Rc 强引用。`Rc::ptr_eq` 命中 ⟹ 同一
    // allocation 且内容未变（强引用在手 ⟹ strong_count>1 ⟹ classifier 端任何 `Rc::make_mut` 必写时
    // 复制换新指针，见 mod.rs cascade_reset/07c memo）⟹ 该级全部 (lvl, source_index, bsp_class) 键
    // 上一 bar 已入 seen ⟹ 整级跳过 bit-exact（内层循环 seen 命中外无副作用）。禁裸指针：强引用保证
    // allocation 不释放、地址不复用，无 ABA 假命中。塔收缩后槽位残留旧 Rc 无害——级别重现且 memo 复用
    // 同一 Rc 时命中仍蕴含"已 seen"（ptr_eq 不变量与级别连续性无关）。
    let mut prev_bsp = Vec::new();

    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        // ★#883：改取 `classify_at_with_l0`——l0.strokes（L0 笔序列，source_index 域，因果前缀）
        // 供 div_cand 的 ForceL 教义力度判据（#873/#989/#990）；classify_at 本就走同一增量链、
        // 丢弃 l0，成本相同。
        let __wl0 = classifier_incr.classify_at(i);
        let l0_i = classifier_incr
            .last_l0()
            .expect("classify_at 已推进 ParseLayer");
        let cls_i = __wl0.classification;
        let tower_i = __wl0.tower;
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            if prev_bsp
                .get(lvl)
                .map_or(false, |prev| Rc::ptr_eq(prev, &ls.bsp))
            {
                continue; // C2：同 Rc ⟹ 全键已 seen，跳级
            }
            if lvl < prev_bsp.len() {
                prev_bsp[lvl] = Rc::clone(&ls.bsp);
            } else {
                prev_bsp.push(Rc::clone(&ls.bsp)); // enumerate 连续 ⟹ 恰在 lvl==len 时到达
            }
            for p in ls.bsp.iter() {
                let bsp_class = bsp_disc(&p.bits); // W4：类型位掩码（seen-set 键复用，纯透传）
                if !seen.insert((lvl, p.source_index, bsp_class)) {
                    continue; // 已确认过
                }
                let pivot_bar = p.source_index; // 信号挂靠 pivot 端点（bsp.rs:103）= λ_rev / ρ_rev 取价处
                                                // dir 经 assemble_gamma 拿（构造仅含该点的单级别分类；G4 前与旧 build_walk_forward_mu 同源）。
                                                // ponytail（fullhist-oom-fix-20260630）: single 的空 moves/centers 不是冗余——它**屏蔽其他层
                                                // bsp**，只让这一个 bsp 产单候选。elements 从 tower 提取，classification 只供 bsp 做 Γ 组装；
                                                // dir 来自该 bsp 的 coverage role 判定，非 tower 裸结构可读。此重建仅每新信号触发（600K bar=1682
                                                // 次，万级噪声非 O(bar) 主导），勿当 O(N×L) 去重写 coverage 核心——会破 bit-exact。O(n²) 真因在
                                                // classify_at 每 bar O(tree) 续算（#104/#105/#106 generation 快路）。
                let single = super::super::classifier::Classification {
                    levels: cls_i
                        .levels
                        .iter()
                        .enumerate()
                        .map(|(l2, _)| super::super::classifier::LevelState {
                            moves: Vec::new(),
                            centers: Rc::new(Vec::new()),
                            cp_ownership: Rc::new(Vec::new()),
                            bsp: Rc::new(if l2 == lvl {
                                vec![p.clone()]
                            } else {
                                Vec::new()
                            }),
                            pan_div: Rc::new(Vec::new()), // Q4：single 屏蔽层无盘整背驰载荷（只供 Γ 组装）
                            first_class_grades: Rc::new(Vec::new()), // #885：同 pan_div 口径（single 屏蔽层无分级记录载荷）
                            level_projection: None, // #110 门关口径（与生产 stamping 关闭分支同形）
                        })
                        .collect(),
                };
                let sigma_higher = sigma_higher_at(&tower_i, bars, lvl); // 666 号：入场时上级方向态
                for c in &assemble_gamma_with_tower(&single, &tower_i) {
                    if c.dir == VoiceSide::Flat {
                        continue;
                    }
                    // W1 返工：N^δ 多级区间套准入门（替换 bsp_div_cand 单级门）。
                    // 调 build_multilevel_nest_cert：从塔构造 rungs 链，调 NestCertificate::n_delta()。
                    // N^δ=false ⟹ 跳过（合法定位失败；任一级 Cand=0 或区间不收缩均拒）。
                    // 认识论 L0（结构过滤，不声明 alpha；alpha 有效性待 W-VERIFY L2/L3）。
                    // **有效域诚实（codex #39 + task#22 实测）**：接通 ≠ 多级执行。执行级 e=lvl，
                    // rungs 从 tower[lvl+1..] 收集上级语境。**BTC 300K 实测 95.36% 通过门信号 rungs 空
                    // ⟹ n_delta 退化为 base-case Conf^δ_e（单 bit 方向确认），仅 4.64% 真跨级 J 嵌套
                    // （max 深度=1）**。有效深度分布见 acc_classification_level_hole_dx（effective_nest_depth）。
                    let delta_side = match c.dir {
                        VoiceSide::Long => Side::Long,
                        VoiceSide::Short => Side::Short,
                        VoiceSide::Flat => continue,
                    };
                    // 二通道准入门（小转大 landing）：区间套 Nest→n_delta；小转大 Xzd→gate_pass
                    // （level==1 时 C2∧C3(新中枢+突破) 硬门，level!=1 维持 C2-only，codex #44 终局裁定(c)，
                    // C2+C3(breakout) xzd）。次级别（lvl-1）中枢/bsp/走势供 C3 判据；小转大域 lvl≥1，
                    // lvl==0 走区间套不消费此三值。
                    let sub_centers: &[Center] = if lvl > 0 {
                        &cls_i.levels[lvl - 1].centers
                    } else {
                        &[]
                    };
                    let sub_bsp: &[BspPoint] = if lvl > 0 {
                        &cls_i.levels[lvl - 1].bsp
                    } else {
                        &[]
                    };
                    let gate_cert = build_gate_certificate(
                        &tower_i,
                        lvl,
                        p.source_index,
                        delta_side,
                        &p.bits,
                        &macd_hist,
                        i,
                        &ls.bsp,
                        sub_centers,
                        sub_bsp,
                        // ★#883：L0 笔序列（因果前缀）供 ForceL；力度档随 config（默认 ForceL，
                        // MacdArea 为显式对照档，#990）。
                        &l0_i.strokes,
                        config.divergence_gauge,
                    );
                    let pass = match &gate_cert {
                        Some(GateCertificate::Nest(cert)) => cert.n_delta(),
                        Some(GateCertificate::Xzd(ev)) => ev.gate_pass(),
                        None => false,
                    };
                    if !pass {
                        continue;
                    }
                    // P0-1：准入触发通道（signal-provenance）——gate 已 pass ⟹ gate_cert 必 Some。
                    let trigger = nest_trigger(
                        gate_cert.as_ref().expect("pass ⟹ gate_cert Some"),
                        bsp_cand_type(&p.bits, delta_side),
                    );
                    // b2（task #83）：升 Z 分桶——从候选构造完整 z（含 σ_p/role/H/σ_higher），非事后从
                    // (level,δ,bsp_class) 粗投影重推。z_of_candidate 复用 selector 既有 role→z 桥
                    // （codex #81 修正1：信号携带 z:MuClass，不再扩位置易错的裸元组）。
                    // ★force_state 生产热路由（beta-route #115 → A6 #159）：一类候选带 p.force（A/C 段
                    // 5 proxy），经 Candidate.force 透传（assemble_gamma 系纯透传 c.force==p.force），
                    // z_of_candidate 调 ForceProxies::force_state() 填 force_state 第 8 维（δ-free
                    // 支配序，perm_test 已按 c.force_state 分桶；二/三类 p.force=None ⟹ force_state=None）。
                    // ★σ_higher 第 9 维（G2 #132）：塔真值经 z_of_candidate 内 sigma_higher_at 填。
                    // ★G3 第 10-12 维（#138）：从已 pass 的 gate_cert 装配——cand_channel=trigger（P0-1
                    // 同源）；Nest 通道 nest_depth=rungs.len()（0=基例真值）、origin_level=lvl+depth
                    // （链顶 ℓ，「执行级 e=lvl，rungs 收集上级语境」的有效域口径）；Xzd 通道无 rungs
                    // 概念 ⟹ depth=None、origin_level 走 z_of_candidate 默认 Some(c.level)（起始=执行）。
                    // risk_mode=None：统计层信号收集无账本（equity/持仓），诚实 None（第 13 维在
                    // runner π fill loop 生态填真值）。
                    let ext = match gate_cert.as_ref().expect("pass ⟹ gate_cert Some") {
                        GateCertificate::Nest(cert) => ZExt {
                            cand_channel: Some(trigger),
                            nest_depth: Some(cert.rungs().len() as u8),
                            origin_level: Some(lvl as u32 + cert.rungs().len() as u32),
                            risk_mode: None,
                            t_stage: None, // #149：统计层无 TW 账本，同 risk_mode 诚实 None
                            eta_bucket: None, // #175：统计层无 TW 账本，同 t_stage 诚实 None
                        },
                        GateCertificate::Xzd(_) => ZExt {
                            cand_channel: Some(trigger),
                            nest_depth: None,
                            origin_level: None, // ⟹ z 填 Some(c.level)（起始=执行真值）
                            risk_mode: None,
                            t_stage: None, // #149：统计层无 TW 账本，同 risk_mode 诚实 None
                            eta_bucket: None, // #175：统计层无 TW 账本，同 t_stage 诚实 None
                        },
                    };
                    let z = z_of_candidate(c, &tower_i, bars, &ext);
                    // G2 一致性护栏：z 内 σ_higher（按 c.level 取）须与信号分解口径（按 lvl 取）同值
                    // ——同函数同塔，仅 level 来源不同（c.level 由 assemble 自 lvl 单级分类产生）。
                    debug_assert_eq!(
                        z.sigma_higher,
                        Some(sigma_higher),
                        "z.sigma_higher 与 SignalDecomp 口径分叉"
                    );
                    signals.push(RawSignal {
                        entry_bar: i,
                        dir: c.dir,
                        pivot_bar,
                        level: lvl as u32,
                        sigma_higher,
                        bsp_class,
                        z,
                        trigger,
                    });
                }
            }

            // ── Q4 盘整背驰承接（task #145）：消费该级 pan_div 证书，走 Nest/XZD 二通道门。──
            // 证书不携 six-bit（不冒充 B1/S1）；任一通道通过 ⟹ 组 RawSignal 入信号流
            // （trigger=PanDivConsolidation，供 μ̂ 归因分桶）；两门皆闭 ⟹ 诚实丢弃。
            // 既有 Nest/Xzd 信号判定路径零改动（PanDiv 是新增入口，既有信号集 bit 不变）。
            for cert in ls.pan_div.iter() {
                let side_disc = match cert.side {
                    Side::Long => 0u8,
                    Side::Short => 1u8,
                };
                if !seen_pan.insert((lvl, cert.source_index, side_disc)) {
                    // 首见即终局（冻结约定，与 bsp 通道 seen.insert 同时序——:316 先例）：承接门
                    // 在证书首见 bar 用当时可得证据评一次，两门皆闭 ⟹ 永久丢弃不重试。codex
                    // ac4-r2 #2 显式化：这是全通道统一的 τin 语义（信号在确认 bar 评定），非
                    // PanDiv 特例；若裁决改为「证据出现即承接」须全通道同改。
                    continue;
                }
                let sub_centers: &[Center] = if lvl > 0 {
                    &cls_i.levels[lvl - 1].centers
                } else {
                    &[]
                };
                let sub_bsp: &[BspPoint] = if lvl > 0 {
                    &cls_i.levels[lvl - 1].bsp
                } else {
                    &[]
                };
                if !pan_div_gate_pass(
                    &tower_i,
                    lvl,
                    cert,
                    &macd_hist,
                    i,
                    &ls.bsp,
                    sub_centers,
                    sub_bsp,
                    &l0_i.strokes, // ★#883
                    config.divergence_gauge,
                ) {
                    continue; // 两门皆闭 ⟹ 承接失败诚实丢弃（不入信号）。
                }
                let (dir, delta_i8): (VoiceSide, i8) = match cert.side {
                    Side::Long => (VoiceSide::Long, 1),
                    Side::Short => (VoiceSide::Short, -1),
                };
                let sigma_higher = sigma_higher_at(&tower_i, bars, lvl);
                // z：裸证书口径 + 真值维——PanDiv 无 BspPoint/coverage role（不经 assemble_gamma），
                // i_class=0（零 bit，StructBreak 零类先例）、parent_dir=0/Root（无声部父）、
                // horizontal/force_state 诚实 None（from_certificate 口径）；sigma_higher 塔真值、
                // cand_channel=PanDivConsolidation、origin_level=Some(lvl)（起始=执行）、
                // nest_depth=None（向上 rungs 深度概念不适用，同 Xzd 口径）。
                let z = MuClass {
                    sigma_higher: Some(sigma_higher),
                    cand_channel: Some(NestTrigger::PanDivConsolidation),
                    origin_level: Some(lvl as u32),
                    ..MuClass::from_certificate(
                        lvl as u32,
                        delta_i8,
                        BspBits::default(),
                        0,
                        PositionState::Root,
                    )
                };
                signals.push(RawSignal {
                    entry_bar: i,
                    dir,
                    pivot_bar: cert.source_index,
                    level: lvl as u32,
                    sigma_higher,
                    bsp_class: 0, // 零 bit（盘整背驰非买卖点，不置 six-bit——诚实零类）。
                    z,
                    trigger: NestTrigger::PanDivConsolidation,
                });
            }
        }
    }

    signals
}

/// decompose 配对半边（纯函数，O(信号)）：signals → (decomps, agg)，无分类无 IO。
///
/// C1 从 [`decompose_capturable_spread`] 拆出；`acc_classification_level_hole_dx` 尾部直接调用，
/// 复用其手写循环自建的 `signals`（与生产 [`collect_signals`] 同序同门），省二次 O(bar²) 收集。
fn pair_signals(
    signals: &[RawSignal],
    bars: &[Bar],
    tick: f64,
    fee_rate: f64,
) -> (Vec<SignalDecomp>, SpreadAttribution) {
    // ── 退出配对（664 号反转交易腿）：持有到下一反向新确认信号 = τ^reverse 口径（PDF §9 点名废弃于
    // 生产 μ，G4 99bab5ad68 已删；此处 legacy_reverse_exit_diagnostic 保留，见模块头），
    // ρ_rev = 该配对出场信号的 pivot 端点。 ──
    // 与旧实装的关键差异：ρ_rev 是 **post-signal 且策略 owned**（配对出场信号挂靠 pivot），
    // 不是触发段起点（错对象）。无配对出场信号 ⟹ 诚实跳过（末 bar 不是 pivot 端点，无 ρ_rev，不兜底）。
    let mut decomps: Vec<SignalDecomp> = Vec::new();
    let mut agg = SpreadAttribution::default();

    // ★O(信号²)→O(信号) 配对预处理：next_opp[side][i] = 从位置 i 起第一个该 side 信号的 idx
    // （无则 signals.len()）。signals 按 entry_bar 单调非降（逐 bar 收集）⟹ 从 i 起第一个 opp 方向
    // 信号在 next 表 O(1) 查；配对须 eb>entry_bar（排同 bar），同 bar opp 聚在 entry_bar 批内且排在
    // eb>entry_bar 的 opp 之前，故 while 跳同 bar 平摊 O(1)（同 bar 信号数有限）。bit-exact == 旧
    // signals[idx+1..].find/any（同一"首个后续 opp"语义，仅从线性扫换 O(1) 表查）。
    // ponytail: O(信号) 预处理替代每信号 O(信号) 扫，全历史 12626 信号 O(n²)→O(n)。
    let ns = signals.len();
    let mut next_long = vec![ns; ns + 1];
    let mut next_short = vec![ns; ns + 1];
    for i in (0..ns).rev() {
        next_long[i] = if signals[i].dir == VoiceSide::Long {
            i
        } else {
            next_long[i + 1]
        };
        next_short[i] = if signals[i].dir == VoiceSide::Short {
            i
        } else {
            next_short[i + 1]
        };
    }

    for (idx, s) in signals.iter().enumerate() {
        let RawSignal {
            entry_bar,
            dir,
            pivot_bar: lambda_rev_bar,
            level,
            sigma_higher,
            bsp_class,
            z,
            trigger,
        } = *s;
        let delta: i8 = match dir {
            VoiceSide::Long => 1,
            VoiceSide::Short => -1,
            VoiceSide::Flat => continue,
        };
        let opp = if delta == 1 {
            VoiceSide::Short
        } else {
            VoiceSide::Long
        };
        let next_opp = if opp == VoiceSide::Long {
            &next_long
        } else {
            &next_short
        };
        // 首个后续 opp 方向信号（O(1) 表查，替代 signals[idx+1..].find/any 的 O(信号)扫）。
        let mut j = next_opp[idx + 1];
        // 三审计统计②（codex Q2）：同 bar 反向信号（首个 opp 若 eb==entry_bar 即命中——单调非降 ⟹
        // 同 bar opp 排在 eb>entry_bar opp 之前）。
        if j < ns && signals[j].entry_bar == entry_bar {
            agg.n_same_bar_opposite += 1;
        }
        // 配对须 eb>entry_bar：跳过同 bar opp（平摊 O(1)，同 bar 信号有限）。
        while j < ns && signals[j].entry_bar <= entry_bar {
            j = next_opp[j + 1];
        }
        // 配对出场信号（首个 eb>entry_bar 反向新确认信号，π^bsp owned）：entry_bar(τout) + pivot_bar(ρ_rev) + exit_bsp_class(P7)。
        let (exit_bar, rho_rev_bar, exit_bsp_class) = if j < ns {
            (
                signals[j].entry_bar,
                signals[j].pivot_bar,
                signals[j].bsp_class,
            )
        } else {
            agg.n_unpaired += 1; // 三审计统计③（codex Q4）：右删失，无配对出场反转信号，诚实跳过不兜底
            continue;
        };
        if exit_bar <= entry_bar {
            continue;
        }
        // 端点价（close 口径，与回测成交价同口径）：
        // P[λ_rev]=入场信号 pivot 端点 close（理想入场），P[ρ_rev]=配对出场信号 pivot 端点 close（理想出场），
        // Pτin=入场确认 bar close（实际入场，含确认滞后），Pτout=出场确认 bar close（实际出场）。
        let p_lambda = px_at(bars, lambda_rev_bar, tick);
        let p_rho = px_at(bars, rho_rev_bar, tick);
        let p_tau_in = px_at(bars, entry_bar, tick);
        let p_tau_out = px_at(bars, exit_bar, tick);
        if [p_lambda, p_rho, p_tau_in, p_tau_out]
            .iter()
            .any(|&x| x <= 0.0)
        {
            continue;
        }
        let eps = delta as f64; // 交易方向 δ（反转交易腿；664 号 δ≠ε 笔方向）
        let a_b = eps * (p_rho - p_lambda); // Ab_rev=δ(P[ρ_rev]−P[λ_rev])，可正可负（测对了对象）
        let x_in = eps * (p_tau_in - p_lambda); // signed 入场滑移（664-Q3）：<0 有利 / >0 不利
        let y_out = eps * (p_rho - p_tau_out); // signed 出场滑移（664-Q3）
        let eta_in = x_in.max(0.0); // adverse-only（丢 x<0 有利滑移）
        let eta_out = y_out.max(0.0);
        let actual_spread = eps * (p_tau_out - p_tau_in); // = a_b − x_in − y_out（真实成交价差，含全部滑移）
                                                          // 单位双边成本：与 marginal_return 口径一致（fee 在 entry/exit 各扣一次）。
        let ce_unit = (p_tau_in + p_tau_out) * fee_rate;
        let captured = a_b - eta_in - eta_out - ce_unit; // adverse-only 保守压力测试
        let actual_pnl = actual_spread - ce_unit; // 真实成交 PnL 代理（664-Q3）
                                                  // P7 正规出场口径：从配对出场信号 bsp_class 派生（接 interp ExitType CloseRoot/ReduceCore）。
        let exit_decision = exit_decision_from_bits(exit_bsp_class, delta);

        decomps.push(SignalDecomp {
            entry_bar,
            exit_bar,
            level,
            delta,
            a_b,
            x_in,
            y_out,
            eta_in,
            eta_out,
            actual_spread,
            ce_unit,
            captured,
            actual_pnl,
            sigma_higher,
            bsp_class,
            z, // b2：入场信号完整 z（升 Z 分桶键；entry 信号的 MuClass 携带 σ_p/role/H）
            exit_decision,
            trigger, // P0-1：入场信号准入触发通道（每桶 μ̂ 归因）
        });
        agg.n_signals += 1;
        agg.sum_a_b += a_b;
        agg.sum_x_in += x_in;
        agg.sum_y_out += y_out;
        agg.sum_eta_in += eta_in;
        agg.sum_eta_out += eta_out;
        agg.sum_actual_spread += actual_spread;
        agg.sum_ce += ce_unit;
        agg.sum_captured += captured;
        if captured > 0.0 {
            agg.n_captured_positive += 1;
        }
        if actual_pnl > 0.0 {
            agg.n_actual_positive += 1;
        }
        if rho_rev_bar > lambda_rev_bar {
            agg.n_rho_after_lambda += 1; // 三审计统计①（codex Q1 不变量）
        }
        // P7 出场决策统计（接 interp ExitType CloseRoot/ReduceCore，Type2Missing=still-MISSING 诚实标注）。
        match exit_decision {
            ExitDecision::CloseRoot => agg.n_exit_close_root += 1,
            ExitDecision::ReduceCore => agg.n_exit_reduce_core += 1,
            ExitDecision::Type2Missing => agg.n_exit_type2_missing += 1,
            ExitDecision::Hold => agg.n_exit_hold += 1,
        }
    }

    (decomps, agg)
}

/// P7 正规出场口径：配对出场信号 bsp_class bits → ExitDecision，**委托 interp 平仓出场权威**。
///
/// **接线（非新逻辑）**：type1>type3 优先级 + CloseRoot/ReduceCore 语义由
/// [`interp::exit_type_of_classes`](super::super::strategy::interp::exit_type_of_classes) 单一决定
/// （#181 自 `closed_loop/sell.rs::sell_decision_of` 迁入——SellDecision 死路径下线，语义统一
/// 收敛到 interp `ExitType` 单源），本函数只做「bsp bits → (is_type1, is_type3, is_type2) 判据」
/// 的解包与方向选择，再把 [`ExitType`] 提升为 [`ExitDecision`]（加诊断态 Type2Missing）。
/// 优先级不在此重编码（no-patch）。
///
/// **定义依据**：
/// - 多头入场（δ=+1），exit 是 Short 信号：看卖侧 bits（bit3=sell1, bit4=sell2, bit5=sell3）。
/// - 空头入场（δ=−1），exit 是 Long 信号：看买侧 bits（bit0=buy1, bit1=buy2, bit2=buy3，买点镜像卖点）。
/// - `exit_type_of_classes` 返回 `CloseRoot`（type1 命中，优先）/ `ReduceCore`（type3 命中）/ `Hold`。
/// - `Hold` 且命中 type2 → `Type2Missing`（第二类闭环 still-MISSING 诚实标注，见 [`ExitDecision`]）；否则 `Hold`。
///
/// **边界条件**：若出场方向无 type1/type2/type3 bit（含 bsp_class=0）→ Hold（无正规出场依据）。
/// **账本边界**：不触碰 TW 三阶段（GAP3/576 still-MISSING，econ 只用 R 账本 closed_loop 对齐）。
/// **认识论 L0**：纯 bit 解包 + interp 权威映射，不声明 alpha（alpha 待 W-VERIFY L2/L3）。
pub(super) fn exit_decision_from_bits(exit_bsp_class: u8, delta: i8) -> ExitDecision {
    // bsp_class 位掩码：bit0=buy1, bit1=buy2, bit2=buy3, bit3=sell1, bit4=sell2, bit5=sell3
    const SELL1: u8 = 1 << 3;
    const SELL2: u8 = 1 << 4;
    const SELL3: u8 = 1 << 5;
    const BUY1: u8 = 1 << 0;
    const BUY2: u8 = 1 << 1;
    const BUY3: u8 = 1 << 2;
    // 出场方向选 bits：多头出场看卖侧，空头出场看买侧（买点镜像卖点，同一平仓优先级）。
    let (t1, t2, t3) = if delta > 0 {
        (SELL1, SELL2, SELL3)
    } else {
        (BUY1, BUY2, BUY3)
    };
    let is_type1 = exit_bsp_class & t1 != 0;
    let is_type2 = exit_bsp_class & t2 != 0;
    let is_type3 = exit_bsp_class & t3 != 0;
    // interp 权威：type1>type3 优先级 + 平仓语义单一来源（#181 自 closed_loop::sell 迁入）。
    match exit_type_of_classes(is_type1, is_type3) {
        ExitType::CloseRoot => ExitDecision::CloseRoot,
        ExitType::ReduceCore => ExitDecision::ReduceCore,
        // Hold（无 type1/type3）：命中 type2 则诚实标注闭环缺口，否则真 Hold。
        ExitType::Hold if is_type2 => ExitDecision::Type2Missing,
        ExitType::Hold => ExitDecision::Hold,
        // exit_type_of_classes 只产 CloseRoot/ReduceCore/Hold——CloseReverseOpen/RiskExit 由
        // entry_v/risk 通道产出（reverse_exit_type / 风控门），bits 入口不可达。
        ExitType::CloseReverseOpen | ExitType::RiskExit => {
            unreachable!("exit_type_of_classes 只产 CloseRoot/ReduceCore/Hold")
        }
    }
}

/// bar close → 价格（close×tick），越界/非正返 0。
fn px_at(bars: &[Bar], i: usize, tick: f64) -> f64 {
    bars.get(i)
        .map(|b| b.close as f64 * tick)
        .filter(|&p| p > 0.0)
        .unwrap_or(0.0)
}

/// W1 返工：多级 N^δ 区间套证书构造 + 真递归调用（替换 bsp_div_cand 单级门）。
///
/// ## 结果包（六要素）
/// - **结论**：从塔构造 `NestCertificate`，调用 `n_delta()`，返回 N^δ_{ℓ↓e} ∈ {0,1}。
/// - **定义依据**：spec P5 §6 `N^δ_{ℓ↓e} = Conf^δ_e`（基例 ℓ=e）或
///   `Cand^δ_ℓ ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}`（递归步）。执行级 e=lvl（信号所在级），
///   rungs 包含 lvl+1 到 tower 顶-1 的各级 Cand + 区间。
/// - **边界条件**：tower 为空或 tower[lvl] 无 source_index 匹配段 ⟹ false。任一级 Cand=false
///   ⟹ false。任一相邻级区间不满足 ⊆ ⟹ false。基例 Conf^δ_e=false ⟹ false。
/// - **下游推论**：`decompose_capturable_spread` 的信号过滤改为本函数（多级 N^δ 门），
///   取代 `bsp_div_cand` 单级门——信号集改变（只收多级链通过的信号）。
/// - **谱系引用**：W1 工位 spec 三方一致（cand_predicate.rs docstring + codex C1 裁决 + 本文件）。
///   不确定是否有相关概念分离谱系，保守声明。
/// - **影响声明**：新增本函数；替换 decompose_capturable_spread 中的 bsp_div_cand 调用；
///   信号集内容改变（L0 结构过滤，不声明 alpha）。
///
/// **认识论 L0**：纯结构构造 + 确定性算术。alpha 有效性待 L2/L3（W-VERIFY），不在此声明。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_multilevel_nest_cert(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: super::super::classifier::divergence::DivergenceGauge,
) -> bool {
    match build_nest_certificate(tower, lvl, source_index, delta, bits, hist, strokes, gauge) {
        Some(cert) => cert.n_delta(),
        None => false, // 执行级无候选段（tower[lvl] 无包含 source_index 的段）⟹ 无定位
    }
}

/// 从塔构造 `NestCertificate`（区间套证书的**构造**，与 `n_delta()` **判定**分离）。
///
/// 门函数 `build_multilevel_nest_cert` = 本函数 + `.n_delta()`；诊断函数
/// `effective_nest_depth` = 本函数 + `take_while(cand)` 前缀长度。两者**共用同一构造代码** ⟹
/// 诊断读到的 rung 深度与生产门实际消费的 rung 链 **bit-exact 同源**（不是外部近似重算）。
///
/// **认识论 L0**：纯结构构造 + 确定性算术（同 `build_multilevel_nest_cert`）。
/// 返回 `None` ⟺ 执行级 tower[lvl] 无包含 source_index 的段（无定位候选，门直接拒）。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_nest_certificate(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: super::super::classifier::divergence::DivergenceGauge,
) -> Option<NestCertificate> {
    // 执行级 tower[lvl] 中找包含 source_index 的段（候选段 s，执行级 e=lvl）。
    // ★#1052（#1028 裁定 A）：点锚迁移到 departure 单元终点后 source_index 可能落在 C 段**内部**
    // （末子段为回抽反趋势段），`end ==` 精确匹配会定位失败误拒 ⟹ 改「区间包含」定位。
    let exec_moves = tower.get(lvl)?.as_slice();
    let s = &exec_moves[find_move_containing_index(exec_moves, source_index)?];
    let base_interval = NestInterval {
        start_index: s.start_index as u64,
        end_index: s.end_index as u64,
        idx: s.id.ordinal,
    };

    // 673-fix（codex 裁决①）：Cand^δ_ℓ 按 bsp 类型**接口级三分拆**——分派键 + base gate + per-rung
    // 均经 [`bsp_cand_type`]/[`cand_delta_base_gate`]/[`cand_delta`] 命名谓词，函数内不再 if bsp_class
    // 混跑。Type1 走 div_cand；Type2/Type3 存在性由定律一下沉锚定，保护边界（Type2=一类点极值 /
    // Type3=中枢 ZG/ZD）归属记录于各谓词 docstring（上游 bit 置位时已强制，Cand 层不双门）。
    let cand_type = bsp_cand_type(bits, delta);

    // base gate：Type2/3 定律一下沉锚定（第29课L396）。None=小转大 ⟹ 整证书拒（显式可测判别，
    // 非 catch-all fallback）。Type1 无 base gate（判据在 per-rung）；lvl==0 存在性免门。
    if !cand_delta_base_gate(cand_type, s, source_index, delta, hist, lvl, strokes, gauge) {
        return None;
    }

    // rungs：从高级向执行级降序（rungs[0]=最高级，rungs[last]=lvl+1 级）。
    // 对每个上级 k = lvl+1 到 tower.len()-1：
    //   - 找 tower[k] 中包含 source_index 的段（start_index ≤ source_index ≤ end_index）作为区间
    //   - 在 tower[k] 的上级 tower[k+1] 的 sub_moves 中计算 Cand^δ_k
    //   - 若某级找不到包含段 ⟹ break（partial chain 合法，上级语境到此为止；非「提前 false」——旧注释
    //     「⟹ 提前 false」与实装的 break 语义相反，已订正为与实装一致，#817 N-2 T-1 / #1073 S10-b）。
    let max_k = tower.len();
    let mut rung_buf: Vec<NestRung> = Vec::new(); // 从低到高先收集，最后反转
    for k in (lvl + 1)..max_k {
        let k_moves = tower[k].as_slice();
        // 找 tower[k] 中包含 source_index 的段（k 级 Compose；leftmost end>=src 且 start<=src）。
        // ponytail: 单生产调用点，partition_point 内联——含段查找非 end== 精确匹配，不套 find_move_by_end_index。
        debug_assert!(
            k_moves.windows(2).all(|w| w[0].end_index <= w[1].end_index),
            "tower[k] 须按 end_index 升序（partition_point 前提）"
        );
        let ki = k_moves.partition_point(|m| m.end_index < source_index);
        let Some(knode) = k_moves.get(ki).filter(|m| m.start_index <= source_index) else {
            // 无 k 级包含段 ⟹ 链断，不延伸。**设计选择：partial chain 合法**（codex #39 Q1 裁定）。
            // N^δ_{ℓ↓e} 的执行级 e=信号级 lvl（非 tower 顶）；rungs 是「e 之上到首个无包含段」的
            // 上级语境层。链断 = 该 source_index 在更高层无覆盖段 = 上级语境到此为止，**不是**要求
            // 从 tower 顶完整下钻（spec ℓ=信号级，无「必须到顶」约束）。故 break 而非 return None——
            // return None 会强加 spec 没有的「完整 tower 链」约束，拒绝合法的部分语境信号（no-patch）。
            break;
        };
        let interval_k = NestInterval {
            start_index: knode.start_index as u64,
            end_index: knode.end_index as u64,
            idx: knode.id.ordinal,
        };
        // Cand^δ_k：薄 dispatcher（673-fix）——per-rung 候选谓词按类型分派。
        //   Type1 → 本级趋势背驰段 div_cand（区间套原文对象，606 号有效域；bit-exact 不动）。
        //   Type2/3 → 存在性已由 base gate 门控（descend anchor），上级 rung 载上级语境 ⟹ true。
        // knode.sub_moves 是 lvl 到 k-1 级的窗口序列（Type1 在其中找执行级候选段算四条件）。
        let cand_k = cand_delta(
            cand_type,
            knode.sub_moves.as_slice(),
            source_index,
            delta,
            hist,
            strokes,
            // ★#883：D-3 取段的「界」= rung 父走势 knode 的最近中枢。
            super::super::classifier::cand_predicate::parent_last_center(knode),
            gauge,
        );
        rung_buf.push(NestRung::new(interval_k, cand_k));
    }
    // n_delta 期望 rungs[0]=最高级，rungs[last]=lvl+1 级——rung_buf 是低到高，需反转。
    rung_buf.reverse();
    // #100 问题① 看守：定位区间链逐级相套 J_e⊆…⊆J_ℓ（区间包含口径）——tower 层级 Compose 不变量
    // 隐式保证的显式断言。降序链 rungs[0](最高)…rungs[last](lvl+1)…base(执行级)，相邻须 inner⊆outer。
    // debug-only（release 编译掉，零行为改动，同 line 808 既有 debug_assert）。空 rungs 平凡成立。
    debug_assert!(
        rung_buf
            .iter()
            .map(|r| r.interval())
            .chain(std::iter::once(base_interval))
            .collect::<Vec<_>>()
            .windows(2)
            .all(|w| is_sub(&w[1], &w[0])),
        "#100 嵌套链破裂 J_e⊆…⊆J_ℓ（Compose 不变量违反）：base={base_interval:?} rungs={rung_buf:?}"
    );
    Some(NestCertificate::from_parts(
        delta,
        *bits,
        base_interval,
        rung_buf,
    ))
}

/// R5-c opsem-dump（基因 073a/274号）：候选的**区间套深度** Ndepth = 从执行级 `lvl` 向上连续
/// 包含 `source_index` 的塔层数（= [`NestCertificate::rungs`]`.len()` 同口径）。
///
/// **与生产门同算法同源**：复用 [`build_nest_certificate`] rungs 构造循环的同款
/// `partition_point` 含段查找（`start ≤ source_index ≤ end`）+ 无包含段即 `break`——读出的深度
/// 与生产门实际构造的 rung 链**逐级对应**（生产门额外算 `cand` 填 rung 字段，但 rung 的**存在性**
/// 由含段查找决定，与本函数完全一致）。
///
/// **不依赖 hist/MACD**：生产门的 base gate（Type2/3 经 [`descend_type1_anchor_depth`]）与 per-rung
/// `cand_delta`（Type1 经 [`div_cand`]）才需 hist；深度本身是纯结构读数（含段层数），不需力度判据。
/// runner π 路径无 hist（MACD 在 signal 层算），故本函数绕开 hist 依赖——这是它与
/// [`build_nest_certificate`] 的唯一差异（算法同源，输入约束不同）。
///
/// **bit-exact**（R5-1 铁律）：仅供 opsem-dump 消费，写入 `OpsemEntrySnapshot.nest_depth`（dump
/// 专用），**不进 `entry_z`/MuClass/μ 桶键**——纯只读外化。返回 `u8`（与 `MuClass.nest_depth`
/// 同类型；深度上限 = 塔层数，远小于 256）。
pub(super) fn structural_nest_depth(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
) -> u8 {
    let mut depth: usize = 0;
    for k in (lvl + 1)..tower.len() {
        let k_moves = tower[k].as_slice();
        // 与 build_nest_certificate 行 929-940 同款：partition_point 含段查找，无包含段即 break。
        debug_assert!(
            k_moves.windows(2).all(|w| w[0].end_index <= w[1].end_index),
            "tower[k] 须按 end_index 升序（partition_point 前提）"
        );
        let ki = k_moves.partition_point(|m| m.end_index < source_index);
        if !k_moves
            .get(ki)
            .map_or(false, |m| m.start_index <= source_index)
        {
            break;
        }
        depth += 1;
    }
    depth as u8
}

/// 阶段0 对拍探针（cfg(test)，NO-SHIP）：PDF §二「最严格实现 = bottom-up」区间套构造。
///
/// 与生产 [`build_nest_certificate`] 的**唯一差异**在 rung 锚定口径：
/// - 生产：每级 rung 用 **source_index 点包含**（`start≤source≤end`，partition_point 定位含点段）。
/// - 本探针：每级 rung 用 **子区间包含** `J_{k-1} ⊆ I(c)`（`c.start≤child.start ∧ child.end≤c.end`，
///   PDF §二/p4），`Sel_Θ` 作用于「包含 child 的候选集」（PDF p4 反例：先全局 Sel 再检包含会 false
///   negative），且 child 逐级加宽（k=lvl+1 取 base_interval，之后取上一级 J_k）。
///
/// base gate / base_interval / cand 判据全部复用生产同一私有谓词（唯一变量是 knode 选取），故对拍
/// 差异纯粹归因于「点包含 vs 区间包含 + Sel 域」。PDF §三.1：良式分解（recursive_tower Compose
/// refinement，tower[k] 边界 ⊆ tower[k-1] 边界）下含段唯一 ⟹ 二者应逐信号 bit-exact。差异>0 ⟹ 塔
/// 在某处非严格 refinement，须把生产改成 bottom-up（PDF §五最终裁决 b）。
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn build_nest_certificate_bottomup(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: super::super::classifier::divergence::DivergenceGauge,
) -> Option<NestCertificate> {
    use super::super::classifier::nest::sel_order;
    // 基例 e=lvl：与生产同——tower[lvl] 中包含 source_index 的执行级候选段（PDF 基例 J^δ_e；
    // ★#1052 #1028 裁定 A 后与生产一致改用「区间包含」定位）。
    let exec_moves = tower.get(lvl)?.as_slice();
    let s = &exec_moves[find_move_containing_index(exec_moves, source_index)?];
    let base_interval = NestInterval {
        start_index: s.start_index as u64,
        end_index: s.end_index as u64,
        idx: s.id.ordinal,
    };
    let cand_type = bsp_cand_type(bits, delta);
    if !cand_delta_base_gate(cand_type, s, source_index, delta, hist, lvl, strokes, gauge) {
        return None;
    }
    let max_k = tower.len();
    let mut rung_buf: Vec<NestRung> = Vec::new();
    let mut child = base_interval; // J_{k-1}：k=lvl+1 时 = J_e（base），之后逐级加宽为 J_k。
    for k in (lvl + 1)..max_k {
        let k_moves = tower[k].as_slice();
        // bottom-up 候选集 C^δ_k(J_{k-1}) = {c ∈ tower[k] : J_{k-1} ⊆ I(c)}，Sel_Θ 选最优（PDF §二/§三.2）。
        let mut chosen: Option<&LeveledMove> = None;
        for m in k_moves {
            if (m.start_index as u64) <= child.start_index
                && child.end_index <= (m.end_index as u64)
            {
                let mi = NestInterval {
                    start_index: m.start_index as u64,
                    end_index: m.end_index as u64,
                    idx: m.id.ordinal,
                };
                let take = match chosen {
                    None => true,
                    Some(b) => sel_order(
                        &mi,
                        &NestInterval {
                            start_index: b.start_index as u64,
                            end_index: b.end_index as u64,
                            idx: b.id.ordinal,
                        },
                    ),
                };
                if take {
                    chosen = Some(m);
                }
            }
        }
        // 无包含父候选 ⟹ 链断（与生产同：partial chain 合法，codex #39 Q1）。
        let Some(knode) = chosen else {
            break;
        };
        let interval_k = NestInterval {
            start_index: knode.start_index as u64,
            end_index: knode.end_index as u64,
            idx: knode.id.ordinal,
        };
        let cand_k = cand_delta(
            cand_type,
            knode.sub_moves.as_slice(),
            source_index,
            delta,
            hist,
            strokes,
            // ★#883：D-3 取段的「界」= rung 父走势 knode 的最近中枢。
            super::super::classifier::cand_predicate::parent_last_center(knode),
            gauge,
        );
        rung_buf.push(NestRung::new(interval_k, cand_k));
        child = interval_k; // 加宽：下一级用本级 J_k 作 child（真 bottom-up 递归）。
    }
    rung_buf.reverse();
    Some(NestCertificate::from_parts(
        delta,
        *bits,
        base_interval,
        rung_buf,
    ))
}

/// 诊断：通过门信号的 N^δ 证书**有效跨级深度** = 从最高级 rung 起连续 `cand==true` 的层数。
///
/// **为什么不是 `rungs.len()`**：`n_delta` 逐级 `cand ∧ is_sub ∧ 递归`——任一级 `cand==false`
/// 即短路拒绝。故对**通过门**（n_delta=true）的信号，其所有 rung 的 cand 必为 true（否则被拒），
/// `effective_nest_depth = rungs.len()`。但本函数对**任意**证书通用：返回从 rungs[0]（最高级）
/// 起连续 cand=true 的前缀长度——即 N^δ 递归实际穿越的跨级层数。深度 0 = 纯 base-case
/// `confirm_side`（退化，等价单 bit 检查，无区间套跨级）；深度 ≥1 = 真跨级 `[J_{ℓ-1}⊆J_ℓ]` 触达。
///
/// **认识论 L0**：纯结构读数（不重跑分类）。用于 P1 验收「区间套是否真触达 ≥2 层」的 bit-exact 证据。
pub(super) fn effective_nest_depth(cert: &NestCertificate) -> usize {
    cert.rungs().iter().take_while(|r| r.cand()).count()
}

/// 准入门凭据二通道（codex §6-2）：区间套 `Nest` 与小转大 `Xzd` 分派。
///
/// **为何枚举而非空 rungs NestCertificate**：`NestCertificate{rungs:[]}` 的 `n_delta()` 退化为
/// `terminal.confirm_side(side)`——进小转大分支的 Type2/3 信号本已满足该析取（凭 buy2/buy3 走到这里），
/// 会被**无条件放行**（与 C2/C3 无关，静默总放行 bug）。枚举使 `effective_nest_depth`/`n_delta`
/// 只吃 `Nest` 分支，Xzd 通道走独立 `gate_pass`，两者互不污染。
pub(super) enum GateCertificate {
    Nest(NestCertificate),
    Xzd(XzdEvidence),
}

/// P0-1 下沉触发枚举（codex-f2 修正案 #1，`codex-f2-design-ruling-20260703.md`）：**已存在**的三路
/// 准入 dispatch 的显式命名——非新增门，是把 `GateCertificate` 变体 × `BspCandType` 的现有分派语义
/// 显式化，供每桶 μ̂ 质量归因。**核心裁决**：所谓「高级别背驰段前置门」只对 Type1 通道有意义
/// （Type1 = 本级趋势背驰段，第29课 A3），Xzd 通道由 C2/C3 小转大判据独立准入，**不受「无高级别
/// 背驰段」一票否决**（codex #1：现状已 Nest/Xzd 二通道，单一 bool 背驰前置门外延过窄会误杀 Xzd）。
/// G3（#138）可见性/derive 说明：`pub` + `Hash/Ord` 因本枚举作为 [`super::mu_estimator::MuClass`]
/// 第 10 维 `cand_channel` 的分量（§6 CandType「Cand 门通道类型」）——MuClass 是 pub 结构且派生
/// `Hash+Ord`（HashMap 桶键 + BTreeMap 有序报告），分量类型必须同级。定义留在本文件（单源，
/// 不镜像到 mu_estimator——生产者在此，nest_trigger() 是唯一构造点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NestTrigger {
    /// 高级别趋势背驰段（Type1 → `Nest`，div_cand 准入；第29课 A3 先看大级别背驰段）。
    Type1TrendDivergence,
    /// 二三类经次级别第一类下沉锚（Type2/3 → `Nest`，`descend_type1_anchor_depth` base gate 准入）。
    Type23SublevelType1,
    /// 小转大通道（Type2/3 → `Xzd`，C2/C3 小转大判据准入；**不受背驰门一票否决**）。
    XiaoZhuanDa,
    /// Q4 盘整背驰承接（task #145，`PanDivCert` → Nest/XZD 二通道任一通过）：盘整背驰不冒充
    /// 同级 B1/S1，经 `PanDiv^δ_ℓ ⟹ ∃e<ℓ Conf^δ_e ∨ XZD^δ_{ℓ↓e}` 承接入信号流（provenance，
    /// 供每桶 μ̂ 归因分桶；两门皆闭的 PanDiv 不入信号 ⟹ 本变体只标记承接成功者）。
    PanDivConsolidation,
}

/// 从已决 `GateCertificate` + 候选类型派生 `NestTrigger`（纯分类，零行为改动）。
///
/// StructBreak/Type1 nest 失败不产 `GateCertificate`（`build_gate_certificate` 返 None），故到达本函数的
/// `Nest` 变体必为 Type1（div_cand 过）或 Type2/3（descend anchor 过）。`Xzd` 恒 XiaoZhuanDa。
pub(super) fn nest_trigger(cert: &GateCertificate, cand_type: BspCandType) -> NestTrigger {
    match cert {
        GateCertificate::Xzd(_) => NestTrigger::XiaoZhuanDa,
        GateCertificate::Nest(_) => match cand_type {
            BspCandType::Type1 => NestTrigger::Type1TrendDivergence,
            // Type2/3 走 Nest = 经次级别 Type1 下沉锚（base gate 过）；StructBreak 不产 Nest（None 门拒）。
            BspCandType::Type2 | BspCandType::Type3 | BspCandType::StructBreak => {
                NestTrigger::Type23SublevelType1
            }
        },
    }
}

/// Q4 盘整背驰承接门（task #145 裁决：「盘整背驰不能消失：它必须被某级别买卖点或小转大/区间套
/// 证书承接。Route it through N^δ_{ℓ↓e} or a pan-divergence certificate：PanDiv^δ_ℓ ⟹ ∃e<ℓ,
/// Conf^δ_e，或 PanDiv^δ_ℓ ⟹ XZD^δ_{ℓ↓e}」）。
///
/// 二通道复用现有判据基础设施（不 fork 第二套门，不改动既有 Nest/Xzd 信号判定路径——本函数是
/// PanDiv 的**新增入口**，既有信号集 bit 不变）：
/// - **Nest 通道**（∃e<ℓ Conf^δ_e）：[`descend_type1_anchor_depth`]——次级别 Type1 下沉锚
///   （定律一，第29课L396；与 Type2/3 base gate 同一判据函数）。★S4 接线：返回 [`DescendLocator`]
///   ——`.anchored()` ⟹ e=ℓ−depth 的下级确认存在；深度与终止成因不再丢弃（#802 空洞①）。
/// - **XZD 通道**：[`xiaozhuanda_confirm`] → [`XzdEvidence::gate_pass`]（每级同一个判据
///   `C2 ∧ C3(新中枢突破) ∧ ¬例外臂`，单一来源）。
///
/// 任一通过 ⟹ true（承接成立，调用方组 RawSignal，trigger=[`NestTrigger::PanDivConsolidation`]）；
/// 两门皆闭 / tower[lvl] 无包含 source_index 的执行段 ⟹ false（承接失败，诚实丢弃）。
#[allow(clippy::too_many_arguments)]
pub(super) fn pan_div_gate_pass(
    tower: &[Rc<Vec<LeveledMove>>],
    lvl: usize,
    cert: &PanDivCert,
    hist: &[f64],
    confirm_index: usize,
    bsp_of_level: &[BspPoint],
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    // ★#883：通道1 下沉锚（descend → div_cand）的 ForceL 数据源与判据档。
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: super::super::classifier::divergence::DivergenceGauge,
) -> bool {
    // 执行段定位（与 build_nest_certificate 同口径，★#1052/#1234 区间包含）：tower[lvl] 中包含
    // source_index 的段（start_index ≤ source_index ≤ end_index）。
    let Some(exec_moves) = tower.get(lvl).map(|m| m.as_slice()) else {
        return false;
    };
    let Some(si) = find_move_containing_index(exec_moves, cert.source_index) else {
        return false;
    };
    let s = &exec_moves[si];
    // 通道1（Nest 语义 ∃e<ℓ Conf^δ_e）：次级别 Type1 下沉锚。lvl==0（L0 天花板——塔不从笔递归，
    // 构造选择，#520 订正）恒 BaseL0 ⟹ 走通道2。
    let locator = descend_type1_anchor_depth(s, cert.source_index, cert.side, hist, strokes, gauge);
    // ★S4 不变式（#846 B 类零反例 36175/36175）：L0 以上取不到包含段即报错（结构不变量违反）。
    if lvl > 0 {
        debug_assert_ne!(
            locator.stop(),
            DescendStop::NoAlign,
            "PanDiv 下钻：L0 以上取不到包含段（#846 B 类零反例 36175/36175）"
        );
    }
    if locator.anchored() {
        return true;
    }
    // 通道2（XZD^δ_{ℓ↓e}）：小转大确认（gate_pass 单一来源，不重判 C1）。
    let sub_moves: &[LeveledMove] = lvl
        .checked_sub(1)
        .and_then(|l| tower.get(l))
        .map(|m| m.as_slice())
        .unwrap_or(&[]);
    xiaozhuanda_confirm(
        s,
        cert.source_index,
        confirm_index,
        lvl,
        cert.side,
        bsp_of_level,
        sub_centers,
        sub_bsp,
        sub_moves,
        strokes,
    )
    .gate_pass()
}

/// 准入门凭据构造（二通道分派，codex §6-2）：区间套优先，None 域 Type2/3 落小转大通道。
///
/// - `build_nest_certificate` Some ⟹ `Nest`（区间套通道，bit-exact 不动）。
/// - Nest None + 执行段存在 + Type2/3 ⟹ **小转大**（base gate false = descend anchor None）⟹ `Xzd`。
/// - Nest None + 无执行段（case-1 无包含 source_index 的定位候选）或 Type1 背驰失败 ⟹ `None`（门拒，旧语义保留）。
///
/// **bit-exact**：区间套通道逐字复用 build_nest_certificate；只有 Type2/3 小转大域从「None 门拒」改为
/// 「Xzd + gate_pass」——仅新增小转大通过信号，区间套信号集不变。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_gate_certificate(
    tower: &[Rc<Vec<LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
    confirm_index: usize,
    bsp_of_level: &[BspPoint],
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    strokes: &[crate::theta_v0::types::Stroke],
    gauge: super::super::classifier::divergence::DivergenceGauge,
) -> Option<GateCertificate> {
    if let Some(cert) =
        build_nest_certificate(tower, lvl, source_index, delta, bits, hist, strokes, gauge)
    {
        return Some(GateCertificate::Nest(cert));
    }
    // Nest None 回退分支 = `build_xzd_fallback` 单一来源（#75 提取；hist 仅 nest 证构建用，
    // Xzd 分支消费 strokes 供 #985 ForceL 例外臂）——既有调用点改经该函数。
    build_xzd_fallback(
        tower,
        lvl,
        source_index,
        delta,
        bits,
        confirm_index,
        bsp_of_level,
        sub_centers,
        sub_bsp,
        strokes,
    )
    .map(GateCertificate::Xzd)
}

/// Nest-None 域的小转大回退判定（`build_gate_certificate` 回退分支逐字提取为独立函数，
/// #75 单一来源纪律：admission `admit()` 重走（#94 择 (b)）与门凭据构造共用本函数，
/// 禁第二查法）。
///
/// - 无执行段（case-1 无包含 source_index 的定位候选）/ Type1 背驰失败 / StructBreak ⟹ `None`（门拒，旧语义保留）。
/// - Type2/3 ⟹ `Some(XzdEvidence)`（小转大证据，门读 [`XzdEvidence::gate_pass`]）。
#[allow(clippy::too_many_arguments)]
pub(super) fn build_xzd_fallback(
    tower: &[Rc<Vec<LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    confirm_index: usize,
    bsp_of_level: &[BspPoint],
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    strokes: &[crate::theta_v0::types::Stroke],
) -> Option<XzdEvidence> {
    // Nest None：区分小转大（Type2/3 base gate false）与 case-1（无执行段）/Type1 背驰失败。
    let exec_moves = tower.get(lvl)?.as_slice();
    // ★#1052/#1234：与 build_nest_certificate 同口径——区间包含定位（source_index 可能落在
    // 段内部，非段终点）；None=无包含段（无定位候选）⟹ 门拒。
    let s = &exec_moves[find_move_containing_index(exec_moves, source_index)?];
    match bsp_cand_type(bits, delta) {
        BspCandType::Type1 => None, // Type1 nest 失败=div_cand 假，非小转大 ⟹ 拒
        BspCandType::StructBreak => None, // 门拒（codex 终局裁决A）：零 bit 破中枢未背驰候选无确认语义
        BspCandType::Type2 | BspCandType::Type3 => {
            // C3 新判据（codex #44(c)）所需次级走势序列——只读 slice，tower[lvl-1] 不存在时空切片
            // （lvl==0 不入本通道，见函数头注；防御性 `checked_sub` 不 panic）。
            let sub_moves: &[LeveledMove] = lvl
                .checked_sub(1)
                .and_then(|l| tower.get(l))
                .map(|m| m.as_slice())
                .unwrap_or(&[]);
            Some(xiaozhuanda_confirm(
                s,
                source_index,
                confirm_index,
                lvl,
                delta,
                bsp_of_level,
                sub_centers,
                sub_bsp,
                sub_moves,
                strokes,
            ))
        }
    }
}

// σ_higher_at 已上移 selector.rs（codex-q1 G2 单一来源）：z 构造（第 9 维）与信号分解
// （SignalDecomp.sigma_higher，666 号）共用同一函数，防训练/查询口径分叉。本文件顶部导入消费。

// ═══════════════════════════════════════════════════════════════════════════════
// #1147 探针（#[cfg(test)]，env 驱动）：门拒候选的旧臂 StepFail 归因（#846 A/B/C 三分 +
// div_cand 四条件 cond1-4）。只读、零生产影响；未设 env（P1147_STEPFAIL_DUMP_PATH）时全方法 no-op。
//
// 归因口径 = 复刻生产旧臂 build_nest_certificate 的失败分支（base gate / per-rung），
// **不另造判据**：Type2/3 base gate = descend_type1_anchor_depth（A/B/C）；Type1 per-rung =
// cand_delta → div_cand_fail（cond 0..=4）；基例 Conf^δ_e 单列（base_confirm_false）。
// 与生产判据同一函数（div_cand_fail / descend_type1_anchor_depth 即生产本体），parity 由
// 构造保证，无镜像体对拍锁。dump 只落「旧臂 StepFail 桶 + admit + channel」，不进任何判定。
// ═══════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
pub(super) mod stepfail_probe {
    use super::super::super::classifier::cand_predicate::{
        div_cand_fail, parent_last_center, rmove_dir, DivCandInput,
    };
    use super::super::super::classifier::recursive_tower::{
        find_move_by_end_index, find_move_containing_index,
    };
    use super::super::super::strategy::interp::Candidate;
    use super::super::super::strategy::voice::VoiceSide;
    use super::super::super::types::Side;
    use super::*;
    use std::io::Write;

    thread_local! {
        static WRITER: std::cell::RefCell<Option<std::io::BufWriter<std::fs::File>>> =
            std::cell::RefCell::new(None);
    }

    fn lazy_open() {
        WRITER.with(|w| {
            if w.borrow().is_some() {
                return;
            }
            if let Ok(path) = std::env::var(crate::theta_v0::env_registry::P1147_STEPFAIL_DUMP_PATH)
            {
                if path.is_empty() {
                    return;
                }
                match std::fs::File::create(&path) {
                    Ok(f) => *w.borrow_mut() = Some(std::io::BufWriter::new(f)),
                    Err(e) => eprintln!("[p1147] stepfail dump 创建失败 {path}：{e}（no-op 继续）"),
                }
            }
        });
    }

    /// 门拒候选的旧臂 StepFail 归因（纯读数；与 build_nest_certificate 同判据，不参与判定）。
    fn classify(
        tower: &[std::rc::Rc<Vec<super::super::super::classifier::recursive_tower::LeveledMove>>],
        c: &Candidate,
        hist: &[f64],
        classification: &super::super::super::classifier::Classification,
        strokes: &[crate::theta_v0::types::Stroke],
        gauge: super::super::super::classifier::divergence::DivergenceGauge,
    ) -> String {
        let delta = match c.dir {
            VoiceSide::Long => Side::Long,
            VoiceSide::Short => Side::Short,
            VoiceSide::Flat => return "flat_dir".to_string(),
        };
        let lvl = c.level as usize;
        if classification.levels.get(lvl).is_none() {
            return "no_level".to_string();
        }
        let Some(exec_moves) = tower.get(lvl) else {
            return "base_none".to_string();
        };
        let Some(si) = find_move_containing_index(exec_moves.as_slice(), c.source_index) else {
            return "base_none".to_string();
        };
        let cand_type = super::bsp_cand_type(&c.bits, delta);
        match cand_type {
            BspCandType::StructBreak => "struct_break".to_string(),
            BspCandType::Type1 => {
                // base gate（Type1 恒 true，下钻只做 NoAlign 守卫）→ per-rung div_cand 归因。
                let max_k = tower.len();
                for k in (lvl + 1)..max_k {
                    let k_moves = tower[k].as_slice();
                    let ki = k_moves.partition_point(|m| m.end_index < c.source_index);
                    let Some(knode) = k_moves.get(ki).filter(|m| m.start_index <= c.source_index)
                    else {
                        break; // 无 k 级包含段（partial chain 合法）
                    };
                    let ok = super::cand_delta(
                        cand_type,
                        knode.sub_moves.as_slice(),
                        c.source_index,
                        delta,
                        hist,
                        strokes,
                        parent_last_center(knode),
                        gauge,
                    );
                    if !ok {
                        match find_move_containing_index(knode.sub_moves.as_slice(), c.source_index)
                        {
                            Some(tidx) => {
                                let cond = div_cand_fail(&DivCandInput {
                                    context: knode.sub_moves.as_slice(),
                                    target_idx: tidx,
                                    hist,
                                    delta,
                                    strokes,
                                    parent_center: parent_last_center(knode),
                                    gauge,
                                    close_src: None,
                                });
                                return format!("type1_div_fail_{cond:?}");
                            }
                            None => return "type1_rung_no_loc".to_string(),
                        }
                    }
                }
                if !c.bits.confirm_side(delta) {
                    "type1_base_confirm_false".to_string()
                } else {
                    "type1_pass".to_string()
                }
            }
            BspCandType::Type2 | BspCandType::Type3 => {
                let s = &exec_moves[si];
                // 生产 base gate（cand_delta_type2/3_*）：lvl==0 免门（L0 天花板，#520），
                // 否则 descend_type1_anchor_depth 的 `.anchored()` 即门。归因照此复刻，不另造。
                if lvl == 0 {
                    return "type23_l0_exempt".to_string();
                }
                let locator = super::descend_type1_anchor_depth(
                    s,
                    c.source_index,
                    delta,
                    hist,
                    strokes,
                    gauge,
                );
                let stop = locator.stop();
                match stop {
                    DescendStop::BaseL0 => {
                        format!("type23_descend_anchor_baseL0(d={})", locator.depth())
                    }
                    DescendStop::NoAlign => "type23_descend_noalign".to_string(),
                    DescendStop::NoDivergence => {
                        // C 桶再归因：首级对齐/包含段的 div_cand_fail 条件号（1=方向 / 2=D-3取段 /
                        // 3=Extreme / 4=Weak）。与 descend 同一对齐口径（end== → 区间包含回退）。
                        let subs = s.sub_moves.as_slice();
                        let tidx = find_move_by_end_index(subs, c.source_index)
                            .or_else(|| find_move_containing_index(subs, c.source_index));
                        match tidx {
                            Some(t) => {
                                let cond = div_cand_fail(&DivCandInput {
                                    context: subs,
                                    target_idx: t,
                                    hist,
                                    delta,
                                    strokes,
                                    parent_center: parent_last_center(s),
                                    gauge,
                                    close_src: None,
                                });
                                format!("type23_descend_nodiv_cond{cond:?}")
                            }
                            None => "type23_descend_nodiv_noalign".to_string(),
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn record(
        bar: usize,
        c: &Candidate,
        tower: &[std::rc::Rc<Vec<super::super::super::classifier::recursive_tower::LeveledMove>>],
        hist: &[f64],
        classification: &super::super::super::classifier::Classification,
        strokes: &[crate::theta_v0::types::Stroke],
        gauge: super::super::super::classifier::divergence::DivergenceGauge,
        admit: bool,
        channel: &str,
        would_close: bool,
    ) {
        lazy_open();
        WRITER.with(|w| {
            let mut slot = w.borrow_mut();
            let Some(writer) = slot.as_mut() else { return };
            let bucket = classify(tower, c, hist, classification, strokes, gauge);
            let dir = match c.dir {
                VoiceSide::Long => "Long",
                VoiceSide::Short => "Short",
                VoiceSide::Flat => "Flat",
            };
            let line = serde_json::json!({
                "bar": bar,
                "level": c.level,
                "source_index": c.source_index,
                "dir": dir,
                "stepfail": bucket,
                "admit": admit,
                "channel": channel,
                "would_close": would_close,
            });
            let _ = writeln!(writer, "{line}");
            // P1152（#1147 追问票）逐例结构 dump（env 未设 = no-op，见 sample_structure）。
            sample_structure(
                bar,
                c,
                tower,
                hist,
                classification,
                strokes,
                gauge,
                admit,
                channel,
                would_close,
            );
        });
    }

    // ── P1152（#1147 追问票）逐例结构 dump ─────────────────────────────────
    // env P1152_SAMPLE_KEYS_PATH（JSONL：{"bar":..,"level":..,"source_index":..}）载入样本键；
    // 命中候选时落 P1152_SAMPLE_DUMP_PATH 的逐例结构（执行段 / sub_moves / 锚定段 / 方向 /
    // 父中枢 / div_cand 条件 / cand_type / bits / locator_stop）。纯读数、只写 dump，不进任何
    // 判定；对齐/判据口径与 classify 完全同一（find_move_by_end_index → 区间包含回退 →
    // div_cand_fail），不另造判据。env 未设 ⟹ 全方法 no-op。
    thread_local! {
        static SAMPLE_KEYS: std::cell::RefCell<
            Option<std::collections::HashSet<(usize, u32, usize)>>,
        > = std::cell::RefCell::new(None);
        static SAMPLE_WRITER: std::cell::RefCell<Option<std::io::BufWriter<std::fs::File>>> =
            std::cell::RefCell::new(None);
    }

    fn lazy_open_sample() {
        SAMPLE_KEYS.with(|k| {
            if k.borrow().is_some() {
                return;
            }
            let mut keys = std::collections::HashSet::new();
            if let Ok(path) = std::env::var(crate::theta_v0::env_registry::P1152_SAMPLE_KEYS_PATH) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                            if let (Some(bar), Some(level), Some(si)) = (
                                v.get("bar").and_then(|x| x.as_u64()),
                                v.get("level").and_then(|x| x.as_u64()),
                                v.get("source_index").and_then(|x| x.as_u64()),
                            ) {
                                keys.insert((bar as usize, level as u32, si as usize));
                            }
                        }
                    }
                }
            }
            *k.borrow_mut() = Some(keys);
        });
        SAMPLE_WRITER.with(|w| {
            if w.borrow().is_some() {
                return;
            }
            if let Ok(path) = std::env::var(crate::theta_v0::env_registry::P1152_SAMPLE_DUMP_PATH) {
                if !path.is_empty() {
                    if let Ok(f) = std::fs::File::create(&path) {
                        *w.borrow_mut() = Some(std::io::BufWriter::new(f));
                    }
                }
            }
        });
    }

    fn dir_str(d: Option<super::super::super::types::Direction>) -> Option<&'static str> {
        match d {
            Some(super::super::super::types::Direction::Up) => Some("Up"),
            Some(super::super::super::types::Direction::Down) => Some("Down"),
            None => None,
        }
    }

    /// 逐例结构 dump（P1152）：把 cond1 拒的出场候选展开成可读结构，供三档归因
    /// （真该拦 / 判据误伤 / 生成面噪声）。与 [`classify`] 同一对齐/判据口径。
    #[allow(clippy::too_many_arguments)]
    fn sample_structure(
        bar: usize,
        c: &Candidate,
        tower: &[std::rc::Rc<Vec<super::super::super::classifier::recursive_tower::LeveledMove>>],
        hist: &[f64],
        classification: &super::super::super::classifier::Classification,
        strokes: &[crate::theta_v0::types::Stroke],
        gauge: super::super::super::classifier::divergence::DivergenceGauge,
        admit: bool,
        channel: &str,
        would_close: bool,
    ) {
        lazy_open_sample();
        let hit = SAMPLE_KEYS.with(|k| {
            k.borrow()
                .as_ref()
                .is_some_and(|m| m.contains(&(bar, c.level, c.source_index)))
        });
        if !hit {
            return;
        }
        SAMPLE_WRITER.with(|w| {
            let mut slot = w.borrow_mut();
            let Some(writer) = slot.as_mut() else { return };
            let cand_type = match c.dir {
                VoiceSide::Long => super::bsp_cand_type(&c.bits, Side::Long),
                VoiceSide::Short => super::bsp_cand_type(&c.bits, Side::Short),
                VoiceSide::Flat => super::bsp_cand_type(&c.bits, Side::Long),
            };
            let cand_type_str = match cand_type {
                BspCandType::Type1 => "Type1",
                BspCandType::Type2 => "Type2",
                BspCandType::Type3 => "Type3",
                BspCandType::StructBreak => "StructBreak",
            };
            let mut exec_move = serde_json::Value::Null;
            let mut sub_moves = serde_json::Value::Null;
            let mut parent_center = serde_json::Value::Null;
            let mut anchor_sub = serde_json::Value::Null;
            let mut anchor_idx: Option<usize> = None;
            let mut div_cond: Option<u8> = None;
            let mut locator_stop: Option<&str> = None;
            let lvl = c.level as usize;
            if let Some(exec_moves) = tower.get(lvl) {
                if let Some(si) = find_move_containing_index(exec_moves.as_slice(), c.source_index)
                {
                    let s = &exec_moves[si];
                    exec_move = serde_json::json!({
                        "start": s.start_index,
                        "end": s.end_index,
                        "dir": dir_str(rmove_dir(&s.rmove)),
                    });
                    let pc = parent_last_center(s);
                    parent_center = pc
                        .map(|p| {
                            serde_json::json!({
                                "zg": p.zg, "zd": p.zd, "gg": p.gg, "dd": p.dd,
                                "start": p.start_index, "end": p.end_index,
                            })
                        })
                        .unwrap_or(serde_json::Value::Null);
                    let subs = s.sub_moves.as_slice();
                    sub_moves = serde_json::json!(subs
                        .iter()
                        .map(|m| serde_json::json!({
                            "start": m.start_index,
                            "end": m.end_index,
                            "dir": dir_str(rmove_dir(&m.rmove)),
                        }))
                        .collect::<Vec<_>>());
                    if matches!(cand_type, BspCandType::Type2 | BspCandType::Type3) {
                        let tidx = find_move_by_end_index(subs, c.source_index)
                            .or_else(|| find_move_containing_index(subs, c.source_index));
                        anchor_idx = tidx;
                        if let Some(t) = tidx {
                            let tm = &subs[t];
                            anchor_sub = serde_json::json!({
                                "idx": t,
                                "start": tm.start_index,
                                "end": tm.end_index,
                                "dir": dir_str(rmove_dir(&tm.rmove)),
                            });
                        }
                        let delta = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => Side::Long,
                        };
                        let locator = super::descend_type1_anchor_depth(
                            s,
                            c.source_index,
                            delta,
                            hist,
                            strokes,
                            gauge,
                        );
                        locator_stop = Some(match locator.stop() {
                            DescendStop::BaseL0 => "BaseL0",
                            DescendStop::NoAlign => "NoAlign",
                            DescendStop::NoDivergence => "NoDivergence",
                        });
                        if let Some(t) = tidx {
                            div_cond = div_cand_fail(&DivCandInput {
                                context: subs,
                                target_idx: t,
                                hist,
                                delta,
                                strokes,
                                parent_center: parent_last_center(s),
                                gauge,
                                close_src: None,
                            });
                        }
                    }
                }
            }
            let bits = serde_json::json!({
                "class_index": c.bits.class_index(),
                "buy1": c.bits.buy1, "buy2": c.bits.buy2, "buy3": c.bits.buy3,
                "sell1": c.bits.sell1, "sell2": c.bits.sell2, "sell3": c.bits.sell3,
            });
            let line = serde_json::json!({
                "bar": bar,
                "level": c.level,
                "source_index": c.source_index,
                "dir": match c.dir {
                    VoiceSide::Long => "Long",
                    VoiceSide::Short => "Short",
                    VoiceSide::Flat => "Flat",
                },
                "bsp_class": c.bsp_class,
                "cand_type": cand_type_str,
                "bits": bits,
                "exec_move": exec_move,
                "parent_center": parent_center,
                "sub_moves": sub_moves,
                "anchor_sub": anchor_sub,
                "anchor_idx": anchor_idx,
                "div_cond": div_cond,
                "locator_stop": locator_stop,
                "stepfail": classify(tower, c, hist, classification, strokes, gauge),
                "admit": admit,
                "channel": channel,
                "would_close": would_close,
            });
            let _ = writeln!(writer, "{line}");
        });
    }
}

#[cfg(test)]
mod tests;
