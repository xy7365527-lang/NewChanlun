//! 经济正条件逐信号分解（《经济正条件.pdf》第5节可捕获价差判据 L2 诊断）。
//!
//! 证明链 L0 见 `.chanlun/proofs/economic-positive-condition-chain.md`。本模块是其
//! **唯一可否证环节**（可捕获价差判据前件）的 L2 实装：逐信号确定性分解
//!
//!     captured = Ab_rev − ηin − ηout − Ce/qe        (PDF p4-5 §5，对象=反转交易腿)
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

use std::rc::Rc;
use super::data::Dataset;
use super::incremental::IncrementalClassifier;
use super::super::classifier::divergence::compute_macd;
use super::super::classifier::recursive_tower::find_move_by_end_index;
use super::super::classifier::nest::{is_sub, NestCertificate, NestInterval, NestRung};
use super::super::closed_loop::sell::{sell_decision_of, SellDecision};
use super::super::config::ThetaConfig;
use super::super::strategy::interp::assemble_gamma_with_tower;
use super::super::strategy::voice::VoiceSide;
use super::super::types::{Bar, BspBits, Center, Side};
use super::super::classifier::bsp::BspPoint;
use super::super::classifier::recursive_tower::LeveledMove;
use super::super::classifier::signal::PanDivCert;
use super::super::classifier::center::{center_from_segments, UnitRange};
use super::super::classifier::descend::RMove;
use super::mu_estimator::{MuClass, PositionState};
use super::selector::{sigma_higher_at, z_of_candidate, ZExt};
use super::super::strategy::coverage::Horizontal;

/// P7 正规出场口径：配对出场信号的缠论卖点（买点）类别（sell.rs:50 CloseRoot/ReduceCore 对齐）。
///
/// **定义依据**：出场信号 bsp_class bits 已编码卖点判据（EndpointSituation → endpoint_to_bsp）。
/// - `CloseRoot`：exit bits.sell1=true（第一类顶背驰，对应 sell.rs SellDecision::CloseRoot）。
/// - `ReduceCore`：exit bits.sell3=true ∧ sell1=false（第三类，sell.rs SellDecision::ReduceCore）。
/// - `Type2Missing`：exit bits.sell2=true（第二类卖点闭环 sell.rs:35 still-MISSING，诚实标注）。
/// - `Hold`：exit 是反向方向信号但无正规卖点 bits（bits 全0 或仅 buy/sell 位未命中正规类）。
///
/// 对空头入场（δ=−1），exit 是多头信号，用 buy1/buy3/buy2 镜像判据（买点类型对应买侧正规出场）。
///
/// **第二类闭环 still-MISSING**：sell.rs:35 的诚实边界已声明"第二类卖点闭环需次级别递归，见 descend.rs"。
/// 本枚举不臆造 Type2 实装（no-patch）——遇 sell2/buy2 入 `Type2Missing`，记入统计但不改账本口径。
///
/// **账本边界**：本枚举只影响 econ 的 exit_decision 字段（分析统计口径），不触碰 TW 三阶段（GAP3/576）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitDecision {
    /// 第一类正规出场（顶背驰清仓）→ sell.rs CloseRoot。
    CloseRoot,
    /// 第三类正规出场（减核）→ sell.rs ReduceCore。
    ReduceCore,
    /// 第二类卖点闭环 still-MISSING（sell.rs:35 诚实边界）。
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
    /// P7 正规出场口径：配对出场信号的缠论卖（买）点类别（接 sell.rs CloseRoot/ReduceCore）。
    /// 从配对出场信号的 bsp_class 派生（bsp_class bits 已编码卖点判据结果）。
    /// `Type2Missing` = 第二类闭环 sell.rs:35 still-MISSING，诚实标注，不改账本口径。
    pub exit_decision: ExitDecision,
    /// z：完整 [`MuClass`] 全互斥分类键（b2 升 Z 分桶；含 level/δ/i_class/parent_dir/short_swing/position）。
    /// 分桶/dx 投影**一律用 `z.i_class`**（未压缩 6-bit），**禁止 `z.bsp_class()`**（会丢 2B/3B 重合，
    /// 违反 P4 codex 判决）。`bsp_class` 旧字段保留供旧口径对照（CSV/旧报告），不作分桶键。
    pub z: MuClass,
    /// P0-1：准入触发通道（[`NestTrigger`]，codex-f2 #1）——signal-provenance，供每桶 μ̂ 质量归因。
    pub(super) trigger: NestTrigger,
}

/// 聚合诊断「钱去哪了」（确定性分解，Σ 精确等于 Σ trade gross，非概率推断）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpreadAttribution {
    pub n_signals: usize,
    /// Σ Ab_rev：反转交易腿给的理想总价差（664 号：post-signal 腿，非触发段）。
    pub sum_a_b: f64,
    /// Σ x_in：**signed** 入场滑移总和（664-Q3）。含有利（<0）+不利（>0），非 adverse-only。
    pub sum_x_in: f64,
    /// Σ y_out：**signed** 出场滑移总和（664-Q3）。
    pub sum_y_out: f64,
    /// Σ ηin = Σ max(0, x_in)：入场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_in: f64,
    /// Σ ηout = Σ max(0, y_out)：出场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_out: f64,
    /// Σ actual_spread = Σ δ(Pτout − Pτin)：**真实成交价差**总和（无成本，664-Q3，含全部滑移）。
    pub sum_actual_spread: f64,
    /// Σ Ce/qe：成本吃掉。
    pub sum_ce: f64,
    /// Σ captured：adverse-only 保守压力测试剩余 alpha（丢有利滑移，系统性偏负）。
    pub sum_captured: f64,
    /// 可捕获信号数（captured>0）——逐信号路径级正占比的分子。
    pub n_captured_positive: usize,
    /// actual_pnl>0 信号数（真实成交口径）——区别于 n_captured_positive（adverse-only）。
    pub n_actual_positive: usize,
    /// 三审计统计①：rho_rev_bar>lambda_rev_bar 计数（出场 pivot 端点在入场 pivot 端点之后；codex Q1 不变量）。
    pub n_rho_after_lambda: usize,
    /// 三审计统计②：same_bar_opposite 计数（同 bar 出现反向信号被 eb>entry_bar 排除；codex Q2 边界）。
    pub n_same_bar_opposite: usize,
    /// 三审计统计③：n_unpaired 计数（入场信号无配对出场反转信号，右删失诚实跳过；codex Q4 边界）。
    pub n_unpaired: usize,
    /// P7 正规出场统计：CloseRoot（第一类顶背驰）配对出场信号数。
    pub n_exit_close_root: usize,
    /// P7 正规出场统计：ReduceCore（第三类）配对出场信号数。
    pub n_exit_reduce_core: usize,
    /// P7 正规出场统计：Type2Missing（第二类 sell.rs:35 still-MISSING）配对出场信号数（诚实标注）。
    pub n_exit_type2_missing: usize,
    /// P7 正规出场统计：Hold（无正规卖点 bit 的反向信号出场）配对出场信号数。
    pub n_exit_hold: usize,
}

impl SpreadAttribution {
    /// L2 否证判据（**adverse-only 保守口径**）：执行损耗（含成本）是否吃光结构价差。
    ///
    /// `true` ⟺ Σcaptured ≤ 0。注意：captured 用 max(0,·) 丢弃有利滑移 ⟹ 系统性低估真实 PnL，
    /// 这是**压力测试**口径，不等于真实成交。真实成交口径见 `actual_pnl_proxy`/`actual_pnl_eaten`。
    pub fn spread_eaten(&self) -> bool {
        self.sum_captured <= 0.0
    }

    /// 真实成交 PnL 代理（664-Q3 codex）：Σactual_spread − ΣCe = Σδ(Pτout−Pτin) − ΣCe。
    /// 含全部滑移（有利+不利），是确认 bar 实际成交价差减成本——比 adverse-only captured 更接近真实成交。
    pub fn actual_pnl_proxy(&self) -> f64 {
        self.sum_actual_spread - self.sum_ce
    }

    /// 真实成交口径的「执行吃光」判据：actual_pnl_proxy ≤ 0。
    ///
    /// 与 `spread_eaten`（adverse-only）的分歧即 codex Q3 关注点：若 `spread_eaten`=true 但本判据=false，
    /// 则「执行滞后吃光」是 adverse-only 伪结论（有利滑移被丢弃所致），真实成交其实可捕获。
    pub fn actual_pnl_eaten(&self) -> bool {
        self.actual_pnl_proxy() <= 0.0
    }
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
pub fn decompose_capturable_spread(data: &Dataset, config: &ThetaConfig) -> (Vec<SignalDecomp>, SpreadAttribution) {
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
        let (cls_i, tower_i) = classifier_incr.classify_at(i);
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            if prev_bsp.get(lvl).map_or(false, |prev| Rc::ptr_eq(prev, &ls.bsp)) {
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
                            bsp: Rc::new(if l2 == lvl { vec![p.clone()] } else { Vec::new() }),
                            pan_div: Rc::new(Vec::new()), // Q4：single 屏蔽层无盘整背驰载荷（只供 Γ 组装）
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
                    let sub_centers: &[Center] = if lvl > 0 { &cls_i.levels[lvl - 1].centers } else { &[] };
                    let sub_bsp: &[BspPoint] = if lvl > 0 { &cls_i.levels[lvl - 1].bsp } else { &[] };
                    let gate_cert = build_gate_certificate(
                        &tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist, i, &ls.bsp, sub_centers, sub_bsp,
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
                    // （链顶 ℓ，「执行级 e=lvl，rungs 收集上级语境」的有效域口径）；Xzd 通道无下沉
                    // 概念 ⟹ depth=None、origin_level 走 z_of_candidate 默认 Some(c.level)（起始=执行）。
                    // risk_mode=None：统计层信号收集无账本（equity/持仓），诚实 None（第 13 维在
                    // runner π fill loop 生态填真值）。
                    let ext = match gate_cert.as_ref().expect("pass ⟹ gate_cert Some") {
                        GateCertificate::Nest(cert) => ZExt {
                            cand_channel: Some(trigger),
                            nest_depth: Some(cert.rungs.len() as u8),
                            origin_level: Some(lvl as u32 + cert.rungs.len() as u32),
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
                    debug_assert_eq!(z.sigma_higher, Some(sigma_higher), "z.sigma_higher 与 SignalDecomp 口径分叉");
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
                let sub_centers: &[Center] =
                    if lvl > 0 { &cls_i.levels[lvl - 1].centers } else { &[] };
                let sub_bsp: &[BspPoint] = if lvl > 0 { &cls_i.levels[lvl - 1].bsp } else { &[] };
                if !pan_div_gate_pass(
                    &tower_i, lvl, cert, &macd_hist, i, &ls.bsp, sub_centers, sub_bsp,
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
                // nest_depth=None（rungs 下沉深度概念不适用，同 Xzd 口径）。
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
        next_long[i] = if signals[i].dir == VoiceSide::Long { i } else { next_long[i + 1] };
        next_short[i] = if signals[i].dir == VoiceSide::Short { i } else { next_short[i + 1] };
    }

    for (idx, s) in signals.iter().enumerate() {
        let RawSignal { entry_bar, dir, pivot_bar: lambda_rev_bar, level, sigma_higher, bsp_class, z, trigger } = *s;
        let delta: i8 = match dir {
            VoiceSide::Long => 1,
            VoiceSide::Short => -1,
            VoiceSide::Flat => continue,
        };
        let opp = if delta == 1 { VoiceSide::Short } else { VoiceSide::Long };
        let next_opp = if opp == VoiceSide::Long { &next_long } else { &next_short };
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
            (signals[j].entry_bar, signals[j].pivot_bar, signals[j].bsp_class)
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
        if [p_lambda, p_rho, p_tau_in, p_tau_out].iter().any(|&x| x <= 0.0) {
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
        // P7 正规出场口径：从配对出场信号 bsp_class 派生（接 sell.rs CloseRoot/ReduceCore）。
        let exit_decision = exit_decision_from_bits(exit_bsp_class, delta);

        decomps.push(SignalDecomp {
            entry_bar, exit_bar, level, delta, a_b, x_in, y_out,
            eta_in, eta_out, actual_spread, ce_unit, captured, actual_pnl, sigma_higher, bsp_class,
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
        // P7 出场决策统计（接 sell.rs CloseRoot/ReduceCore，Type2Missing=still-MISSING 诚实标注）。
        match exit_decision {
            ExitDecision::CloseRoot => agg.n_exit_close_root += 1,
            ExitDecision::ReduceCore => agg.n_exit_reduce_core += 1,
            ExitDecision::Type2Missing => agg.n_exit_type2_missing += 1,
            ExitDecision::Hold => agg.n_exit_hold += 1,
        }
    }

    (decomps, agg)
}

/// P7 正规出场口径：配对出场信号 bsp_class bits → ExitDecision，**委托 closed_loop 平仓决策权威**。
///
/// **接线（非新逻辑）**：type1>type3 优先级 + CloseRoot/ReduceCore 语义由
/// [`closed_loop::sell::sell_decision_of`](super::super::closed_loop::sell::sell_decision_of) 单一决定，
/// 本函数只做「bsp bits → (is_type1, is_type3, is_type2) 判据」的解包与方向选择，再把 [`SellDecision`]
/// 提升为 [`ExitDecision`]（加诊断态 Type2Missing）。优先级不在此重编码 ⟹ 与 `recog_chanlun_sell`
/// 共用同一来源（no-patch）。
///
/// **定义依据**：
/// - 多头入场（δ=+1），exit 是 Short 信号：看卖侧 bits（bit3=sell1, bit4=sell2, bit5=sell3）。
/// - 空头入场（δ=−1），exit 是 Long 信号：看买侧 bits（bit0=buy1, bit1=buy2, bit2=buy3，买点镜像卖点）。
/// - `sell_decision_of` 返回 `CloseRoot`（type1 命中，优先）/ `ReduceCore`（type3 命中）/ `Hold`。
/// - `Hold` 且命中 type2 → `Type2Missing`（sell.rs:35 第二类闭环 still-MISSING 诚实标注）；否则 `Hold`。
///
/// **边界条件**：若出场方向无 type1/type2/type3 bit（含 bsp_class=0）→ Hold（无正规出场依据）。
/// **账本边界**：不触碰 TW 三阶段（GAP3/576 still-MISSING，econ 只用 R 账本 closed_loop 对齐）。
/// **认识论 L0**：纯 bit 解包 + closed_loop 权威映射，不声明 alpha（alpha 待 W-VERIFY L2/L3）。
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
    // closed_loop 权威：type1>type3 优先级 + 平仓语义单一来源。
    match sell_decision_of(is_type1, is_type3) {
        SellDecision::CloseRoot => ExitDecision::CloseRoot,
        SellDecision::ReduceCore => ExitDecision::ReduceCore,
        // Hold（无 type1/type3）：命中 type2 则诚实标注闭环缺口，否则真 Hold。
        SellDecision::Hold if is_type2 => ExitDecision::Type2Missing,
        SellDecision::Hold => ExitDecision::Hold,
    }
}

/// bar close → 价格（close×tick），越界/非正返 0。
fn px_at(bars: &[Bar], i: usize, tick: f64) -> f64 {
    bars.get(i).map(|b| b.close as f64 * tick).filter(|&p| p > 0.0).unwrap_or(0.0)
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
pub(super) fn build_multilevel_nest_cert(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
) -> bool {
    match build_nest_certificate(tower, lvl, source_index, delta, bits, hist) {
        Some(cert) => cert.n_delta(),
        None => false, // 执行级无候选段（tower[lvl] 无 end_index==source_index）⟹ 无定位
    }
}

/// 673 号段2：定律一下沉锚定深度（第29课L396「二三类精确的都要下次级别以下找第一类」）。
///
/// Type2/3@ℓ 的精确点 = 次级别 Type1（回抽这个次级别走势的结束点=次级别一类背驰点，定律一 第17课L66）。
/// 从候选段 `s`（=回抽次级别走势，`end_index==source_index`）向次级别下钻：在 `s.sub_moves`
/// （次级别 ℓ-1 走势序列）中找 `end_index==source_index` 的段，跑**完整** `div_cand`（Extreme+Weak
/// 四条件，含盘整背驰——背驰段定义第27课L21 涵盖趋势/盘整，非弱化版）。真递归下沉：锚定成立后继续
/// 钻入该次级别 Type1 段，逐级收缩到最低可用级别（`sub_moves` 空=递归底 level0）。
///
/// - `Some(d)`（d≥1）：次级别 Type1 锚点成立，区间套逐级收缩穿越 d 层（d=最低可用级别的下沉深度）。
/// - `None`：次级别存在但无 Type1 锚点（`div_cand` 假 / 无回抽端点对齐段 / `s` 已是递归底无次级别）
///   = **小转大**（该级别无一类买卖点，精确点无法下沉定位——知识库 L410「区间套和背驰不可解释情况
///   的补充」）。显式可测判别（非 catch-all fallback），门直接拒。
///
/// 区间套 `[J_{ℓ-1}⊆J_ℓ]` 由下钻**结构性保证**：`sub_move` 的 `[start,end]` ⊆ parent 的 `[start,end]`
/// （recursive_tower Compose 由连续 `sub_moves` 组装的不变量），故不重复 `is_sub` 检查（invariant 非条件）。
///
/// **认识论 L0**：纯结构下钻 + 确定性 `div_cand`。复用 N^δ 上钻路径同一 `div_cand` 判据
/// （no-patch，非平行简化版；上钻找 parent，下钻用 sub_moves，是同一区间套的对偶方向）。
fn descend_type1_anchor_depth(
    s: &super::super::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
) -> Option<usize> {
    let subs = s.sub_moves.as_slice();
    if subs.is_empty() {
        return None; // 递归底（level0 无次级别）⟹ 无可下沉的一类锚点 = 小转大
    }
    // 次级别 Type1 背驰段判据：s.sub_moves 中 end_index==source_index 段跑完整 div_cand。
    let tidx = find_move_by_end_index(subs, source_index)?; // 无回抽端点对齐段 ⟹ 小转大
    let anchor_ok = super::super::classifier::cand_predicate::div_cand(
        &super::super::classifier::cand_predicate::DivCandInput {
            context: subs,
            target_idx: tidx,
            hist,
            delta,
        },
    );
    if !anchor_ok {
        return None; // 次级别无一类背驰锚点 ⟹ 小转大
    }
    // 真递归下沉：钻入该次级别 Type1 段，逐级收缩到最低可用级别（深层无锚/到底 ⟹ 本级即最低可用锚）。
    match descend_type1_anchor_depth(&subs[tidx], source_index, delta, hist) {
        Some(d) => Some(d + 1),
        None => Some(1),
    }
}

/// 673-fix（codex 裁决①§673-fix）：Cand^δ_ℓ 候选类型——接口级三分拆的分派键。
///
/// bits 非互斥（P4§5 六买卖点可重合），故按优先级 Type1>Type2>Type3 坍缩到单一候选类型。
/// 保持旧 `is_type1` 语义：buy1/sell1 置位即走 Type1 的 `div_cand` 路径（bit-exact 不动）。
/// StructBreak（codex 终局裁决A，2026-07-02）：`class_index()==0` 的零 bit 破中枢未背驰候选
/// （P2-R2，`signal.rs:577-580`）——概念上与 Type3（未破核心区间回试）几何前提互斥，
/// 不可再落 `else => Type3`（673 号先例：互斥语义混入同一分支须拆分谓词/分支）。
/// 门控：样本层/`MuClass.bsp_class()==0` 统计保留，τ 门控层恒拒（无 BSP 证书）。
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum BspCandType {
    Type1,
    Type2,
    Type3,
    StructBreak,
}

/// 从 `BspBits`+方向派生候选类型（优先级 StructBreak（六 bit 全零）>Type1>Type2>Type3）。
fn bsp_cand_type(bits: &BspBits, delta: Side) -> BspCandType {
    if bits.class_index() == 0 {
        return BspCandType::StructBreak;
    }
    match delta {
        Side::Long => {
            if bits.buy1 {
                BspCandType::Type1
            } else if bits.buy2 {
                BspCandType::Type2
            } else {
                BspCandType::Type3
            }
        }
        Side::Short => {
            if bits.sell1 {
                BspCandType::Type1
            } else if bits.sell2 {
                BspCandType::Type2
            } else {
                BspCandType::Type3
            }
        }
    }
}

/// 673-fix Type1 候选谓词：本级趋势背驰段（区间套原文对象，606 号有效域）。
///
/// per-rung `Cand^δ_k`——在 rung 的次级别走势序列 `rung_subs` 中找 `end_index==source_index`
/// 的执行级候选段，跑 `div_cand`（Extreme+Weak 四条件）。bit-exact 复用旧 Type1 分支逻辑。
fn cand_delta_type1_extreme(
    rung_subs: &[super::super::classifier::recursive_tower::LeveledMove],
    source_index: usize,
    delta: Side,
    hist: &[f64],
) -> bool {
    match find_move_by_end_index(rung_subs, source_index) {
        Some(tidx) => super::super::classifier::cand_predicate::div_cand(
            &super::super::classifier::cand_predicate::DivCandInput {
                context: rung_subs,
                target_idx: tidx,
                hist,
                delta,
            },
        ),
        None => false, // 执行级候选段不在 k 级次级别序列中 ⟹ 无 Cand
    }
}

/// 673-fix Type2 候选谓词：一类点后回抽走势完成（第17课L60 完备性）。
///
/// **保护边界 = 一类点极值**（回抽不破一类点）——由上游结构分类器置 buy2/sell2 位时强制，
/// 本谓词不在 Cand 层重门保护位（no-patch 双门）。存在性锚 = 次级别 Type1（定律一下沉，
/// 第29课L396「二三类精确点要下次级别以下找第一类」）。`lvl==0`（无次级别，递归底）⟹ 存在性
/// 免门（Type2@level0 不误拒）；`None`（无锚）= 小转大（该级无一类精确点无法下沉定位）⟹ 门拒。
fn cand_delta_type2_completion(
    s: &super::super::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
) -> bool {
    lvl == 0 || descend_type1_anchor_depth(s, source_index, delta, hist).is_some()
}

/// 673-fix Type3 候选谓词：离开中枢后回抽/反抽走势完成（**独立分支**，codex 裁决①）。
///
/// **保护边界 = 中枢 ZG/ZD**（离开中枢回抽不入 `c.zg`/`c.zd`，几何见 `descend.rs`
/// `sub_broke_above`/`sub_broke_below`）——与 Type2 的「一类点极值」锚点/失效条件不同，故 Type3
/// **不复用 Type2 顶层谓词**（codex：「Type2 保护位是一类点极值，Type3 保护边界是中枢区间边界」）。
/// ZG/ZD 边界由上游 `six_state.rs` 置 buy3/sell3 位时强制（V型反转回试不入中枢已判），本谓词不在
/// Cand 层重门 ZG/ZD（上游已滤 ⟹ 双门=dead gate，信号集差 0）——保护边界**归属**记录于此，语义与
/// Type2 分离。存在性锚复用 `descend_type1_anchor_depth`（codex 允许「Type3 最多复用 Type2 的
/// 反向走势完成 helper」）：精确点=次级别 Type1（定律一下沉）；`lvl==0` 免门，`None`=小转大门拒。
fn cand_delta_type3_retest(
    s: &super::super::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
) -> bool {
    lvl == 0 || descend_type1_anchor_depth(s, source_index, delta, hist).is_some()
}

/// 673-fix 薄 dispatcher：per-rung `Cand^δ_k` 按候选类型分派（不承载判据逻辑，codex 裁决①）。
///
/// Type1 → 本级背驰段 `div_cand`；Type2/3 存在性已由 [`cand_delta_base_gate`] 门控（base 一次），
/// 上级 rung 载上级语境（第17课L60 完备性保证 Type2/3 存在）⟹ cand=true。
fn cand_delta(
    cand_type: BspCandType,
    rung_subs: &[super::super::classifier::recursive_tower::LeveledMove],
    source_index: usize,
    delta: Side,
    hist: &[f64],
) -> bool {
    match cand_type {
        BspCandType::Type1 => cand_delta_type1_extreme(rung_subs, source_index, delta, hist),
        BspCandType::Type2 | BspCandType::Type3 => true,
        BspCandType::StructBreak => false, // 门拒（codex 终局裁决A）：无对应确认语义，不复用 Type3 锚
    }
}

/// 673-fix base 存在性门（一次，非 per-rung）：Type2/3 定律一下沉锚定，按类型分派。
///
/// Type1 无 base gate（判据在 per-rung `div_cand`）⟹ true。Type2/3 委托各自谓词（存在性锚 +
/// 保护边界归属记录）。返回 false ⟹ 整证书拒（小转大：该级无一类精确点无法下沉定位）。
fn cand_delta_base_gate(
    cand_type: BspCandType,
    s: &super::super::classifier::recursive_tower::LeveledMove,
    source_index: usize,
    delta: Side,
    hist: &[f64],
    lvl: usize,
) -> bool {
    match cand_type {
        BspCandType::Type1 => true,
        BspCandType::Type2 => cand_delta_type2_completion(s, source_index, delta, hist, lvl),
        BspCandType::Type3 => cand_delta_type3_retest(s, source_index, delta, hist, lvl),
        BspCandType::StructBreak => false, // 门拒（codex 终局裁决A）：无对应确认语义，不复用 Type3 锚
    }
}

/// 从塔构造 `NestCertificate`（区间套证书的**构造**，与 `n_delta()` **判定**分离）。
///
/// 门函数 `build_multilevel_nest_cert` = 本函数 + `.n_delta()`；诊断函数
/// `effective_nest_depth` = 本函数 + `take_while(cand)` 前缀长度。两者**共用同一构造代码** ⟹
/// 诊断读到的 rung 深度与生产门实际消费的 rung 链 **bit-exact 同源**（不是外部近似重算）。
///
/// **认识论 L0**：纯结构构造 + 确定性算术（同 `build_multilevel_nest_cert`）。
/// 返回 `None` ⟺ 执行级 tower[lvl] 无 end_index==source_index 段（无定位候选，门直接拒）。
pub(super) fn build_nest_certificate(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
) -> Option<NestCertificate> {
    // 执行级 tower[lvl] 中找 end_index == source_index 的段（候选段 s，执行级 e=lvl）。
    let exec_moves = tower.get(lvl)?.as_slice();
    let s = &exec_moves[find_move_by_end_index(exec_moves, source_index)?];
    let base_interval = NestInterval {
        start_time: s.start_index as u64,
        end_time: s.end_index as u64,
        idx: s.id.ordinal,
    };

    // 673-fix（codex 裁决①）：Cand^δ_ℓ 按 bsp 类型**接口级三分拆**——分派键 + base gate + per-rung
    // 均经 [`bsp_cand_type`]/[`cand_delta_base_gate`]/[`cand_delta`] 命名谓词，函数内不再 if bsp_class
    // 混跑。Type1 走 div_cand；Type2/Type3 存在性由定律一下沉锚定，保护边界（Type2=一类点极值 /
    // Type3=中枢 ZG/ZD）归属记录于各谓词 docstring（上游 bit 置位时已强制，Cand 层不双门）。
    let cand_type = bsp_cand_type(bits, delta);

    // base gate：Type2/3 定律一下沉锚定（第29课L396）。None=小转大 ⟹ 整证书拒（显式可测判别，
    // 非 catch-all fallback）。Type1 无 base gate（判据在 per-rung）；lvl==0 存在性免门。
    if !cand_delta_base_gate(cand_type, s, source_index, delta, hist, lvl) {
        return None;
    }

    // rungs：从高级向执行级降序（rungs[0]=最高级，rungs[last]=lvl+1 级）。
    // 对每个上级 k = lvl+1 到 tower.len()-1：
    //   - 找 tower[k] 中包含 source_index 的段（start_index ≤ source_index ≤ end_index）作为区间
    //   - 在 tower[k] 的上级 tower[k+1] 的 sub_moves 中计算 Cand^δ_k
    //   - 若任何级别找不到包含段 ⟹ 提前 false（无上级语境）
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
            start_time: knode.start_index as u64,
            end_time: knode.end_index as u64,
            idx: knode.id.ordinal,
        };
        // Cand^δ_k：薄 dispatcher（673-fix）——per-rung 候选谓词按类型分派。
        //   Type1 → 本级趋势背驰段 div_cand（区间套原文对象，606 号有效域；bit-exact 不动）。
        //   Type2/3 → 存在性已由 base gate 门控（descend anchor），上级 rung 载上级语境 ⟹ true。
        // knode.sub_moves 是 lvl 到 k-1 级的窗口序列（Type1 在其中找执行级候选段算四条件）。
        let cand_k = cand_delta(cand_type, knode.sub_moves.as_slice(), source_index, delta, hist);
        rung_buf.push(NestRung { interval: interval_k, cand: cand_k });
    }
    // n_delta 期望 rungs[0]=最高级，rungs[last]=lvl+1 级——rung_buf 是低到高，需反转。
    rung_buf.reverse();
    // #100 问题① 看守：定位区间链逐级相套 J_e⊆…⊆J_ℓ（区间包含口径）——tower 层级 Compose 不变量
    // 隐式保证的显式断言。降序链 rungs[0](最高)…rungs[last](lvl+1)…base(执行级)，相邻须 inner⊆outer。
    // debug-only（release 编译掉，零行为改动，同 line 808 既有 debug_assert）。空 rungs 平凡成立。
    debug_assert!(
        rung_buf
            .iter()
            .map(|r| r.interval)
            .chain(std::iter::once(base_interval))
            .collect::<Vec<_>>()
            .windows(2)
            .all(|w| is_sub(&w[1], &w[0])),
        "#100 嵌套链破裂 J_e⊆…⊆J_ℓ（Compose 不变量违反）：base={base_interval:?} rungs={rung_buf:?}"
    );
    Some(NestCertificate {
        side: delta,
        terminal: *bits,
        base_interval,
        rungs: rung_buf,
    })
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
pub(super) fn build_nest_certificate_bottomup(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    lvl: usize,
    source_index: usize,
    delta: Side,
    bits: &BspBits,
    hist: &[f64],
) -> Option<NestCertificate> {
    use super::super::classifier::nest::sel_order;
    // 基例 e=lvl：与生产同——tower[lvl] 中 end_index==source_index 的执行级候选段（PDF 基例 J^δ_e）。
    let exec_moves = tower.get(lvl)?.as_slice();
    let s = &exec_moves[find_move_by_end_index(exec_moves, source_index)?];
    let base_interval = NestInterval {
        start_time: s.start_index as u64,
        end_time: s.end_index as u64,
        idx: s.id.ordinal,
    };
    let cand_type = bsp_cand_type(bits, delta);
    if !cand_delta_base_gate(cand_type, s, source_index, delta, hist, lvl) {
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
            if (m.start_index as u64) <= child.start_time && child.end_time <= (m.end_index as u64) {
                let mi = NestInterval {
                    start_time: m.start_index as u64,
                    end_time: m.end_index as u64,
                    idx: m.id.ordinal,
                };
                let take = match chosen {
                    None => true,
                    Some(b) => sel_order(&mi, &NestInterval {
                        start_time: b.start_index as u64,
                        end_time: b.end_index as u64,
                        idx: b.id.ordinal,
                    }),
                };
                if take { chosen = Some(m); }
            }
        }
        // 无包含父候选 ⟹ 链断（与生产同：partial chain 合法，codex #39 Q1）。
        let Some(knode) = chosen else { break; };
        let interval_k = NestInterval {
            start_time: knode.start_index as u64,
            end_time: knode.end_index as u64,
            idx: knode.id.ordinal,
        };
        let cand_k = cand_delta(cand_type, knode.sub_moves.as_slice(), source_index, delta, hist);
        rung_buf.push(NestRung { interval: interval_k, cand: cand_k });
        child = interval_k; // 加宽：下一级用本级 J_k 作 child（真 bottom-up 递归）。
    }
    rung_buf.reverse();
    Some(NestCertificate {
        side: delta,
        terminal: *bits,
        base_interval,
        rungs: rung_buf,
    })
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
    cert.rungs.iter().take_while(|r| r.cand).count()
}

/// 小转大确认凭据（次级别结构确认通道——独立于区间套；本级无背驰段可套 ⟹ **无 depth**）。
///
/// ## 结果包（六要素）
/// - **结论**：Type2/3 信号在 `descend_type1_anchor_depth==None`（小转大域）时，用二类买卖点代替
///   区间套定位；门通行 = **level==1 时 C2∧C3(新中枢+突破) 硬门，level!=1 维持 C2-only**（#41 判据
///   即 `same_side_same_center` 经 #44 探针确定性证伪为归属链错位——判据换为第43课「背驰后新中枢+
///   反向突破」，codex #44 终局裁定(c)）。输出标注 `C2+C3(breakout) xzd`，不得沿用旧 `C2-only xzd` 标签。
/// - **定义依据**：`053:28`（二类点补充小转大）；第43课「背驰后新中枢+反向突破」原文语义；codex #44
///   终局裁定(c)（judge_third 归属链 vs last_zs 选择链结构性不重合，见 `.chanlun/review-results/
///   codex-decide-20260702-193853-5bbe.md`）。
/// - **边界条件**：若 L2 复测显示 level==1 子集 `c3_new_center_breakout_ok` 命中率为 0% 或 100%，
///   说明判据本身有实现问题（非死门/非全通过的真实结构应产生中间命中率），需回到 codex 复审，不得
///   静默接受（`acc_classification_level_hole_dx` 断言守护）。
/// - **下游推论**：Type2/3 小转大域从「证书 None 门直接拒」改为二通道分派；两通道输入域不相交
///   = Some/None 互斥（codex §6-4 同义反复，非经验命题）。level==1 硬门收紧吞吐（C2∧C3 而非仅 C2），
///   level>=2 维持既定 C2-only 吞吐（本次翻案范围限 level==1，裁定(c)）。
/// - **谱系引用**：606（区间套有效域=Type1）、673（Cand^δ 三分拆）、知识库 L410（小转大补充定位）、
///   #44 探针（C3 center 匹配口径重设计裁决：结构性不重合）。
/// - **影响声明**：`gate_pass()` 改为 `type2_confirmed && (level != 1 || c3_new_center_breakout_ok)`；
///   新增 `c3_new_center_exists`/`c3_new_center_breakout_ok` 两参门字段；旧 4 项 C3 死门诊断字段
///   （`last_zs_exists`/`same_side_l0_type3_any`/`same_center_any`/`same_side_causal_ok`）全部保留为
///   诊断字段（不再是本次唯一分级依据）；不改 build_nest_certificate/n_delta。
///
/// **认识论 L0**：纯结构判据（新中枢存在性+突破几何）。C3 新判据的 level==1 命中率认识论等级见
/// `acc_classification_level_hole_dx` 测试（L2，真实 BTC 数据）。alpha 有效性待 W-VERIFY(#13)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct XzdEvidence {
    /// 本级信号精确点（bsp source_index）。
    pub source_index: usize,
    /// 本级 lvl（小转大域恒 ≥1：lvl==0 时 base gate 免门走区间套，不入本通道）。
    pub level: usize,
    /// δ 方向。
    pub side: Side,
    /// 判据观察点（as-of 首次塔=确认 bar，零前视）。codex §6-3：显式区分 source_index/confirm_index，
    /// C3「动态最后次级中枢」只用 confirm_index 及之前的塔快照（`cls_i` 因果分类），杜绝前视。
    pub confirm_index: usize,
    /// C2：本级二类买卖点成立（跨条目按 source_index 查同级 B2，codex §6-1）。**唯一参门项**。
    pub type2_confirmed: bool,
    /// C3（诊断字段，**不参与 gate_pass**——codex 终局裁定 §5.2：level>=2 结构性死门，level==1
    /// 需下方诊断字段另裁）：最后次级中枢出现三类买卖点（as-of 首次塔）。= `same_side_same_center`。
    pub sub_last_zs_type3: bool,
    /// 诊断（§5.4）：`s` 跨度内是否存在次级中枢（`last_zs` 是否非 None）。
    pub last_zs_exists: bool,
    /// 诊断（§5.4）：as-of 次级 bsp 中是否存在同向 Type3（不问 center 归属）。
    pub same_side_l0_type3_any: bool,
    /// 诊断（§5.4）：as-of 次级 bsp 中是否存在**任意侧** Type3 其 `center == last_zs`。
    pub same_center_any: bool,
    /// 诊断（§5.4）：同侧 Type3 候选中是否存在 `source_index <= confirm_index` 者——为 false 时
    /// 说明同侧 Type3 只存在于 confirm_index 之后（时间确认问题），而非 center 归属问题。
    pub same_side_causal_ok: bool,
    /// 诊断（§5.4）：次级 bsp（sub_bsp）中 Type3 点总数。原「lvl>=2 恒 0」死门前提
    /// （extract_second_for_level 只产 B2/S2）已被 codex-t1 裁定A + #123（0a35f0167c，级别≥1
    /// 一/三类候选生成实装，三类净增 2364）合法作废——lvl>=2 现可非 0，dx 死门改锁基线数值。
    pub sub_bsp_type3_count: usize,
    /// C3 新判据（codex #44 终局裁定(c)）：`source_index`~`confirm_index` 间是否存在新确认次级中枢。
    pub c3_new_center_exists: bool,
    /// C3 新判据（codex #44 终局裁定(c)）：新中枢是否被其后次级走势反向突破（第43课「背驰后新中枢+
    /// 反向突破」）。**level==1 硬门参门项**；level>=2 维持既定 C2-only（本次翻案不改 lvl>=2）。
    pub c3_new_center_breakout_ok: bool,
}

impl XzdEvidence {
    /// 门通行（codex #44 终局裁定(c)）：`level==1` 时 C2 ∧ C3(新中枢突破) 硬门；`level!=1`
    /// 维持既定 C2-only（#41 裁定，本次翻案范围限 level==1）。旧字段
    /// （`sub_last_zs_type3`/`same_side_l0_type3_any`/`same_center_any`/`same_side_causal_ok`）
    /// 全部保留为诊断字段，不参门。
    pub(super) fn gate_pass(&self) -> bool {
        self.type2_confirmed && (self.level != 1 || self.c3_new_center_breakout_ok)
    }
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
///   （定律一，第29课L396；与 Type2/3 base gate 同一判据函数）。`Some(d)` ⟹ e=ℓ−d 的下级确认存在。
/// - **XZD 通道**：[`xiaozhuanda_confirm`] → [`XzdEvidence::gate_pass`]（level==1 C2∧C3 硬门 /
///   其余 C2-only，单一来源）。
///
/// 任一通过 ⟹ true（承接成立，调用方组 RawSignal，trigger=[`NestTrigger::PanDivConsolidation`]）；
/// 两门皆闭 / tower[lvl] 无 end_index==source_index 执行段 ⟹ false（承接失败，诚实丢弃）。
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
) -> bool {
    // 执行段定位（与 build_nest_certificate 同口径）：tower[lvl] 中 end_index==source_index 的段。
    let Some(exec_moves) = tower.get(lvl).map(|m| m.as_slice()) else { return false };
    let Some(si) = find_move_by_end_index(exec_moves, cert.source_index) else { return false };
    let s = &exec_moves[si];
    // 通道1（Nest 语义 ∃e<ℓ Conf^δ_e）：次级别 Type1 下沉锚。lvl==0（递归底无次级别）恒 None ⟹ 走通道2。
    if descend_type1_anchor_depth(s, cert.source_index, cert.side, hist).is_some() {
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
    )
    .gate_pass()
}

/// C2 跨条目查找（codex §6-1）：同级 bsp 列表按 source_index 找共生二类买卖点。
///
/// B1/B3（`extract_signals_with_hist`）与 B2（`extract_second_for_level`）在 mod.rs 是独立提取 +
/// extend + sort，**非按 source_index merge**。Type3-only 信号自身 `bits.buy2=false`，直读自身 bits
/// 拿不到共生 B2 ⟹ 必须显式查同级列表。匹配 δ 侧的 buy2/sell2。
fn xzd_type2_confirmed(bsp_of_level: &[BspPoint], source_index: usize, side: Side) -> bool {
    bsp_of_level.iter().any(|q| {
        q.source_index == source_index
            && match side {
                Side::Long => q.bits.buy2,
                Side::Short => q.bits.sell2,
            }
    })
}

/// C3 as-of（codex §6-3；**诊断字段来源，不再参门**——codex 终局裁定 §5.2）：本级走势 `s` 的
/// **最后一个**次级中枢出现三类买卖点。
///
/// 次级别 = level lvl-1（小转大域 lvl≥1 恒成立）。用 as-of 首次塔（`cls_i` at confirm bar，因果无前视）。
/// 最后次级中枢 = `s` 跨度 [start,end] 内 end_index 最大的次级中枢（动态最后中枢，`044:56` as-of 口径）。
/// C3 = 存在次级三类 bsp 其 center 即该最后中枢（离开该中枢回试不入 ZG/ZD）。必要非充分（思维导图 128）。
/// 值等同 `XzdEvidence.sub_last_zs_type3` / `XzdC3Diag.same_side_same_center`。
fn xzd_sub_last_zs_type3(s: &LeveledMove, sub_centers: &[Center], sub_bsp: &[BspPoint], side: Side) -> bool {
    let Some(last_zs) = sub_centers
        .iter()
        .filter(|c| c.start_index >= s.start_index && c.end_index <= s.end_index)
        .max_by_key(|c| c.end_index)
    else {
        return false; // s 内无次级中枢 ⟹ 无「最后次级中枢」⟹ C3 不成立
    };
    sub_bsp.iter().any(|q| {
        (match side {
            Side::Long => q.bits.buy3,
            Side::Short => q.bits.sell3,
        }) && q
            .center
            .map_or(false, |c| c.start_index == last_zs.start_index && c.end_index == last_zs.end_index)
    })
}

/// C3 死门诊断分项（codex 终局裁定 §5.4；供 level==1 子集死门裁断消费——不参与 gate_pass）。
struct XzdC3Diag {
    last_zs_exists: bool,
    same_side_l0_type3_any: bool,
    same_center_any: bool,
    same_side_causal_ok: bool,
    sub_bsp_type3_count: usize,
}

/// 计算 [`XzdC3Diag`]（裁定 §5.4 精确规格）：`last_zs` 与 [`xzd_sub_last_zs_type3`] 用同一取法
/// （s 跨度内 end_index 最大的次级中枢），避免两条选择链再次分叉（呼应审计代理 §3.4 的开放疑点）。
fn xzd_c3_diag(
    s: &LeveledMove,
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    side: Side,
    confirm_index: usize,
) -> XzdC3Diag {
    let last_zs = sub_centers
        .iter()
        .filter(|c| c.start_index >= s.start_index && c.end_index <= s.end_index)
        .max_by_key(|c| c.end_index);
    let is_same_side_type3 = |q: &BspPoint| match side {
        Side::Long => q.bits.buy3,
        Side::Short => q.bits.sell3,
    };
    let is_type3 = |q: &BspPoint| q.bits.buy3 || q.bits.sell3;
    XzdC3Diag {
        last_zs_exists: last_zs.is_some(),
        same_side_l0_type3_any: sub_bsp.iter().any(is_same_side_type3),
        same_center_any: match last_zs {
            Some(z) => sub_bsp.iter().any(|q| {
                is_type3(q)
                    && q.center.map_or(false, |c| c.start_index == z.start_index && c.end_index == z.end_index)
            }),
            None => false,
        },
        same_side_causal_ok: sub_bsp
            .iter()
            .any(|q| is_same_side_type3(q) && q.source_index <= confirm_index),
        sub_bsp_type3_count: sub_bsp.iter().filter(|q| is_type3(q)).count(),
    }
}

/// C3 死门诊断分项二：「背驰后新中枢+反向突破」（第43课语义，codex #44 终局裁定(c)）。
struct XzdC3BreakoutDiag {
    /// `source_index`（背驰确认点）之后、`confirm_index` 之前是否存在已确认的次级中枢。
    new_center_exists: bool,
    /// 该新中枢之后、`confirm_index` 之前的次级走势是否反向突破其核心区间（ZG/ZD）。
    new_center_breakout_ok: bool,
}

/// C3 新判据（codex #44 终局裁定(c) 精确规格）：新中枢 = `sub_centers` 中
/// `start_index >= source_index && end_index <= confirm_index` 者（小转大 source 之后、confirm
/// 之前已确认的次级中枢）；突破 = 该中枢之后、confirm 之前的次级走势 `m` 满足
/// `Side::Long ⟹ m.rmove.hi() > z.zg`（向上破）/ `Side::Short ⟹ m.rmove.lo() < z.zd`（向下破）。
///
/// **off-by-one**：`start_index >= source_index` 是硬约束（不用 `>`）——中枢起点与 source 同 bar
/// 仍算「source 之后已确认」，最小单测钉住此边界（见 tests）。ZG/ZD 是最小结构突破口径；
/// GG/DD 属 C4，不混入本判据（裁定原文边界条件）。
fn xzd_c3_new_center_breakout(
    source_index: usize,
    confirm_index: usize,
    side: Side,
    sub_centers: &[Center],
    sub_moves: &[LeveledMove],
) -> XzdC3BreakoutDiag {
    let new_centers: Vec<&Center> = sub_centers
        .iter()
        .filter(|c| c.start_index >= source_index && c.end_index <= confirm_index)
        .collect();
    let new_center_breakout_ok = new_centers.iter().any(|z| {
        sub_moves
            .iter()
            .filter(|m| m.start_index >= z.end_index && m.end_index <= confirm_index)
            .any(|m| match side {
                Side::Long => m.rmove.hi() > z.zg,
                Side::Short => m.rmove.lo() < z.zd,
            })
    });
    XzdC3BreakoutDiag { new_center_exists: !new_centers.is_empty(), new_center_breakout_ok }
}

/// C3 L1 零命中根因判别探针（codex #55 终局裁定(5) 精确规格）：**纯只读旁路**，区分「非重叠
/// 三段窗口压掉新中枢」（候选1，`detect_centers_with` 算法限制）vs「真实几何无新中枢」（候选2，
/// 市场事实）。不写 `Classification.levels[*].centers`，不改 `tower`，不改正常输出 digest；
/// 默认不调用（调用现场 `ECON_C3_OVERLAP_PROBE` 环境变量门控，生产/默认测试路径零开销）。
struct XzdC3OverlapProbeDiag {
    /// 滑动一格扫描（非 `detect_centers_with` 的非重叠三段消费）产出的 post-source 新中枢窗口数
    /// （`start_index >= source_index && end_index <= confirm_index`，可能同一中枢被多个重叠窗口
    /// 重复命中——诊断计数，非去重集合）。
    overlapping_new_center_count: usize,
    /// 上述新中枢中，被其后次级走势反向突破者的窗口数（突破规则完全复用
    /// [`xzd_c3_new_center_breakout`]：`Side::Long ⟹ hi>zg` / `Side::Short ⟹ lo<zd`）。
    overlapping_breakout_count: usize,
    /// 第一个 post-source 新中枢的 `(start_index, end_index)`（调试用，无命中为 `None`）。
    first_overlapping_center: Option<(usize, usize)>,
}

/// 滑动重叠窗口扫描（`sub_units.windows(3)`，步长1）——与生产 `detect_centers_with`（成立支+3、
/// 不成立支+1，非重叠）唯一的差异点。复用同一中枢构造函数 `center_from_segments`（方向交替+全三段
/// 核心非空，L0 完整判据——本探针只在 `lvl==1` 调用，`sub_units` 恒为 L0 段，`center_from_window`
/// 的几何路径不适用于此层）+ 同一突破规则（`xzd_c3_new_center_breakout`）。
fn xzd_c3_overlap_window_probe(
    source_index: usize,
    confirm_index: usize,
    side: Side,
    sub_units: &[UnitRange],
    sub_moves: &[LeveledMove],
) -> XzdC3OverlapProbeDiag {
    let mut new_centers: Vec<Center> = Vec::new();
    for w in sub_units.windows(3) {
        if let Some(c) = center_from_segments(&w[0], &w[1], &w[2]) {
            if c.start_index >= source_index && c.end_index <= confirm_index {
                new_centers.push(c);
            }
        }
    }
    let first_overlapping_center = new_centers.first().map(|c| (c.start_index, c.end_index));
    let overlapping_breakout_count = new_centers
        .iter()
        .filter(|z| {
            sub_moves
                .iter()
                .filter(|m| m.start_index >= z.end_index && m.end_index <= confirm_index)
                .any(|m| match side {
                    Side::Long => m.rmove.hi() > z.zg,
                    Side::Short => m.rmove.lo() < z.zd,
                })
        })
        .count();
    XzdC3OverlapProbeDiag {
        overlapping_new_center_count: new_centers.len(),
        overlapping_breakout_count,
        first_overlapping_center,
    }
}

/// 从 L0 层携坐标走势塔（`tower[0]`，恒为 `RMove::Segment` 变体——递归底）还原 `UnitRange` 序列，
/// 供 [`xzd_c3_overlap_window_probe`] 的滑窗输入。**必须**取真实线段方向（`RMove::Segment.direction`），
/// 不能用 `recursive_tower::project_to_units` 的外缘折叠方向（`fold_direction` 是几何路径占位，
/// L0 完整判据 `DirAlternates` 需要真方向——见 center.rs 诚实有效域声明）。`tower[0]` 本就是
/// `moves_tower = units.iter().map(LeveledMove::from_unit).collect()`（mod.rs `classify_impl`
/// level0 处理前的初始塔）的逐字段包装，故此还原与 `detect_centers_with` 原本消费的 L0 段账本
/// bit-exact 一致（同 start/end/direction/lo/hi）。
fn l0_units_from_tower(moves: &[LeveledMove]) -> Vec<UnitRange> {
    moves
        .iter()
        .map(|m| {
            let direction = match &m.rmove {
                RMove::Segment { direction, .. } => *direction,
                RMove::Compose { .. } => {
                    unreachable!("tower[0] 恒为 L0 RMove::Segment（递归底，调用侧只在 lvl==1 用本函数）")
                }
            };
            UnitRange {
                start_index: m.start_index,
                end_index: m.end_index,
                direction,
                lo: m.rmove.lo(),
                hi: m.rmove.hi(),
            }
        })
        .collect()
}

/// 小转大确认（设计 §2.2 C1∧C2；C3 改为「新中枢+突破」硬门——仅 level==1，codex #44 终局裁定(c)，
/// 门通行判据见 [`XzdEvidence::gate_pass`]）。
///
/// 前提（调用侧路由保证）：`s` 是执行级 tower[lvl] 中 end_index==source_index 的候选段，且信号已判为
/// 小转大域（Type2/3 ∧ descend anchor None ⟹ build_nest_certificate 返回 None）。C1（descend=None）由
/// 调用侧保证，本函数不重判。evidence 始终构造（含 C2/C3/诊断分项取值）——诊断可读分项，门读 gate_pass。
#[allow(clippy::too_many_arguments)]
fn xiaozhuanda_confirm(
    s: &LeveledMove,
    source_index: usize,
    confirm_index: usize,
    lvl: usize,
    side: Side,
    bsp_of_level: &[BspPoint],
    sub_centers: &[Center],
    sub_bsp: &[BspPoint],
    sub_moves: &[LeveledMove],
) -> XzdEvidence {
    let diag = xzd_c3_diag(s, sub_centers, sub_bsp, side, confirm_index);
    let breakout = xzd_c3_new_center_breakout(source_index, confirm_index, side, sub_centers, sub_moves);
    XzdEvidence {
        source_index,
        level: lvl,
        side,
        confirm_index,
        type2_confirmed: xzd_type2_confirmed(bsp_of_level, source_index, side),
        sub_last_zs_type3: xzd_sub_last_zs_type3(s, sub_centers, sub_bsp, side),
        last_zs_exists: diag.last_zs_exists,
        same_side_l0_type3_any: diag.same_side_l0_type3_any,
        same_center_any: diag.same_center_any,
        same_side_causal_ok: diag.same_side_causal_ok,
        sub_bsp_type3_count: diag.sub_bsp_type3_count,
        c3_new_center_exists: breakout.new_center_exists,
        c3_new_center_breakout_ok: breakout.new_center_breakout_ok,
    }
}

/// 准入门凭据构造（二通道分派，codex §6-2）：区间套优先，None 域 Type2/3 落小转大通道。
///
/// - `build_nest_certificate` Some ⟹ `Nest`（区间套通道，bit-exact 不动）。
/// - Nest None + 执行段存在 + Type2/3 ⟹ **小转大**（base gate false = descend anchor None）⟹ `Xzd`。
/// - Nest None + 无执行段（case-1 无定位候选）或 Type1 背驰失败 ⟹ `None`（门拒，旧语义保留）。
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
) -> Option<GateCertificate> {
    if let Some(cert) = build_nest_certificate(tower, lvl, source_index, delta, bits, hist) {
        return Some(GateCertificate::Nest(cert));
    }
    // Nest None：区分小转大（Type2/3 base gate false）与 case-1（无执行段）/Type1 背驰失败。
    let exec_moves = tower.get(lvl)?.as_slice();
    let s = &exec_moves[find_move_by_end_index(exec_moves, source_index)?]; // None=无定位候选 ⟹ 门拒
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
            Some(GateCertificate::Xzd(xiaozhuanda_confirm(
                s,
                source_index,
                confirm_index,
                lvl,
                delta,
                bsp_of_level,
                sub_centers,
                sub_bsp,
                sub_moves,
            )))
        }
    }
}

// σ_higher_at 已上移 selector.rs（codex-q1 G2 单一来源）：z 构造（第 9 维）与信号分解
// （SignalDecomp.sigma_higher，666 号）共用同一函数，防训练/查询口径分叉。本文件顶部导入消费。

#[cfg(test)]
mod tests {
    use super::*;

    // ── 小转大通道阶段2（xiaozhuanda）：C2 跨条目 / C3 as-of 最后次级中枢 / gate_pass 二通道 ──

    fn xzd_bsp(source_index: usize, bits: BspBits, center: Option<Center>) -> BspPoint {
        BspPoint { source_index, bits, pivot_low: 0, pivot_high: 0, center, struct_break_dir: None, force: None }
    }

    fn xzd_center(zd: i64, zg: i64, s: usize, e: usize) -> Center {
        Center { zd, zg, dd: zd, gg: zg, start_index: s, end_index: e }
    }

    fn xzd_seg(s: usize, e: usize) -> LeveledMove {
        use super::super::super::classifier::recursive_tower::ElementId;
        use super::super::super::classifier::descend::RMove;
        LeveledMove {
            rmove: RMove::Segment { direction: super::super::super::types::Direction::Down, lo: 0, hi: 100 },
            start_index: s,
            end_index: e,
            sub_moves: Rc::new(vec![]),
            id: ElementId { level: 1, ordinal: 0 },
        }
    }

    /// 次级走势，配置 hi（C3 突破判据测试用：`m.rmove.hi() > z.zg` 是否成立由此控制）。
    fn xzd_seg_hi(s: usize, e: usize, hi: i64) -> LeveledMove {
        use super::super::super::classifier::recursive_tower::ElementId;
        use super::super::super::classifier::descend::RMove;
        LeveledMove {
            rmove: RMove::Segment { direction: super::super::super::types::Direction::Up, lo: 0, hi },
            start_index: s,
            end_index: e,
            sub_moves: Rc::new(vec![]),
            id: ElementId { level: 1, ordinal: 0 },
        }
    }

    /// ★#100 问题① 验收（分歧案例，ChatGPT 裁决）：J0⊂J1⊂J2 但 end(J1)≠source(J0)——旧「端点
    /// 相等」口径（`find_move_by_end_index`，end==src）定位不到上级 rung；#77 现「区间包含」口径
    /// （`build_nest_certificate` line 812-813，start≤src≤end）正确定位。补 #77 探针只测两口径**等价**、
    /// 未测**分歧案例上新口径正确**的缺口。
    #[test]
    fn nest_containment_locates_rung_where_endpoint_equality_fails() {
        use super::super::super::classifier::recursive_tower::find_move_by_end_index;
        let src = 50usize;
        // 三层塔：J0(exec,end==src) ⊂ J1(含 src,end 80≠50) ⊂ J2(含 src,end 100≠50)。
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![xzd_seg(40, 50)]),  // tower[0]=J0 执行级：end==src
            Rc::new(vec![xzd_seg(20, 80)]),  // tower[1]=J1：含 50，end 80≠50
            Rc::new(vec![xzd_seg(0, 100)]),  // tower[2]=J2：含 50，end 100≠50
        ];
        let hist: Vec<f64> = vec![];
        let bits = BspBits { buy1: true, ..Default::default() };

        // 旧「端点相等」口径：上级 tower[1]/tower[2] 无 end==src 段 ⟹ 定位失败（旧会漏掉 J1/J2）。
        assert!(find_move_by_end_index(&tower[1], src).is_none(), "J1 end 80≠src 50，端点相等应定位失败");
        assert!(find_move_by_end_index(&tower[2], src).is_none(), "J2 end 100≠src 50，端点相等应定位失败");

        // 新「区间包含」口径：build_nest_certificate 定位到两级 rung，三层嵌套 J0⊂J1⊂J2 可见。
        let cert = build_nest_certificate(&tower, 0, src, Side::Long, &bits, &hist)
            .expect("区间包含口径应定位到执行级段");
        assert_eq!(cert.rungs.len(), 2, "含 src 的两上级 rung 均被区间包含口径定位（端点相等口径为 0）");
        // rungs 从高到低：rungs[0]=J2[0,100]、rungs[1]=J1[20,80]，且 end≠src（分歧标记）。
        assert_eq!((cert.rungs[0].interval.start_time, cert.rungs[0].interval.end_time), (0, 100));
        assert_eq!((cert.rungs[1].interval.start_time, cert.rungs[1].interval.end_time), (20, 80));
        assert_ne!(cert.rungs[0].interval.end_time, src as u64);
        assert_ne!(cert.rungs[1].interval.end_time, src as u64);
        // 嵌套链 J0⊆J1⊆J2（区间包含口径下才可见的三层套）——同时验证 build_nest_certificate 内看守放行。
        assert!(is_sub(&cert.base_interval, &cert.rungs[1].interval), "J0⊆J1");
        assert!(is_sub(&cert.rungs[1].interval, &cert.rungs[0].interval), "J1⊆J2");
    }

    /// ★#100 问题① 验收（边界等号 source==end(m)）：上级 rung 的 end 恰等于 source_index——
    /// 旧「端点相等」（`find_move_by_end_index`）与新「区间包含」（`build_nest_certificate`）口径
    /// **定位到同一段**（新口径是旧口径的真扩展，边界重合处一致，非分歧）。
    #[test]
    fn nest_boundary_source_equals_end_both_criteria_agree() {
        use super::super::super::classifier::recursive_tower::find_move_by_end_index;
        let src = 50usize;
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![xzd_seg(40, 50)]),  // exec：end==src
            Rc::new(vec![xzd_seg(10, 50)]),  // J1：end==src（边界等号）
        ];
        let hist: Vec<f64> = vec![];
        let bits = BspBits { buy1: true, ..Default::default() };
        // 旧端点相等口径定位到 tower[1] move[0]（end==src）。
        assert_eq!(find_move_by_end_index(&tower[1], src), Some(0));
        // 新区间包含口径定位到同一段——rung interval == tower[1][0]，且 end_time==src（与旧一致）。
        let cert = build_nest_certificate(&tower, 0, src, Side::Long, &bits, &hist).expect("定位成功");
        assert_eq!(cert.rungs.len(), 1);
        assert_eq!((cert.rungs[0].interval.start_time, cert.rungs[0].interval.end_time), (10, 50));
        assert_eq!(cert.rungs[0].interval.end_time, src as u64, "边界等号：新口径与旧端点相等一致");
    }

    /// C2 跨条目（codex §6-1）：Type3-only 信号自身无 buy2，须在同级列表查共生 B2 条目。
    #[test]
    fn xzd_c2_cross_entry_finds_cobsp_second() {
        let src = 42;
        // 同级列表：Type3-only 信号条目 + 独立 B2 条目（同 source_index，分离提取）。
        let list = vec![
            xzd_bsp(src, BspBits { buy3: true, ..Default::default() }, Some(xzd_center(10, 20, 0, 30))),
            xzd_bsp(src, BspBits { buy2: true, ..Default::default() }, None),
        ];
        assert!(xzd_type2_confirmed(&list, src, Side::Long), "跨条目查到共生 B2 ⟹ C2 成立");
        // 无共生 B2（另一 source_index 的 B2 不算）⟹ C2 假。
        let list2 = vec![
            xzd_bsp(src, BspBits { buy3: true, ..Default::default() }, None),
            xzd_bsp(src + 1, BspBits { buy2: true, ..Default::default() }, None),
        ];
        assert!(!xzd_type2_confirmed(&list2, src, Side::Long), "无同 source_index B2 ⟹ C2 假");
        // 侧向匹配：Long 只看 buy2，卖侧 sell2 不误命中。
        let list3 = vec![xzd_bsp(src, BspBits { sell2: true, ..Default::default() }, None)];
        assert!(!xzd_type2_confirmed(&list3, src, Side::Long), "sell2 不满足 Long 的 C2");
        assert!(xzd_type2_confirmed(&list3, src, Side::Short), "sell2 满足 Short 的 C2");
    }

    /// C3 as-of（codex §6-3/思维导图 128）：仅当**最后一个**次级中枢出现三类点才成立。
    #[test]
    fn xzd_c3_last_sublevel_zs_type3_only() {
        let s = xzd_seg(0, 100); // 本级走势跨度 [0,100]
        // 两个次级中枢：早 [10,30]、晚 [60,90]（最后中枢=[60,90]）。
        let centers = vec![xzd_center(10, 20, 10, 30), xzd_center(30, 40, 60, 90)];
        // 三类点挂在最后中枢 [60,90] ⟹ C3 成立。
        let bsp_last = vec![xzd_bsp(88, BspBits { buy3: true, ..Default::default() }, Some(xzd_center(30, 40, 60, 90)))];
        assert!(xzd_sub_last_zs_type3(&s, &centers, &bsp_last, Side::Long), "最后次级中枢出现三类点 ⟹ C3");
        // 三类点挂在**早**中枢 [10,30]（非最后）⟹ C3 假（no-patch：不接受非最后中枢的三类点）。
        let bsp_early = vec![xzd_bsp(28, BspBits { buy3: true, ..Default::default() }, Some(xzd_center(10, 20, 10, 30)))];
        assert!(!xzd_sub_last_zs_type3(&s, &centers, &bsp_early, Side::Long), "非最后中枢的三类点 ⟹ C3 假");
        // s 内无次级中枢 ⟹ C3 假。
        assert!(!xzd_sub_last_zs_type3(&s, &[], &bsp_last, Side::Long), "无次级中枢 ⟹ C3 假");
    }

    /// C3 新判据（codex #44(c)）：新中枢存在+突破 / 存在未突破 / 无新中枢 三态。
    #[test]
    fn xzd_c3_new_center_breakout_cases() {
        let source_index = 50;
        let confirm_index = 100;
        // 新中枢 [55,70]（start>=source_index, end<=confirm_index），其后走势 [71,90] 上破 ZG=20。
        let centers = vec![xzd_center(10, 20, 55, 70)];
        let moves_break = vec![xzd_seg_hi(71, 90, 25)]; // hi=25 > zg=20 ⟹ 突破
        let diag = xzd_c3_new_center_breakout(source_index, confirm_index, Side::Long, &centers, &moves_break);
        assert!(diag.new_center_exists, "新中枢在 [source_index,confirm_index] 区间内 ⟹ 存在");
        assert!(diag.new_center_breakout_ok, "其后走势 hi>zg ⟹ 突破成立");

        // 中枢存在但其后走势未突破（hi=15 <= zg=20）。
        let moves_no_break = vec![xzd_seg_hi(71, 90, 15)];
        let diag2 = xzd_c3_new_center_breakout(source_index, confirm_index, Side::Long, &centers, &moves_no_break);
        assert!(diag2.new_center_exists, "新中枢仍存在");
        assert!(!diag2.new_center_breakout_ok, "未破 ZG ⟹ 突破假");

        // 无新中枢（中枢 start_index < source_index，不满足 off-by-one 硬约束）。
        let centers_old = vec![xzd_center(10, 20, 40, 45)];
        let diag3 = xzd_c3_new_center_breakout(source_index, confirm_index, Side::Long, &centers_old, &moves_break);
        assert!(!diag3.new_center_exists, "中枢 start_index<source_index ⟹ 非新中枢");
        assert!(!diag3.new_center_breakout_ok, "无新中枢 ⟹ 突破假");

        // off-by-one：start_index == source_index 仍算「source 之后已确认」（硬约束用 >=，非 >）。
        let centers_eq = vec![xzd_center(10, 20, source_index, 70)];
        let diag4 = xzd_c3_new_center_breakout(source_index, confirm_index, Side::Long, &centers_eq, &moves_break);
        assert!(diag4.new_center_exists, "start_index==source_index ⟹ 新中枢存在（>= 非 >）");
    }

    /// gate_pass（codex #44(c) 终局裁定）：level==1 时 C2∧C3(突破) 硬门；level!=1 维持 C2-only。
    #[test]
    fn xzd_gate_pass_level1_c3_breakout_hard_gate() {
        let mk = |level, c2, c3_breakout| XzdEvidence {
            source_index: 1, level, side: Side::Long, confirm_index: 5,
            type2_confirmed: c2, sub_last_zs_type3: false,
            last_zs_exists: false, same_side_l0_type3_any: false,
            same_center_any: false, same_side_causal_ok: false, sub_bsp_type3_count: 0,
            c3_new_center_exists: c3_breakout, c3_new_center_breakout_ok: c3_breakout,
        };
        // level==1：C2∧C3 硬门。
        assert!(mk(1, true, true).gate_pass(), "level1 C2 真∧C3 真 ⟹ 通过");
        assert!(!mk(1, true, false).gate_pass(), "level1 C2 真、C3 假 ⟹ 拒（C3 已是硬门参门项）");
        assert!(!mk(1, false, true).gate_pass(), "level1 C2 假 ⟹ 仍拒");
        // level!=1：C2-only（既定 #41 裁定不变）。
        assert!(mk(2, true, false).gate_pass(), "level2 C2 真、C3 假 ⟹ 仍通过（lvl>=2 维持 C2-only）");
        assert!(!mk(2, false, true).gate_pass(), "level2 C2 假 ⟹ 拒");
    }

    // ── P7 正规出场口径测试（RED→GREEN：exit_decision_from_bits 派生，接 sell.rs CloseRoot/ReduceCore）────

    /// **P7 多头入场 exit_decision_from_bits：sell1 → CloseRoot（优先级最高）**。
    ///
    /// 定义依据：recog_chanlun_sell 第一分支「sell1=顶背驰 → CloseRoot」。
    /// 边界条件：sell1=1 即触发，不看 sell3/sell2（第一类优先）。
    #[test]
    fn p7_exit_decision_long_sell1_close_root() {
        // bit3=sell1
        let exit_class: u8 = 1 << 3;
        assert_eq!(exit_decision_from_bits(exit_class, 1), ExitDecision::CloseRoot,
            "多头入场 sell1 → CloseRoot");
    }

    /// **P7 多头入场 exit_decision_from_bits：sell3（无 sell1）→ ReduceCore**。
    ///
    /// 定义依据：recog_chanlun_sell 第二分支「sell3=第三类 → ReduceCore」（sell1=false 前提）。
    #[test]
    fn p7_exit_decision_long_sell3_reduce_core() {
        // bit5=sell3，bit3=sell1=0
        let exit_class: u8 = 1 << 5;
        assert_eq!(exit_decision_from_bits(exit_class, 1), ExitDecision::ReduceCore,
            "多头入场 sell3（无 sell1）→ ReduceCore");
    }

    /// **P7 多头入场 exit_decision_from_bits：sell1+sell3 同时 → CloseRoot（第一类优先）**。
    ///
    /// 边界条件：sell1 与 sell3 共存时，第一类优先（镜像 recog_chanlun_sell 分支顺序）。
    #[test]
    fn p7_exit_decision_long_sell1_sell3_priority() {
        let exit_class: u8 = (1 << 3) | (1 << 5); // sell1+sell3
        assert_eq!(exit_decision_from_bits(exit_class, 1), ExitDecision::CloseRoot,
            "sell1+sell3 共存 → CloseRoot（第一类优先）");
    }

    /// **P7 多头入场 exit_decision_from_bits：sell2（无 sell1/3）→ Type2Missing（still-MISSING 诚实标注）**。
    ///
    /// 定义依据：sell.rs:35 诚实边界「第二类卖点闭环 still-MISSING」。
    /// 边界条件：Type2Missing 不改账本，只记入统计（no-patch）。
    #[test]
    fn p7_exit_decision_long_sell2_type2_missing() {
        let exit_class: u8 = 1 << 4; // sell2
        assert_eq!(exit_decision_from_bits(exit_class, 1), ExitDecision::Type2Missing,
            "多头入场 sell2 → Type2Missing（sell.rs:35 still-MISSING）");
    }

    /// **P7 多头入场 exit_decision_from_bits：bsp_class=0（无任何 bit）→ Hold**。
    ///
    /// 边界条件：exit 信号无正规卖点 bit → Hold（无出场依据，不改账本）。
    #[test]
    fn p7_exit_decision_long_no_bit_hold() {
        assert_eq!(exit_decision_from_bits(0, 1), ExitDecision::Hold,
            "多头入场 bsp_class=0 → Hold");
    }

    /// **P7 空头入场 exit_decision_from_bits：买侧镜像判据**。
    ///
    /// 定义依据：空头入场 exit 是 Long 信号，buy1/3/2 镜像 sell1/3/2。
    #[test]
    fn p7_exit_decision_short_buy_mirror() {
        // buy1 → CloseRoot
        assert_eq!(exit_decision_from_bits(1 << 0, -1), ExitDecision::CloseRoot,
            "空头入场 buy1 → CloseRoot");
        // buy3 → ReduceCore
        assert_eq!(exit_decision_from_bits(1 << 2, -1), ExitDecision::ReduceCore,
            "空头入场 buy3 → ReduceCore");
        // buy2 → Type2Missing
        assert_eq!(exit_decision_from_bits(1 << 1, -1), ExitDecision::Type2Missing,
            "空头入场 buy2 → Type2Missing");
        // 0 → Hold
        assert_eq!(exit_decision_from_bits(0, -1), ExitDecision::Hold,
            "空头入场 bsp_class=0 → Hold");
    }

    /// **P7 ExitDecision 穷举分类完整性（多头）**：所有可能的 exit bsp_class（0..64）都映射到四态之一。
    ///
    /// 边界条件：完整覆盖 64 种 bsp_class，无 panic 无未定义。
    #[test]
    fn p7_exit_decision_exhaustive_long() {
        for cls in 0u8..64 {
            let d = exit_decision_from_bits(cls, 1);
            assert!(matches!(d,
                ExitDecision::CloseRoot | ExitDecision::ReduceCore |
                ExitDecision::Type2Missing | ExitDecision::Hold
            ), "bsp_class={cls:#04x} delta=+1 应映射到四态之一");
        }
    }

    /// **P7 SpreadAttribution 统计字段完整性（L1）**：SignalDecomp 含 exit_decision 字段，
    /// 四种出场决策可加入 agg 统计。
    #[test]
    fn p7_spread_attribution_exit_decision_fields() {
        let mut agg = SpreadAttribution::default();
        // 验证四个统计字段可正常累加
        agg.n_exit_close_root += 1;
        agg.n_exit_reduce_core += 2;
        agg.n_exit_type2_missing += 3;
        agg.n_exit_hold += 4;
        assert_eq!(agg.n_exit_close_root, 1);
        assert_eq!(agg.n_exit_reduce_core, 2);
        assert_eq!(agg.n_exit_type2_missing, 3);
        assert_eq!(agg.n_exit_hold, 4);
        // 验证 SignalDecomp 含 exit_decision 字段
        let d = SignalDecomp {
            entry_bar: 0, exit_bar: 1, level: 0, delta: 1, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0,
            actual_pnl: 0.0, sigma_higher: 0, bsp_class: 1 << 3, // sell1
            z: MuClass::from_certificate(0, 1, BspBits::from_class_index(1 << 3), 0, PositionState::Root),
            exit_decision: ExitDecision::CloseRoot,
            trigger: NestTrigger::Type1TrendDivergence,
        };
        assert_eq!(d.exit_decision, ExitDecision::CloseRoot,
            "SignalDecomp.exit_decision 字段可读写（P7 接口存在）");
    }

    // ── 生产路径测试（RED→GREEN：W1 返工，多级 N^δ 递归真调用） ─────────────────

    /// **生产路径：build_multilevel_nest_cert 存在且可调用（RED：函数不存在则编译失败）**。
    ///
    /// 验证 `build_multilevel_nest_cert` 函数签名正确：接受 tower + lvl + source_index +
    /// delta + BspBits + macd_hist，返回 bool。基例（lvl=0，rungs 空，纯 Conf^δ_e）。
    ///
    /// **认识论 L1**（生产路径结构验证，非 L0 fixture 同义反复）：函数存在 + 接口正确。
    #[test]
    fn multilevel_nest_cert_function_exists() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::LeveledMove;
        use super::super::super::types::{BspBits, Side};

        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![];
        let mut bits = BspBits::default();
        bits.buy1 = true;
        let hist: Vec<f64> = vec![];
        // 空塔 → lvl=0 无法从塔拿区间 → false（无上级语境）。
        // 函数本身必须存在（否则 RED）。
        let result = build_multilevel_nest_cert(&tower, 0, 0, Side::Long, &bits, &hist);
        // 空塔 ⟹ false（无上级 context，N^δ 不成立）。
        assert!(!result, "空塔 ⟹ false（基例无 confirm 来源）");
    }

    /// **生产路径：多级 rungs 链（≥2 级），J 嵌套收缩验证（RED→GREEN 核心测试）**。
    ///
    /// 构造合成 2 级塔：L0 有 4 段（up/down/up/down），L1 有 1 个 Compose 包含全部 4 段。
    /// 信号在 L1（lvl=1），source_index=19（最后 Down 段的 end_index），δ=Long。
    /// 验证 `build_multilevel_nest_cert` 真调用 `n_delta()`（≥1 rung，非单级 bsp_div_cand）。
    ///
    /// **认识论 L1**：合成塔 + 合成 hist（条件4 力度满足），验证多级构造路径正确，
    /// 非 alpha 声明（L2/L3 待 W-VERIFY）。
    #[test]
    fn multilevel_nest_cert_two_level_rung_chain() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{BspBits, Center, Direction, Side};

        // helper：构造 L0 Segment LeveledMove
        let seg = |dir: Direction, lo: i64, hi: i64, s: usize, e: usize, ord: u64| -> LeveledMove {
            LeveledMove {
                rmove: RMove::Segment { direction: dir, lo, hi },
                start_index: s, end_index: e,
                sub_moves: Rc::new(vec![]),
                id: ElementId { level: 0, ordinal: ord },
            }
        };

        // L0 四段：up(0-4) / down(5-9) / up(10-14) / down(15-19)
        let s0 = seg(Direction::Up,   50, 100,  0,  4, 0);
        let s1 = seg(Direction::Down, 40,  90,  5,  9, 1); // s'：lo=40
        let s2 = seg(Direction::Up,   45,  95, 10, 14, 2);
        let s3 = seg(Direction::Down, 30,  85, 15, 19, 3); // s：lo=30 < 40，Extreme ✓

        // L1 Compose 包含全部4段
        let sub_rmoves: Vec<RMove> = vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone(), s3.rmove.clone()];
        let lo = 30i64; let hi = 100i64;
        let parent = LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Center { zd: lo, zg: hi, dd: lo, gg: hi, start_index: 0, end_index: 19 }],
                level: 1,
            },
            start_index: 0, end_index: 19,
            sub_moves: Rc::new(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()]),
            id: ElementId { level: 1, ordinal: 0 },
        };

        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),   // level 0
            Rc::new(vec![parent]),            // level 1
        ];

        // hist：s'(5-9) area=5*2=10，s(15-19) area=5*1=5 < 10（条件4 Weak ✓）
        let hist: Vec<f64> = (0..20usize).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();

        // 买侧 bits（基例 Conf^+）：buy1=true
        let mut bits = BspBits::default();
        bits.buy1 = true;

        // lvl=0，source_index=19（s3 的 end_index），δ=Long
        // 期望：L0 基例 Conf^+ = bits.conf_plus() = true（buy1=true）
        // 且 bsp_div_cand 四条件满足（Down，Extreme lo=30<40，Weak area=5<10）
        // ⟹ build_multilevel_nest_cert ⟹ n_delta() = true
        let result = build_multilevel_nest_cert(&tower, 0, 19, Side::Long, &bits, &hist);
        assert!(result, "两级塔 + 四条件满足 + Conf^+ ⟹ n_delta()=true（多级链 J 嵌套收缩）");
    }

    // ── 673-fix：接口级三分拆独立单元测试（Type1/2/3 分派 + Type2/3 谓词） ──────────

    /// **bsp_cand_type 优先级分派（Type1>Type2>Type3，保持旧 is_type1 语义）**。
    #[test]
    fn bsp_cand_type_priority_dispatch() {
        use super::super::super::types::{BspBits, Side};
        let mk = |b1, b2, b3| {
            let mut bits = BspBits::default();
            bits.buy1 = b1; bits.buy2 = b2; bits.buy3 = b3;
            bits
        };
        // buy1 置位 ⟹ Type1（即使 buy2/buy3 同置，非互斥 P4§5，优先级坍缩）。
        assert_eq!(bsp_cand_type(&mk(true, true, true), Side::Long), BspCandType::Type1);
        assert_eq!(bsp_cand_type(&mk(false, true, true), Side::Long), BspCandType::Type2);
        assert_eq!(bsp_cand_type(&mk(false, false, true), Side::Long), BspCandType::Type3);
        // 卖侧对称。
        let mut sb = BspBits::default(); sb.sell2 = true;
        assert_eq!(bsp_cand_type(&sb, Side::Short), BspCandType::Type2);
    }

    /// **StructBreak 分派（codex 终局裁决A）：六 bit 全零 ⟹ StructBreak，优先于 Type1>2>3**。
    ///
    /// 真三类候选（buy3/sell3 置位，class_index()!=0）不受影响，仍走 Type3——
    /// StructBreak 判别只截获零 bit 候选，不改变既有三类分派逻辑。
    #[test]
    fn bsp_cand_type_structbreak_zero_bits_dispatch() {
        use super::super::super::types::{BspBits, Side};
        assert_eq!(bsp_cand_type(&BspBits::default(), Side::Long), BspCandType::StructBreak);
        assert_eq!(bsp_cand_type(&BspBits::default(), Side::Short), BspCandType::StructBreak);

        let mut b3 = BspBits::default(); b3.buy3 = true;
        assert_eq!(bsp_cand_type(&b3, Side::Long), BspCandType::Type3, "真三类 buy3 置位仍走 Type3，不受 StructBreak 影响");
        let mut s3 = BspBits::default(); s3.sell3 = true;
        assert_eq!(bsp_cand_type(&s3, Side::Short), BspCandType::Type3, "真三类 sell3 置位仍走 Type3，不受 StructBreak 影响");
    }

    /// **build_gate_certificate 护栏（codex 终局裁决A）：零 bit 候选恒 None（门拒）**。
    ///
    /// lvl==0 场景下 Type2/Type3 本有存在性免门（自由通道），但 StructBreak 候选即使在 lvl==0
    /// 也恒被 `cand_delta_base_gate` 拒绝——证明 StructBreak 不会被免门通道误纳。
    /// 对照组：同一 fixture 换成真三类 bits（buy3=true）时，lvl==0 正常放行 ⟹ Some（回归保护，
    /// 证明本次改动只截获零 bit 候选，不影响真三类通路）。
    #[test]
    fn build_gate_certificate_structbreak_zero_bits_always_none() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{BspBits, Side};

        let s = LeveledMove {
            rmove: RMove::Segment { direction: super::super::super::types::Direction::Down, lo: 30, hi: 85 },
            start_index: 15, end_index: 19,
            sub_moves: Rc::new(vec![]),
            id: ElementId { level: 0, ordinal: 0 },
        };
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![Rc::new(vec![s])];
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();

        // 零 bit（StructBreak）：即使 lvl==0（Type2/3 免门场景）也恒 None。
        let zero_bits = BspBits::default();
        assert!(
            build_gate_certificate(&tower, 0, 19, Side::Long, &zero_bits, &hist, 19, &[], &[], &[]).is_none(),
            "零 bit 破中枢未背驰候选（StructBreak）恒门拒，不复用 Type3 lvl==0 免门通道"
        );

        // 对照：真三类（buy3 置位，class_index()!=0）同 fixture 下 lvl==0 正常放行 ⟹ Some。
        let mut real_type3 = BspBits::default();
        real_type3.buy3 = true;
        assert!(
            build_gate_certificate(&tower, 0, 19, Side::Long, &real_type3, &hist, 19, &[], &[], &[]).is_some(),
            "真三类 buy3 置位候选不受 StructBreak 分流影响，仍走 Type3 正常放行"
        );
    }

    /// **Type2/Type3 base gate：lvl==0 存在性免门；lvl>=1 无次级别锚 ⟹ 小转大门拒**。
    ///
    /// Type2/3 谓词是**独立函数**（codex 裁决①），但当前存在性锚共用 `descend_type1_anchor_depth`
    /// （codex 允许复用反向走势完成 helper），故对同一 fixture 行为一致——语义分离体现于保护边界
    /// 归属（Type2=一类点极值 / Type3=中枢 ZG/ZD），记录于各 docstring，非本级信号集差。
    #[test]
    fn cand_delta_type23_base_gate_level0_and_small_to_big() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::Side;

        // 递归底段（无 sub_moves）：lvl==0 免门 ⟹ true；lvl>=1 无锚 ⟹ 小转大 false。
        let s = LeveledMove {
            rmove: RMove::Segment { direction: super::super::super::types::Direction::Down, lo: 30, hi: 85 },
            start_index: 15, end_index: 19,
            sub_moves: Rc::new(vec![]),
            id: ElementId { level: 0, ordinal: 3 },
        };
        let hist: Vec<f64> = (0..20).map(|_| 1.0).collect();

        // lvl==0：存在性免门 ⟹ Type2/Type3 均 true。
        assert!(cand_delta_type2_completion(&s, 19, Side::Long, &hist, 0));
        assert!(cand_delta_type3_retest(&s, 19, Side::Long, &hist, 0));
        // lvl>=1 但 s.sub_moves 空（无次级别 Type1 锚）⟹ 小转大 ⟹ false。
        assert!(!cand_delta_type2_completion(&s, 19, Side::Long, &hist, 1));
        assert!(!cand_delta_type3_retest(&s, 19, Side::Long, &hist, 1));
        // dispatcher：base gate Type1 恒 true（判据在 per-rung），Type2/3 委托上述。
        assert!(cand_delta_base_gate(BspCandType::Type1, &s, 19, Side::Long, &hist, 1));
        assert!(!cand_delta_base_gate(BspCandType::Type2, &s, 19, Side::Long, &hist, 1));
    }

    /// **cand_delta per-rung dispatcher：Type1→div_cand，Type2/3→true（存在性已 base 门控）**。
    #[test]
    fn cand_delta_rung_dispatch() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{Direction, Side};
        let seg = |dir, lo, hi, s, e, ord| LeveledMove {
            rmove: RMove::Segment { direction: dir, lo, hi },
            start_index: s, end_index: e, sub_moves: Rc::new(vec![]),
            id: ElementId { level: 0, ordinal: ord },
        };
        // rung 次级别序列：s'(down lo=40) ... s(down lo=30<40 Extreme✓)，hist 力度衰减 Weak✓。
        let subs = vec![
            seg(Direction::Down, 40, 90, 5, 9, 1),
            seg(Direction::Up, 45, 95, 10, 14, 2),
            seg(Direction::Down, 30, 85, 15, 19, 3),
        ];
        let hist: Vec<f64> = (0..20usize).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        // Type1 → div_cand（四条件满足 ⟹ true）。
        assert!(cand_delta(BspCandType::Type1, &subs, 19, Side::Long, &hist));
        // Type2/Type3 → true（per-rung 存在性已 base 门控，rung 载上级语境）。
        assert!(cand_delta(BspCandType::Type2, &subs, 19, Side::Long, &hist));
        assert!(cand_delta(BspCandType::Type3, &subs, 19, Side::Long, &hist));
        // Type1 无对齐候选段（source_index 不存在）⟹ false。
        assert!(!cand_delta(BspCandType::Type1, &subs, 99, Side::Long, &hist));
    }

    /// **Cand=false ⟹ N^δ 整体=false（Cand 传播测试）**。
    ///
    /// 同塔结构，但 hist=0（条件4 area=0 < 0 = false ⟹ Cand=false）。
    /// 验证 build_multilevel_nest_cert 真走 N^δ 路径（任一级 Cand=0 ⟹ 整体 0）。
    #[test]
    fn multilevel_nest_cert_cand_false_propagates_zero() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{BspBits, Center, Direction, Side};

        let seg = |dir: Direction, lo: i64, hi: i64, s: usize, e: usize, ord: u64| -> LeveledMove {
            LeveledMove {
                rmove: RMove::Segment { direction: dir, lo, hi },
                start_index: s, end_index: e,
                sub_moves: Rc::new(vec![]),
                id: ElementId { level: 0, ordinal: ord },
            }
        };
        let s0 = seg(Direction::Up,   50, 100,  0,  4, 0);
        let s1 = seg(Direction::Down, 40,  90,  5,  9, 1);
        let s2 = seg(Direction::Up,   45,  95, 10, 14, 2);
        let s3 = seg(Direction::Down, 30,  85, 15, 19, 3);
        let sub_rmoves: Vec<RMove> = vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone(), s3.rmove.clone()];
        let lo = 30i64; let hi = 100i64;
        let parent = LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Center { zd: lo, zg: hi, dd: lo, gg: hi, start_index: 0, end_index: 19 }],
                level: 1,
            },
            start_index: 0, end_index: 19,
            sub_moves: Rc::new(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()]),
            id: ElementId { level: 1, ordinal: 0 },
        };
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),
            Rc::new(vec![parent]),
        ];
        // hist 全 0 ⟹ area=0 ⟹ 条件4 0 < 0 = false ⟹ Cand=false ⟹ n_delta()=false
        let hist = vec![0.0f64; 20];
        let mut bits = BspBits::default();
        bits.buy1 = true;
        let result = build_multilevel_nest_cert(&tower, 0, 19, Side::Long, &bits, &hist);
        assert!(!result, "Cand=false（hist=0）⟹ n_delta()=false（Cand 传播路径）");
    }

    /// **673 号段1：Type2 存在性免本级背驰段门**。
    ///
    /// 与 `multilevel_nest_cert_cand_false_propagates_zero` 同塔（hist=0 ⟹ div_cand 条件4 false），
    /// 对照两类信号：Type1（buy1）div_cand 守门 ⟹ false（bit-exact 不动）；Type2（buy2）存在性由
    /// 结构分类前提保证（第17课L60 完备性），不经本级背驰段谓词 ⟹ 免门 ⟹ n_delta()=true。
    #[test]
    fn multilevel_nest_cert_type2_bypasses_divergence_gate() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{BspBits, Center, Direction, Side};

        let seg = |dir: Direction, lo: i64, hi: i64, s: usize, e: usize, ord: u64| -> LeveledMove {
            LeveledMove {
                rmove: RMove::Segment { direction: dir, lo, hi },
                start_index: s, end_index: e,
                sub_moves: Rc::new(vec![]),
                id: ElementId { level: 0, ordinal: ord },
            }
        };
        let s0 = seg(Direction::Up,   50, 100,  0,  4, 0);
        let s1 = seg(Direction::Down, 40,  90,  5,  9, 1);
        let s2 = seg(Direction::Up,   45,  95, 10, 14, 2);
        let s3 = seg(Direction::Down, 30,  85, 15, 19, 3);
        let sub_rmoves: Vec<RMove> = vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone(), s3.rmove.clone()];
        let lo = 30i64; let hi = 100i64;
        let parent = LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Center { zd: lo, zg: hi, dd: lo, gg: hi, start_index: 0, end_index: 19 }],
                level: 1,
            },
            start_index: 0, end_index: 19,
            sub_moves: Rc::new(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()]),
            id: ElementId { level: 1, ordinal: 0 },
        };
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),
            Rc::new(vec![parent]),
        ];
        let hist = vec![0.0f64; 20]; // div_cand 条件4 area=0<0=false（Type1 会被守门）

        // Type1（buy1）：div_cand 守门不变 ⟹ false。
        let mut t1 = BspBits::default();
        t1.buy1 = true;
        assert!(!build_multilevel_nest_cert(&tower, 0, 19, Side::Long, &t1, &hist),
            "Type1 div_cand 条件4 false ⟹ n_delta()=false（本级背驰段门守 Type1 bit-exact）");

        // Type2（buy2）：存在性免本级背驰段门 ⟹ true。
        let mut t2 = BspBits::default();
        t2.buy2 = true;
        assert!(build_multilevel_nest_cert(&tower, 0, 19, Side::Long, &t2, &hist),
            "Type2 存在性免本级背驰段门（673 段1）⟹ n_delta()=true");
    }

    /// **方向 dir=−δ 反测试**：信号 δ=Long 但 source_index 指向 Up 段（dir 不反）⟹ false。
    #[test]
    fn multilevel_nest_cert_wrong_direction_returns_false() {
        use std::rc::Rc;
        use super::super::super::classifier::recursive_tower::{ElementId, LeveledMove};
        use super::super::super::classifier::descend::RMove;
        use super::super::super::types::{BspBits, Center, Direction, Side};

        let seg = |dir: Direction, lo: i64, hi: i64, s: usize, e: usize, ord: u64| -> LeveledMove {
            LeveledMove {
                rmove: RMove::Segment { direction: dir, lo, hi },
                start_index: s, end_index: e,
                sub_moves: Rc::new(vec![]),
                id: ElementId { level: 0, ordinal: ord },
            }
        };
        // 最后一段是 Up（非 Down），δ=Long 要求 Down → 条件1 失败
        let s0 = seg(Direction::Down, 30, 90,  0,  4, 0);
        let s1 = seg(Direction::Up,   40, 100,  5,  9, 1); // s'
        let s2 = seg(Direction::Down, 35,  95, 10, 14, 2);
        let s3 = seg(Direction::Up,   50, 110, 15, 19, 3); // s：Up，δ=Long 要求 Down → 失败
        let sub_rmoves: Vec<RMove> = vec![s0.rmove.clone(), s1.rmove.clone(), s2.rmove.clone(), s3.rmove.clone()];
        let lo = 30i64; let hi = 110i64;
        let parent = LeveledMove {
            rmove: RMove::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Center { zd: lo, zg: hi, dd: lo, gg: hi, start_index: 0, end_index: 19 }],
                level: 1,
            },
            start_index: 0, end_index: 19,
            sub_moves: Rc::new(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()]),
            id: ElementId { level: 1, ordinal: 0 },
        };
        let tower: Vec<Rc<Vec<LeveledMove>>> = vec![
            Rc::new(vec![s0, s1, s2, s3]),
            Rc::new(vec![parent]),
        ];
        let hist: Vec<f64> = (0..20).map(|_| 2.0).collect();
        let mut bits = BspBits::default();
        bits.buy1 = true;
        // s3 是 Up 段，δ=Long 要求 Down → 条件1 失败 → false
        let result = build_multilevel_nest_cert(&tower, 0, 19, Side::Long, &bits, &hist);
        assert!(!result, "dir(s)≠−δ（Up 段但 δ=Long）⟹ false（条件1 失败）");
    }

    // ── 673 号段2：定律一下沉锚定（Type2/3 精确定位锚次级别 Type1）─────────────────

    use super::super::super::classifier::recursive_tower::{ElementId as EId2, LeveledMove as LM2};
    use super::super::super::classifier::descend::RMove as RM2;
    use super::super::super::types::{Center as Ct2, Direction as Dir2};
    use std::rc::Rc as Rc2;

    fn seg2(dir: Dir2, lo: i64, hi: i64, s: usize, e: usize, ord: u64) -> LM2 {
        LM2 {
            rmove: RM2::Segment { direction: dir, lo, hi },
            start_index: s, end_index: e,
            sub_moves: Rc2::new(vec![]),
            id: EId2 { level: 0, ordinal: ord },
        }
    }

    fn compose2(subs: Vec<LM2>, level: u32, ord: u64) -> LM2 {
        let start = subs.first().map(|m| m.start_index).unwrap_or(0);
        let end = subs.last().map(|m| m.end_index).unwrap_or(0);
        let lo = subs.iter().map(|m| m.rmove.lo()).min().unwrap_or(0);
        let hi = subs.iter().map(|m| m.rmove.hi()).max().unwrap_or(0);
        let sub_rmoves: Vec<RM2> = subs.iter().map(|m| m.rmove.clone()).collect();
        LM2 {
            rmove: RM2::Compose {
                subs: Rc::new(sub_rmoves),
                centers: vec![Ct2 { zd: lo, zg: hi, dd: lo, gg: hi, start_index: start, end_index: end }],
                level,
            },
            start_index: start, end_index: end,
            sub_moves: Rc2::new(subs),
            id: EId2 { level, ordinal: ord },
        }
    }

    /// 段2 正例：Type2@lvl1 的回抽次级别走势 m2 内部含次级别 Type1 背驰段（end==source_index）
    /// ⟹ 下沉锚定 Some(1) ⟹ 证书非 None ⟹ n_delta=true（存在性 buy2 基例 + 精确定位已门控）。
    #[test]
    fn nest_cert_type2_sublevel_type1_anchor_passes() {
        // m2（level1 回抽走势）的次级别（level0）：up/down(s')/up/down(s@19)，趋势背驰。
        let s0 = seg2(Dir2::Up,   50, 100,  0,  4, 0);
        let s1 = seg2(Dir2::Down, 40,  90,  5,  9, 1); // s'：Down lo=40
        let s2 = seg2(Dir2::Up,   45,  95, 10, 14, 2);
        let s3 = seg2(Dir2::Down, 30,  85, 15, 19, 3); // s：Down lo=30<40 Extreme，end=19
        let m2 = compose2(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);
        let tower: Vec<Rc2<Vec<LM2>>> = vec![
            Rc2::new(vec![s0, s1, s2, s3]),
            Rc2::new(vec![m2.clone()]),
        ];
        // s3(15-19) area=5*1=5 < s1(5-9) area=5*2=10（Weak ✓）。
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();

        assert_eq!(super::descend_type1_anchor_depth(&m2, 19, Side::Long, &hist), Some(1),
            "次级别 Type1 锚点成立 ⟹ 下沉深度 Some(1)");
        let mut buy2 = super::super::super::types::BspBits::default();
        buy2.buy2 = true;
        assert!(build_multilevel_nest_cert(&tower, 1, 19, Side::Long, &buy2, &hist),
            "Type2@lvl1 次级别锚点成立 ⟹ 证书非 None ⟹ n_delta=true");
    }

    /// 段2 小转大（可证伪判别）：同结构但次级别无一类背驰锚点（hist=0 ⟹ div_cand Weak 假）
    /// ⟹ 下沉锚定 None ⟹ 证书 None ⟹ 门拒（build_multilevel_nest_cert=false）。
    #[test]
    fn nest_cert_type2_no_sublevel_anchor_is_xiaozhuandaa_none() {
        let s0 = seg2(Dir2::Up,   50, 100,  0,  4, 0);
        let s1 = seg2(Dir2::Down, 40,  90,  5,  9, 1);
        let s2 = seg2(Dir2::Up,   45,  95, 10, 14, 2);
        let s3 = seg2(Dir2::Down, 30,  85, 15, 19, 3);
        let m2 = compose2(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);
        let tower: Vec<Rc2<Vec<LM2>>> = vec![
            Rc2::new(vec![s0, s1, s2, s3]),
            Rc2::new(vec![m2.clone()]),
        ];
        let hist = vec![0.0f64; 20]; // area=0 ⟹ 0<0 false ⟹ div_cand 假 ⟹ 次级别无一类锚点

        assert_eq!(super::descend_type1_anchor_depth(&m2, 19, Side::Long, &hist), None,
            "次级别无一类背驰锚点 ⟹ 小转大 ⟹ None（显式可测判别）");
        let mut buy2 = super::super::super::types::BspBits::default();
        buy2.buy2 = true;
        assert!(!build_multilevel_nest_cert(&tower, 1, 19, Side::Long, &buy2, &hist),
            "小转大（次级别无 Type1）⟹ 证书 None ⟹ 门拒");
    }

    /// 段2 真递归下沉：次级别 Type1 段本身内部再含次次级别 Type1 背驰段（end 同为 source_index）
    /// ⟹ 逐级收缩到最低可用级别，深度 Some(2)（不是只下沉一级就停）。
    #[test]
    fn descend_anchor_recurses_below_one_level() {
        // hist 递减：靠后 bar 力度更小（Weak 各级满足）。
        let hist: Vec<f64> = (0..20).map(|i| (20 - i) as f64).collect();
        // A0（level1 Down @0-9）：a0(Up)/a1(Down)。
        let a0 = seg2(Dir2::Up,   60, 100, 0, 4, 0);
        let a1 = seg2(Dir2::Down, 40,  95, 5, 9, 1);
        let big_a0 = compose2(vec![a0, a1], 1, 0); // lo=40
        // Amid（level1 Up @10-13）。
        let am0 = seg2(Dir2::Down, 45, 90, 10, 11, 2);
        let am1 = seg2(Dir2::Up,   50, 110, 12, 13, 3);
        let amid = compose2(vec![am0, am1], 1, 1);
        // A1（level1 Down @14-19，内部 level0 趋势背驰 @19）。
        let b0 = seg2(Dir2::Up,   48, 105, 14, 15, 4);
        let b1 = seg2(Dir2::Down, 42,  92, 16, 16, 5); // 次次级别 s' Down lo=42
        let b2 = seg2(Dir2::Up,   46,  96, 17, 17, 6);
        let b3 = seg2(Dir2::Down, 30,  85, 18, 19, 7); // 次次级别 s Down lo=30<42，end=19
        let big_a1 = compose2(vec![b0, b1, b2, b3], 1, 2); // lo=30
        // s（level2）：[A0(Down), Amid(Up), A1(Down@19)]。
        let s = compose2(vec![big_a0, amid, big_a1], 2, 0);

        assert_eq!(super::descend_type1_anchor_depth(&s, 19, Side::Long, &hist), Some(2),
            "次级别 Type1 + 次次级别 Type1 ⟹ 真递归下沉深度 Some(2)");
    }

    // ── Q4（task #145）：盘整背驰承接门 pan_div_gate_pass（Nest/XZD 二通道）─────────

    fn pan_cert(source_index: usize, side: Side) -> PanDivCert {
        PanDivCert {
            source_index,
            side,
            center: xzd_center(40, 50, 0, 4),
            seg_a: (5, 9),
            seg_c: (15, 19),
        }
    }

    /// Nest 通道正例：复用 `nest_cert_type2_sublevel_type1_anchor_passes` 的塔——m2@tower[1]
    /// 的次级别含 Type1 背驰锚（descend Some(1)）⟹ ∃e<ℓ Conf^δ_e ⟹ PanDiv 承接成立
    /// （collect_signals 据此组 RawSignal，trigger=PanDivConsolidation）。
    #[test]
    fn pan_div_gate_nest_channel_passes_via_sublevel_type1_anchor() {
        let s0 = seg2(Dir2::Up, 50, 100, 0, 4, 0);
        let s1 = seg2(Dir2::Down, 40, 90, 5, 9, 1);
        let s2 = seg2(Dir2::Up, 45, 95, 10, 14, 2);
        let s3 = seg2(Dir2::Down, 30, 85, 15, 19, 3);
        let m2 = compose2(vec![s0.clone(), s1.clone(), s2.clone(), s3.clone()], 1, 0);
        let tower: Vec<Rc2<Vec<LM2>>> =
            vec![Rc2::new(vec![s0, s1, s2, s3]), Rc2::new(vec![m2])];
        let hist: Vec<f64> = (0..20).map(|i| if i < 10 { 2.0 } else { 1.0 }).collect();
        assert!(
            super::pan_div_gate_pass(&tower, 1, &pan_cert(19, Side::Long), &hist, 19, &[], &[], &[]),
            "Nest 通道：次级别 Type1 锚（descend Some）⟹ PanDiv^δ_ℓ ⟹ ∃e<ℓ Conf^δ_e 承接成立"
        );
    }

    /// XZD 通道正例：递归底（sub_moves 空 ⟹ descend None ⟹ Nest 闭）+ 同点共生 buy2
    /// （C2，level=0 ≠1 ⟹ C2-only gate_pass）⟹ XZD 通道承接成立。
    #[test]
    fn pan_div_gate_xzd_channel_passes_via_type2_confirmed() {
        let tower: Vec<Rc2<Vec<LM2>>> = vec![Rc2::new(vec![xzd_seg(10, 19)])];
        let hist = vec![0.0f64; 20];
        let mut buy2 = BspBits::default();
        buy2.buy2 = true;
        let bsp_of_level = vec![xzd_bsp(19, buy2, None)];
        assert!(
            super::pan_div_gate_pass(
                &tower, 0, &pan_cert(19, Side::Long), &hist, 19, &bsp_of_level, &[], &[],
            ),
            "XZD 通道：Nest 闭（递归底无次级别锚）+ C2 共生 buy2 ⟹ XZD^δ 承接成立"
        );
    }

    /// 两门皆闭：递归底无次级别锚（Nest 闭）∧ 无共生 buy2（XZD C2 假）⟹ 承接失败 ⟹
    /// collect_signals 诚实丢弃该 PanDiv（不入信号流，不兜底）。
    #[test]
    fn pan_div_gate_both_channels_closed_rejects() {
        let tower: Vec<Rc2<Vec<LM2>>> = vec![Rc2::new(vec![xzd_seg(10, 19)])];
        let hist = vec![0.0f64; 20];
        assert!(
            !super::pan_div_gate_pass(&tower, 0, &pan_cert(19, Side::Long), &hist, 19, &[], &[], &[]),
            "两门皆闭 ⟹ PanDiv 承接失败（诚实丢弃）"
        );
        // 执行段缺失（tower[lvl] 无 end_index==source_index）⟹ 同样拒。
        assert!(
            !super::pan_div_gate_pass(&tower, 0, &pan_cert(7, Side::Long), &hist, 7, &[], &[], &[]),
            "无执行段定位 ⟹ 承接失败"
        );
    }

    /// PDF §4 反例的分解算术自检（合成，L1 验证分解算术正确）：
    /// 多头反转交易腿 P[λ_rev]=10, P[ρ_rev]=12（Ab_rev=δ·2=2，δ=+1）；确认滞后 Pτin=11.8, Pτout=11.1。
    /// ηin=max(0,11.8−10)=1.8, ηout=max(0,12−11.1)=0.9, captured=2−1.8−0.9−Ce<0（执行吃光）。
    /// 664 号：λ_rev/ρ_rev 是入场/出场信号挂靠 pivot 端点价，δ=交易方向（非触发段笔方向 ε）。
    #[test]
    fn pdf_counterexample_decomp() {
        let eps = 1.0_f64; // 多头反转腿 δ=+1
        let (p_lambda, p_rho, p_tau_in, p_tau_out) = (10.0, 12.0, 11.8, 11.1);
        let a_b = eps * (p_rho - p_lambda);
        let eta_in = (eps * (p_tau_in - p_lambda)).max(0.0);
        let eta_out = (eps * (p_rho - p_tau_out)).max(0.0);
        let captured = a_b - eta_in - eta_out; // 无成本版
        assert!((a_b - 2.0).abs() < 1e-9);
        assert!((eta_in - 1.8).abs() < 1e-9);
        assert!((eta_out - 0.9).abs() < 1e-9);
        // 2 − 1.8 − 0.9 = −0.7 < 0：执行损耗吃光结构价差（PDF §4 因果亏 −0.7 的分解归因）。
        assert!(captured < 0.0, "captured={captured} 应 <0（PDF §4 反例：执行吃光价差）");
        assert!((captured - (-0.7)).abs() < 1e-9);
    }

    /// 664 号反转交易腿方向语义自检（合成，L1）：底背驰买点 δ=+1，入场 pivot 低、出场 pivot 高
    /// ⟹ Ab_rev=δ(P[ρ_rev]−P[λ_rev])>0；顶背驰卖点 δ=−1，入场 pivot 高、出场 pivot 低 ⟹ Ab_rev>0。
    /// 两侧反转腿价差都为正——证明 δ 作用在 post-signal 腿（入场→出场 pivot）上，不与触发段方向 ε 耦合。
    #[test]
    fn reversal_leg_direction_semantics() {
        // 底买反转腿：λ_rev=入场低点 pivot 100，ρ_rev=出场高点 pivot 120，δ=+1。
        let ab_buy = 1.0_f64 * (120.0 - 100.0);
        assert!(ab_buy > 0.0, "底买反转腿 Ab_rev 应 >0（入场低点→出场高点），实得 {ab_buy}");
        // 顶卖反转腿：λ_rev=入场高点 pivot 120，ρ_rev=出场低点 pivot 100，δ=−1。
        let ab_sell = (-1.0_f64) * (100.0 - 120.0);
        assert!(ab_sell > 0.0, "顶卖反转腿 Ab_rev 应 >0（入场高点→出场低点，δ=−1 翻正），实得 {ab_sell}");
        // 对照（664 号错配示意）：δ 作用在触发段（底买出现在下跌段末端，触发段 Pρ<Pλ）⟹ δ·(Pρ−Pλ)<0
        // ——这正是旧实装系统性 ΣAb<0 的来源（测错对象，非判据被否证）。
        let ab_old_trigger = 1.0_f64 * (100.0 - 120.0);
        assert!(ab_old_trigger < 0.0, "旧触发段对象 δ·(Pρ−Pλ) 系统性负（对象错配示意）");
    }

    /// spread_eaten 判据自检：Σcaptured≤0 ⟺ 损耗吃光。
    #[test]
    fn spread_eaten_iff_captured_nonpositive() {
        let mut agg = SpreadAttribution::default();
        agg.sum_captured = -0.1;
        assert!(agg.spread_eaten());
        agg.sum_captured = 0.1;
        assert!(!agg.spread_eaten());
    }

    /// 664-Q3 signed 分解恒等式自检（合成，L1）：actual_spread = Ab_rev − x_in − y_out 恒成立，
    /// 覆盖有利滑移（x<0/y<0）——证明 actual_spread 含全部滑移，而 captured(adverse-only)=Ab−max(0,x)−max(0,y) 丢有利滑移。
    #[test]
    fn signed_decomp_actual_spread_identity() {
        // 多头：λ_rev=100, ρ_rev=120 ⟹ Ab_rev=20。Pτin=98（确认价比入场 pivot 更优，x<0 有利），
        // Pτout=125（出场比 pivot 更高，y<0 有利）。δ=+1。
        for &(p_lambda, p_rho, p_tau_in, p_tau_out, delta) in &[
            (100.0, 120.0, 98.0, 125.0, 1.0_f64),  // 双侧有利滑移
            (100.0, 120.0, 105.0, 110.0, 1.0),     // 双侧不利滑移
            (120.0, 100.0, 122.0, 95.0, -1.0),     // 空头：入场更高(有利)、出场更低(有利)
        ] {
            let a_b = delta * (p_rho - p_lambda);
            let x_in = delta * (p_tau_in - p_lambda);
            let y_out = delta * (p_rho - p_tau_out);
            let actual_spread = delta * (p_tau_out - p_tau_in);
            // 恒等式：actual_spread = Ab_rev − x_in − y_out（codex Q3）。
            assert!((actual_spread - (a_b - x_in - y_out)).abs() < 1e-9,
                "signed 分解恒等式破：actual_spread={actual_spread} ≠ Ab−x−y={}", a_b - x_in - y_out);
            // adverse-only captured 与 actual_spread 的差 = min(x,0)+min(y,0) ≤ 0（系统性低估）。
            let captured_no_cost = a_b - x_in.max(0.0) - y_out.max(0.0);
            let diff = captured_no_cost - actual_spread;
            assert!(diff <= 1e-9,
                "adverse-only captured 应 ≤ actual_spread（丢有利滑移），diff={diff}");
            assert!((diff - (x_in.min(0.0) + y_out.min(0.0))).abs() < 1e-9,
                "captured−actual 应 = min(x,0)+min(y,0)");
        }
    }

    /// actual_pnl_proxy / actual_pnl_eaten 判据自检：与 spread_eaten 可分歧（codex Q3 核心）。
    #[test]
    fn actual_pnl_proxy_diverges_from_adverse_only() {
        let mut agg = SpreadAttribution::default();
        // 构造：actual_spread 正、但 adverse-only captured 负（有利滑移被丢导致伪「吃光」）。
        agg.sum_actual_spread = 100.0;
        agg.sum_ce = 30.0;
        agg.sum_captured = -50.0; // adverse-only 判定「吃光」
        assert!(agg.spread_eaten(), "adverse-only 判吃光");
        assert!((agg.actual_pnl_proxy() - 70.0).abs() < 1e-9);
        assert!(!agg.actual_pnl_eaten(), "真实成交口径未吃光 ⟹ adverse-only 是伪结论");
    }

    /// L2 重测「钱去哪了」：BTC 全历史逐信号反转交易腿 Ab_rev 可捕获价差分解（真实数据，可产否定性结果；664 号）。
    ///
    /// `#[ignore]`：需 BTC 全量数据（314M）+ O(n²) 逐 bar 重分类，`--release` 必须。重跑由 Lead。
    /// 报告落盘 `.chanlun/review-results/econ-abrev-l2-btc-20260630.md`（确定性，可复算；不覆盖旧触发段报告）。
    ///
    /// **L2 纪律**：spread_eaten=true ⟹ 该信号集执行损耗吃光反转交易腿价差（否证可交易，有效域收窄）；
    /// spread_eaten=false ⟹ 该级别有可捕获 alpha——两者照实报，不粉饰（161/formalization-validity-domain）。
    #[test]
    #[ignore]
    fn l2_btc_capturable_spread_diagnosis() {
        use super::super::data;
        use std::collections::BTreeMap;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();

        // 截断窗（显式有效域边界，非全窗结论；l3_delta_r_alpha::MAX_BARS 同纪律）：
        // ★fullhist-oom-fix-20260630 订正：旧注释「500K OOM 被杀=内存上限」是**误诊**（错误归因，090号）。
        // /usr/bin/time -l 实测峰值 RSS：100K=1990MB、200K=2010MB、400K=2034MB、600K=2083MB——
        // 内存随 bar **平坦**（100K→600K 仅 +5%），无 OOM。塔状态（TowerCache.upper_moves）累积是塔元素数
        // （走势/中枢，亚线性于 bar），不是「每 bar 一份 Rc 副本」。旧「500K OOM」实为 **O(n²) 时间墙超时被
        // kill** 被误报为 OOM。真瓶颈 = classify_with_tower_incremental 的 O(tree)/bar 续算（profile_classify_at
        // 已坐实 t_exp≈2.0；归 Task #104/#105 classifier 核心优化）。截断窗在此**为时间非内存**：实测
        // 100K=8.7s、200K=31.8s、400K=124.5s、600K=283s（O(n²)），全量 461万≈数小时可跑通但慢。
        // env ECON_L2_MAX_BARS 覆盖供 Lead 调窗（>4.6M=不截断跑全量）。
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars {
            let start = ds_full.dates[n_full - max_bars].get(..10).unwrap_or("").to_string();
            let end = ds_full.dates[n_full - 1].get(..10).unwrap_or("").to_string();
            eprintln!("截断窗 [{start}→{end}]（最后 {max_bars} bar / 全量 {n_full}）= 显式有效域边界");
            ds_full.slice_date_window(&start, &end)
        } else {
            ds_full
        };
        let untradable = ds.untradable_ratio();
        let n_bars = ds.bars.len();
        let window_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let window_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        let (decomps, agg) = decompose_capturable_spread(&ds, &config);
        crate::theta_v0::classifier::stage_profile::dump();

        // b2（task #83）：per-class 分桶键从粗投影 Y=(level,δ,bsp_class) 升到完整状态 Z=[`MuClass`]
        // （含 σ_p/短差/仓位态/H=完整 R(g)）——编排者点破「回测跑在 Y 粗投影上」的接入点闭合。
        // 分桶载体 `d.z`（`MuClass` 派生 Ord ⟹ BTreeMap 有序）；同 (level,δ,I_γ) 按 σ_p/role 再细分。
        // codex #81 修正2：z 的类型位用 `z.i_class`（未压缩 6-bit），报告列展示避免 `bsp_class()` 压主类。
        // value=(n, Σab, Σηin, Σηout, Σce, Σcaptured, n_cap_pos, Σactual_spread, Σactual_pnl, n_act_pos)。
        type Bucket = (usize, f64, f64, f64, f64, f64, usize, f64, f64, usize);
        let mut buckets: BTreeMap<MuClass, Bucket> = BTreeMap::new();
        for d in &decomps {
            let e = buckets.entry(d.z).or_default();
            e.0 += 1;
            e.1 += d.a_b;
            e.2 += d.eta_in;
            e.3 += d.eta_out;
            e.4 += d.ce_unit;
            e.5 += d.captured;
            if d.captured > 0.0 {
                e.6 += 1;
            }
            e.7 += d.actual_spread;
            e.8 += d.actual_pnl;
            if d.actual_pnl > 0.0 {
                e.9 += 1;
            }
        }

        // ── dx 守恒（分划细化保 Σ，codex #81 修正5：细 key 求和回粗 key，写维度无关通用形式）──
        // Z 桶是 Y=(level,δ,bsp_class) 桶的不相交细分（同 Y 按 σ_p/role/H 再分）⟹ 分划细化。
        // 逐笔 actual_pnl 与分桶无关 ⟹ Σ_{z∈Z} 桶内和 == Σ_{y∈Y} 桶内和 == Σ_全体 decomps（守恒）。
        // 不写死 σ_p 一维——对任意细化维（σ_p/role/H/未来新维）恒成立（维度无关）。
        {
            let sum_z: f64 = buckets.values().map(|b| b.8).sum(); // Σ over Z 桶
            let sum_flat: f64 = decomps.iter().map(|d| d.actual_pnl).sum(); // Σ over 全体（粗 key 极限）
            debug_assert!(
                (sum_z - sum_flat).abs() <= 1e-6 * (1.0 + sum_flat.abs()),
                "dx 守恒破：Σ_Z 桶 actual_pnl={sum_z} ≠ Σ 全体={sum_flat}（分划细化应保 Σ）"
            );
        }

        // 失血三源占比（分母 = Σ(ηin+ηout+Ce) 总损耗；ΣAb 为正分母比对结构价差）。
        let total_drain = agg.sum_eta_in + agg.sum_eta_out + agg.sum_ce;
        let pct = |x: f64| if total_drain > 0.0 { 100.0 * x / total_drain } else { 0.0 };

        let actual_pnl_proxy = agg.actual_pnl_proxy();

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# 663 判据：BTC 全级别×方向逐信号 μ̂>0 + 全历史长窗累积净值");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的逐信号确定性分解，可产否定性结果）。");
        let _ = writeln!(rpt, "**663 判据**：可交易性=μ(z,a)>0（正条件期望，出现就做不统计显著），**非** p<0.05 统计显著/跨品种符号检验。全级别 0..L 照报（级别是缠论全互斥定义构成部分，稀疏高级别不剔不判 Le Cam 硬墙——663 收窄 Le Cam 有效域至短窗单品种区分±Δ）。全历史长窗累积净值（O(n²) 已解锁 exp1.18，461万 bar ~2.6min）。");
        let _ = writeln!(rpt, "**664-Q3 修正**：旧 captured=Ab_rev−max(0,x)−max(0,y)−Ce 用 max(0,·) **丢有利滑移、全计不利滑移** ⟹ 系统性低估真实 PnL（captured−actual_pnl=min(x,0)+min(y,0)≤0）= **adverse-only 保守压力测试，非真实成交**。本报告补 signed Σx/Σy 与真实成交价差 actual_spread=δ(Pτout−Pτin)=Ab_rev−x−y。");
        let _ = writeln!(rpt, "**两口径**：x=δ(Pτin−Pλ_rev)（signed 入场滑移）、y=δ(Pρ_rev−Pτout)（signed 出场滑移）。");
        let _ = writeln!(rpt, "- **adverse-only captured** = Ab_rev−max(0,x)−max(0,y)−Ce（压力测试，保守，保留兼容旧口径）。");
        let _ = writeln!(rpt, "- **真实成交 actual_pnl** = actual_spread−Ce = δ(Pτout−Pτin)−Ce（含全部滑移，更接近真实成交）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release l2_btc_capturable_spread_diagnosis -- --ignored --nocapture`（确定性）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据");
        let _ = writeln!(rpt, "- 品种：BTC（btc_1m_full.json，全量 {n_full} bar，2017-08→2026-05）");
        let _ = writeln!(rpt, "- **截断窗 [{window_start}→{window_end}]，bars={n_bars}**（最后 {max_bars} bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）");
        let _ = writeln!(rpt, "- untradable_ratio={:.4}", untradable);
        let _ = writeln!(rpt, "- 收集信号数 n_signals={}（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）", agg.n_signals);
        // #115 (e) fill-rate 断言（防 β^div 路由全 None 静默）。历史：曾 re-scope 到本报告，因当时
        // Candidate 边界无 force 源（fill loop 侧断言必误报，见 g2-impl-20260703.md）；A6（#159）后
        // Candidate 携 force、fill loop z_of_candidate 同样装配真值——fill loop 侧对应断言见
        // wverify_run::wverify_fullz（records 探针）。零一类信号的窗不假失败（force 仅一类 A/C 对候选有源）。
        let n_type1 = decomps.iter().filter(|d| d.z.i_class & 0b001_001 != 0).count();
        let n_force_some = decomps.iter().filter(|d| d.z.force_state.is_some()).count();
        assert!(n_type1 == 0 || n_force_some > 0, "β^div 路由静默断裂：{n_type1} 条一类信号 force_state 全 None");
        let _ = writeln!(rpt, "- β^div force fill-rate：{}/{}（一类信号 {} 条；断言=有一类则 force 非全 None，#115 (e)）", n_force_some, agg.n_signals, n_type1);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 三审计统计（codex #97 要求）");
        let _ = writeln!(rpt, "| 统计 | 值 | 说明 |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| n_rho_after_lambda | {}/{} | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |", agg.n_rho_after_lambda, agg.n_signals);
        let _ = writeln!(rpt, "| n_same_bar_opposite | {} | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |", agg.n_same_bar_opposite);
        let _ = writeln!(rpt, "| n_unpaired | {} | 无配对出场反转信号（右删失诚实跳过，codex Q4） |", agg.n_unpaired);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## P7 正规出场口径统计（接 sell.rs CloseRoot/ReduceCore，n={}）", agg.n_signals);
        let _ = writeln!(rpt, "| 出场决策 | 信号数 | 占比 | 说明 |");
        let _ = writeln!(rpt, "|---|---|---|---|");
        let pct_sig = |x: usize| if agg.n_signals > 0 { 100.0 * x as f64 / agg.n_signals as f64 } else { 0.0 };
        let _ = writeln!(rpt, "| CloseRoot（第一类顶背驰） | {} | {:.1}% | sell.rs SellDecision::CloseRoot |", agg.n_exit_close_root, pct_sig(agg.n_exit_close_root));
        let _ = writeln!(rpt, "| ReduceCore（第三类减核） | {} | {:.1}% | sell.rs SellDecision::ReduceCore |", agg.n_exit_reduce_core, pct_sig(agg.n_exit_reduce_core));
        let _ = writeln!(rpt, "| Type2Missing（第二类 still-MISSING） | {} | {:.1}% | sell.rs:35 诚实边界，不改账本口径 |", agg.n_exit_type2_missing, pct_sig(agg.n_exit_type2_missing));
        let _ = writeln!(rpt, "| Hold（无正规卖点 bit） | {} | {:.1}% | exit 信号无正规卖侧/买侧 bit |", agg.n_exit_hold, pct_sig(agg.n_exit_hold));
        let _ = writeln!(rpt, "（认识论 L0：口径结构变，不声明 alpha；alpha 待 W-VERIFY L2/L3。第二类闭环 still-MISSING 见 sell.rs:35 诚实边界，不碰 TW 三阶段/576。）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 全局归因（双口径）");
        let _ = writeln!(rpt, "| 量 | 值 |");
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| ΣAb_rev（反转交易腿理想总价差） | {:.6e} |", agg.sum_a_b);
        let _ = writeln!(rpt, "| Σx（signed 入场滑移） | {:.6e} |", agg.sum_x_in);
        let _ = writeln!(rpt, "| Σy（signed 出场滑移） | {:.6e} |", agg.sum_y_out);
        let _ = writeln!(rpt, "| Σmax(0,x)=Σηin（adverse-only 入场） | {:.6e} ({:.1}%) |", agg.sum_eta_in, pct(agg.sum_eta_in));
        let _ = writeln!(rpt, "| Σmax(0,y)=Σηout（adverse-only 出场） | {:.6e} ({:.1}%) |", agg.sum_eta_out, pct(agg.sum_eta_out));
        let _ = writeln!(rpt, "| ΣCe（成本） | {:.6e} ({:.1}%) |", agg.sum_ce, pct(agg.sum_ce));
        let _ = writeln!(rpt, "| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | {:.6e} |", agg.sum_actual_spread);
        let _ = writeln!(rpt, "| **Σcaptured（adverse-only 压力测试剩余）** | {:.6e} |", agg.sum_captured);
        let _ = writeln!(rpt, "| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | {:.6e} |", actual_pnl_proxy);
        let _ = writeln!(rpt, "| n_captured_positive/n（adverse-only 正占比） | {}/{} ({:.1}%) |",
            agg.n_captured_positive, agg.n_signals,
            if agg.n_signals > 0 { 100.0 * agg.n_captured_positive as f64 / agg.n_signals as f64 } else { 0.0 });
        let _ = writeln!(rpt, "| n_actual_positive/n（真实成交正占比） | {}/{} ({:.1}%) |",
            agg.n_actual_positive, agg.n_signals,
            if agg.n_signals > 0 { 100.0 * agg.n_actual_positive as f64 / agg.n_signals as f64 } else { 0.0 });
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## L2 判定（关键：两口径分歧 = codex Q3 核心）");
        let _ = writeln!(rpt, "- **adverse-only spread_eaten = {}**（Σcaptured {} 0）", agg.spread_eaten(), if agg.spread_eaten() { "≤" } else { ">" });
        let _ = writeln!(rpt, "- **真实成交 actual_pnl_eaten = {}**（actual_pnl_proxy {} 0）", agg.actual_pnl_eaten(), if agg.actual_pnl_eaten() { "≤" } else { ">" });
        if agg.spread_eaten() && !agg.actual_pnl_eaten() {
            let _ = writeln!(rpt, "- **翻案（codex Q3 坐实）**：adverse-only 判「执行吃光」但真实成交 actual_pnl_proxy>0 ⟹ 之前「执行滞后吃光」(Σcaptured=−2.33e5) 是 **adverse-only 伪结论**——有利滑移被 max(0,·) 丢弃所致。真实成交口径下反转腿可捕获。");
        } else if agg.actual_pnl_eaten() {
            let _ = writeln!(rpt, "- **「执行吃光」成立（真实成交口径）**：actual_pnl_proxy≤0 ⟹ 即便不丢有利滑移，真实成交价差减成本仍非正 ⟹ 该信号集真实亏（否定性结果，缩小有效域边界，161/formalization-validity-domain）。");
        } else {
            let _ = writeln!(rpt, "- **两口径一致正**：adverse-only 与真实成交均 >0 ⟹ 反转腿价差未被吃光（保守口径都过 ⟹ 真实更宽松）。仅 BTC 单标的 L2，非跨品种功效。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 失血三源对比（ΣAb_rev 为反转腿结构上限）");
        let _ = writeln!(rpt, "- 反转腿价差：ΣAb_rev={:.6e}（664 号测对了对象——可正可负，非触发段同义反复）", agg.sum_a_b);
        let _ = writeln!(rpt, "- 执行吃光：Ση={:.6e}（入场+出场滞后）", agg.sum_eta_in + agg.sum_eta_out);
        let _ = writeln!(rpt, "- 成本：ΣCe={:.6e}", agg.sum_ce);
        if agg.sum_a_b < 0.0 {
            let _ = writeln!(rpt);
            let _ = writeln!(rpt, "**ΣAb_rev<0（反转交易腿）：** 664 号修复后仍为负 ⟹ 不是对象错配（已测对反转腿），是反转交易腿");
            let _ = writeln!(rpt, "本身的真实价差为负——配对出场信号 pivot 相对入场信号 pivot 在交易方向 δ 上整体不利。这是测对");
            let _ = writeln!(rpt, "对象之后的真实否定性结果（231：缩小有效域边界），不靠改符号修复（改 eps=恢复同义反复，090+no-patch）。");
            let _ = writeln!(rpt, "即使零滞后零成本（Ση=ΣCe=0），captured=ΣAb_rev<0 仍无 alpha——但这次测的是反转交易腿，不是触发段。");
        } else {
            let _ = writeln!(rpt);
            let _ = writeln!(rpt, "**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4");
            let _ = writeln!(rpt, "形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。");
        }
        let _ = writeln!(rpt);
        // b2（task #83）：z 角色维紧凑格式化（σ_p / 短差 / 仓位态 / H）——报告展示 Z 细分，
        // 使同 (level,δ,I_γ) 按 σ_p/role/H 再分的桶不显示为重复行（诚实展示分桶键升维）。
        let z_role_str = |z: &MuClass| -> String {
            let h = match z.horizontal {
                Some(Horizontal::First) => "F",
                Some(Horizontal::SameFollow) => "SF",
                Some(Horizontal::SameReverse) => "SR",
                None => "-", // 裸证书口径未定 H（本 alpha 路径 z_of_candidate 恒 Some，None 不应现）
            };
            let pos = match z.position { PositionState::Root => "R", PositionState::Child => "C" };
            format!("{:+}|{}|{}|{}", z.parent_dir, if z.short_swing { "sw" } else { "tr" }, pos, h)
        };
        let _ = writeln!(rpt, "## per-class 完整状态 Z=(level, δ, I_γ, σ_p, 短差, 仓位态, H) 分桶（b2 #83：Y 粗投影→Z 细状态）");
        let _ = writeln!(rpt, "μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。");
        let _ = writeln!(rpt, "I_γ=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3，未压缩 6-bit=z.i_class）。");
        let _ = writeln!(rpt, "role=σ_p|短差(sw/tr)|仓位态(R/C)|H(F/SF/SR)——R(g)=(H,V,δ) 完整角色（codex #81 H 进 canonical Z）。");
        let _ = writeln!(rpt, "| level | δ | I_γ | role(σ_p\\|sw\\|pos\\|H) | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|---|---|---|---|");
        for (z, (n, sab, sin, sout, sce, scap, _ncap, _sact, sactpnl, nact)) in &buckets {
            let mu_hat = if *n > 0 { sactpnl / *n as f64 } else { 0.0 };
            let _ = writeln!(rpt, "| {} | {:+} | 0x{:02x} | {} | {} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {}/{} ({:.0}%) |",
                z.level, z.delta, z.i_class, z_role_str(z), n, mu_hat, sactpnl, sab, sin + sout, sce, scap,
                nact, n, if *n > 0 { 100.0 * *nact as f64 / *n as f64 } else { 0.0 });
        }
        let _ = writeln!(rpt);
        // ── 663 判据：全级别×方向×类型 μ̂>0 分类（正条件期望，出现就做不统计显著）。 ──
        let _ = writeln!(rpt, "## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）");
        let max_level = buckets.keys().map(|z| z.level).max().unwrap_or(0);
        let _ = writeln!(rpt, "涌现最高级别 L={max_level}（全级别 0..{max_level} 均列；稀疏高级别照报不判硬墙，663）。");
        let _ = writeln!(rpt, "**有效域**：本窗 bars={n_bars}（{window_start}→{window_end}）。663 要求全历史长窗——");
        let _ = writeln!(rpt, "若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。");
        let _ = writeln!(rpt, "μ̂>0 类 = 该完整状态 z 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：");
        let mut n_pos_class = 0usize;
        let mut n_total_class = 0usize;
        for (z, (n, _, _, _, _, _, _, _, sactpnl, nact)) in &buckets {
            n_total_class += 1;
            let mu_hat = if *n > 0 { sactpnl / *n as f64 } else { 0.0 };
            let mark = if mu_hat > 0.0 { n_pos_class += 1; "✓μ̂>0" } else { "✗μ̂≤0" };
            let _ = writeln!(rpt, "- (level={}, δ={:+}, I_γ=0x{:02x}, role={}) {mark}: μ̂={mu_hat:.4e}, Σ={sactpnl:.4e}, n_act+/n={nact}/{n}",
                z.level, z.delta, z.i_class, z_role_str(z));
        }
        let _ = writeln!(rpt, "**{n_pos_class}/{n_total_class} 类 μ̂>0**（663 判据：正期望类可交易，不因稀疏判 inconclusive）。");
        let _ = writeln!(rpt);

        // ── 全历史累积净值曲线（长窗，按时间 entry_bar 序）：全级别 + per-level 分层。 ──
        // 663：全历史长窗累积净值——每级别按时间累加 actual_pnl，看是否正期望累积。
        let _ = writeln!(rpt, "## 全历史累积净值曲线（长窗，按 entry_bar 时间序）");
        {
            let mut ordered: Vec<&SignalDecomp> = decomps.iter().collect();
            ordered.sort_by_key(|d| d.entry_bar);
            // 全局累积 + per-level 累积（终值 + 是否单调正向）。
            let mut cum_all = 0.0f64;
            let mut cum_by_level: BTreeMap<u32, f64> = BTreeMap::new();
            let mut min_cum_all = 0.0f64; // 最大回撤参考（累积曲线最低点）。
            for d in &ordered {
                cum_all += d.actual_pnl;
                if cum_all < min_cum_all { min_cum_all = cum_all; }
                *cum_by_level.entry(d.level).or_default() += d.actual_pnl;
            }
            let _ = writeln!(rpt, "- **全级别累积净值终值 = {cum_all:.4e}**（{} 信号，min 累积={min_cum_all:.4e} 曲线最低点）", ordered.len());
            let _ = writeln!(rpt, "| level | 该级别累积净值 | 正期望? |");
            let _ = writeln!(rpt, "|---|---|---|");
            for (lvl, cum) in &cum_by_level {
                let _ = writeln!(rpt, "| {} | {:.4e} | {} |", lvl, cum, if *cum > 0.0 { "✓" } else { "✗" });
            }
            // 曲线采样（10 点等信号间隔）——看累积轨迹形状（单调正 vs 前正后回吐）。
            if ordered.len() >= 10 {
                let _ = writeln!(rpt);
                let _ = writeln!(rpt, "累积净值曲线采样（10 等分点，全级别）：");
                let step = ordered.len() / 10;
                let mut c = 0.0f64;
                for (i, d) in ordered.iter().enumerate() {
                    c += d.actual_pnl;
                    if (i + 1) % step == 0 || i + 1 == ordered.len() {
                        let _ = writeln!(rpt, "- [{}/{}] entry_bar={} 累积={:.4e}", i + 1, ordered.len(), d.entry_bar, c);
                    }
                }
            }
        }

        // ── 666 号 σ_higher 分布 + 逐信号台账 CSV dump。 ──
        // σ_higher 顺/逆/无：δ 与上级方向态一致(δ==σ_higher)/相反/上级无方向(σ_higher==0)。
        let (mut n_align, mut n_against, mut n_none) = (0usize, 0usize, 0usize);
        for d in &decomps {
            match d.sigma_higher {
                0 => n_none += 1,
                s if s == d.delta => n_align += 1,
                _ => n_against += 1,
            }
        }
        let _ = writeln!(rpt, "## 666 号 σ_higher 分布（上级方向态 vs δ）");
        let nsig = decomps.len().max(1);
        let _ = writeln!(rpt, "- 顺上级（δ==σ_higher）：{n_align}/{} ({:.1}%)", decomps.len(), 100.0 * n_align as f64 / nsig as f64);
        let _ = writeln!(rpt, "- 逆上级（δ==−σ_higher）：{n_against}/{} ({:.1}%)", decomps.len(), 100.0 * n_against as f64 / nsig as f64);
        let _ = writeln!(rpt, "- 无上级（σ_higher==0）：{n_none}/{} ({:.1}%)", decomps.len(), 100.0 * n_none as f64 / nsig as f64);
        let _ = writeln!(rpt);

        eprint!("{rpt}");

        // 逐信号台账 CSV（666 号：alpha 分离下游依赖，15 列含 sigma_higher）。输出到 /tmp。
        let csv_path = std::env::var("ECON_LEDGER_CSV")
            .unwrap_or_else(|_| "/tmp/btc_663_ledger_sigma.csv".to_string());
        // W4 类型透传：bsp_class（buy1/2/3+sell1/2/3 位掩码 u8）+ side（买/卖，delta 派生）+ exit_decision（P7）列。
        let mut csv = String::from(
            "idx,entry_bar,exit_bar,level,delta,a_b,x_in,y_out,eta_in,eta_out,actual_spread,ce_unit,captured,actual_pnl,sigma_higher,bsp_class,side,exit_decision\n",
        );
        for (i, d) in decomps.iter().enumerate() {
            let side = if d.delta > 0 { "buy" } else { "sell" };
            let exit_dec = match d.exit_decision {
                ExitDecision::CloseRoot => "CloseRoot",
                ExitDecision::ReduceCore => "ReduceCore",
                ExitDecision::Type2Missing => "Type2Missing",
                ExitDecision::Hold => "Hold",
            };
            let _ = writeln!(csv, "{},{},{},{},{},{:.10e},{:.10e},{:.10e},{:.10e},{:.10e},{:.10e},{:.10e},{:.10e},{:.10e},{},{},{},{}",
                i, d.entry_bar, d.exit_bar, d.level, d.delta,
                d.a_b, d.x_in, d.y_out, d.eta_in, d.eta_out, d.actual_spread,
                d.ce_unit, d.captured, d.actual_pnl, d.sigma_higher, d.bsp_class, side, exit_dec);
        }
        std::fs::write(&csv_path, &csv).unwrap_or_else(|e| panic!("写台账 CSV {csv_path} 失败：{e}"));
        eprintln!("逐信号台账 CSV 已落盘：{csv_path}（{} 行 + 表头）", decomps.len());

        // 落盘 signed 报告（664-Q3，不覆盖 econ-abrev-l2-btc-20260630.md 旧 adverse-only-only 报告）。
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econ-663-full-level-mu-20260701.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封①：adverse-only captured 分解恒等式逐信号成立。
        for d in &decomps {
            let recomputed = d.a_b - d.eta_in - d.eta_out - d.ce_unit;
            assert!((d.captured - recomputed).abs() < 1e-6,
                "captured 分解恒等式破：level={} δ={} captured={} ≠ {}", d.level, d.delta, d.captured, recomputed);
            // 真封②（664-Q3 核心）：actual_spread = Ab_rev − x − y 逐信号恒成立（signed 分解）。
            assert!((d.actual_spread - (d.a_b - d.x_in - d.y_out)).abs() < 1e-6,
                "actual_spread 恒等式破：level={} δ={} actual_spread={} ≠ Ab−x−y={}",
                d.level, d.delta, d.actual_spread, d.a_b - d.x_in - d.y_out);
            // 真封③：eta = max(0, signed) 一致。
            assert!((d.eta_in - d.x_in.max(0.0)).abs() < 1e-9 && (d.eta_out - d.y_out.max(0.0)).abs() < 1e-9,
                "η 应 = max(0, signed)：level={} δ={}", d.level, d.delta);
            // 真封④：captured ≤ actual_pnl（adverse-only 系统性 ≤ 真实成交；丢有利滑移）。
            assert!(d.captured <= d.actual_pnl + 1e-6,
                "adverse-only captured 应 ≤ actual_pnl：level={} δ={} captured={} > actual_pnl={}",
                d.level, d.delta, d.captured, d.actual_pnl);
        }
        // 聚合 Σ 无丢失。
        let sum_check: f64 = decomps.iter().map(|d| d.captured).sum();
        assert!((agg.sum_captured - sum_check).abs() < 1e-3,
            "Σcaptured 聚合 {} ≠ 逐信号和 {}", agg.sum_captured, sum_check);
        let sum_act: f64 = decomps.iter().map(|d| d.actual_spread).sum();
        assert!((agg.sum_actual_spread - sum_act).abs() < 1e-3,
            "Σactual_spread 聚合 {} ≠ 逐信号和 {}", agg.sum_actual_spread, sum_act);
    }

    /// per-class `(level, δ, bsp_class)` 真实成交统计：(n, Σactual_pnl, n_act+, Σab_rev)。
    /// P4（codex E-3）：分桶键含 bsp_class ⟹ 单类型 α 可辨——混合 (level,δ) 池会把 buy1/buy2/buy3
    /// 混一桶，桶均值只给混合均值（稀释是 L0 结构必然，codex E-2）。estimand 桶键 `(ℓ,δ,bsp_class)`。
    fn class_actual_pnl(decomps: &[SignalDecomp], level: u32, delta: i8, bsp_class: u8) -> (usize, f64, usize, f64) {
        decomps.iter().filter(|d| d.level == level && d.delta == delta && d.bsp_class == bsp_class).fold(
            (0usize, 0.0f64, 0usize, 0.0f64),
            |(n, pnl, npos, ab), d| {
                (n + 1, pnl + d.actual_pnl, npos + (d.actual_pnl > 0.0) as usize, ab + d.a_b)
            },
        )
    }

    /// `(level, δ)` 聚合（Σ over bsp_class）真实成交统计——用于 δ 方向不对称对照（level0 卖 vs 买，
    /// 与 bsp_class 正交）+ 诊断报告。**非 estimand 桶**（estimand 桶是 per-class [`class_actual_pnl`]）。
    fn class_actual_pnl_agg(decomps: &[SignalDecomp], level: u32, delta: i8) -> (usize, f64, usize, f64) {
        decomps.iter().filter(|d| d.level == level && d.delta == delta).fold(
            (0usize, 0.0f64, 0usize, 0.0f64),
            |(n, pnl, npos, ab), d| {
                (n + 1, pnl + d.actual_pnl, npos + (d.actual_pnl > 0.0) as usize, ab + d.a_b)
            },
        )
    }

    /// 窗口净涨跌方向：首尾 close 总收益**百分数**（>0 涨段 / <0 跌段）。tick 抵消，直接用 close 整数比。
    fn window_net_return(ds: &Dataset) -> f64 {
        let (first, last) = (ds.bars.first(), ds.bars.last());
        match (first, last) {
            (Some(f), Some(l)) if f.close > 0 => 100.0 * (l.close as f64 - f.close as f64) / f.close as f64,
            _ => 0.0,
        }
    }

    /// **663 推论 OOS 验证（防 winner's curse）**：对 in-sample 最强正类 level0卖（level=0,δ=−1，
    /// in-sample actual_pnl=+3.47e4）做 train/holdout 时间切分 + 方向不对称分解。
    ///
    /// `#[ignore]`：同 l2_btc 需 BTC 全量 + O(n²) 重分类，`--release`。
    ///
    /// **L2 纪律**：holdout level0卖 actual_pnl≤0 ⟹ +3.47e4 是窗口/挑赢家产物（否证 alpha，照实报）；
    /// >0 ⟹ OOS 初步稳健（仍单标的单切分，标有效域）。否定性结果照实报（161/formalization-validity-domain）。
    #[test]
    #[ignore]
    fn acc_level0sell_oos() {
        use super::super::data;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();

        // 截断窗（同 l2_btc_capturable_spread_diagnosis：最后 MAX_BARS，OOM 边界=显式有效域）。
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let n = ds.bars.len();
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        // ── 任务1：train(前 frac)/holdout(后 1−frac) 时间切分（半开区间无重叠，时间序不打乱）。 ──
        let train_frac = std::env::var("ECON_L2_TRAIN_FRAC").ok()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.6);
        let split = (n as f64 * train_frac) as usize;
        let ds_train = ds.slice_bar_range(0, split);
        let ds_hold = ds.slice_bar_range(split, n);
        let train_split_day = ds.dates[split].get(..10).unwrap_or("").to_string();

        let (decomps_train, _) = decompose_capturable_spread(&ds_train, &config);
        let (decomps_hold, _) = decompose_capturable_spread(&ds_hold, &config);
        let (tr_n, tr_pnl, tr_pos, tr_ab) = class_actual_pnl_agg(&decomps_train, 0, -1);
        let (hd_n, hd_pnl, hd_pos, hd_ab) = class_actual_pnl_agg(&decomps_hold, 0, -1);
        // 对照：买（δ=+1，in-sample 亏损主源）OOS 是否仍亏。
        let (hd_buy_n, hd_buy_pnl, hd_buy_pos, _) = class_actual_pnl_agg(&decomps_hold, 0, 1);

        let train_ret = window_net_return(&ds_train);
        let hold_ret = window_net_return(&ds_hold);
        let full_ret = window_net_return(&ds);

        // ── 任务2：2 个不重叠子窗（前半/后半，各 n/2），分别算 level0卖 vs level0买。 ──
        let half = n / 2;
        let ds_w1 = ds.slice_bar_range(0, half);
        let ds_w2 = ds.slice_bar_range(half, n);
        let (dw1, _) = decompose_capturable_spread(&ds_w1, &config);
        let (dw2, _) = decompose_capturable_spread(&ds_w2, &config);
        let (w1_s_n, w1_sell, _, _) = class_actual_pnl_agg(&dw1, 0, -1);
        let (w1_b_n, w1_buy, _, _) = class_actual_pnl_agg(&dw1, 0, 1);
        let (w2_s_n, w2_sell, _, _) = class_actual_pnl_agg(&dw2, 0, -1);
        let (w2_b_n, w2_buy, _, _) = class_actual_pnl_agg(&dw2, 0, 1);
        let w1_ret = window_net_return(&ds_w1);
        let w2_ret = window_net_return(&ds_w2);

        // ── 判定 ──
        let oos_robust = hd_pnl > 0.0; // holdout 卖正 ⟹ 初步稳健
        // 方向不对称结构性 ⟺ 两个不重叠子窗 level0卖都优于买（不论子窗涨跌）。
        let sell_beats_buy_w1 = w1_sell > w1_buy;
        let sell_beats_buy_w2 = w2_sell > w2_buy;
        let asym_structural = sell_beats_buy_w1 && sell_beats_buy_w2;

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# 663 推论 OOS 验证：level0卖 train/holdout 切分 + 方向不对称（防 winner's curse）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的单切分，可产否定性结果；非 L3 跨标的）。");
        let _ = writeln!(rpt, "**对象**：in-sample 最强正类 level0卖（level=0, δ=−1，in-sample actual_pnl=+3.47e4/349/36%）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release acc_level0sell_oos -- --ignored --nocapture`（确定性）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据 / 窗口");
        let _ = writeln!(rpt, "- BTC 全量 {n_full} bar；**截断窗 [{win_start}→{win_end}]，bars={n}**（最后 {max_bars} bar；OOM 边界=显式有效域，非全历史，同 l2_btc 纪律）。");
        let _ = writeln!(rpt, "- train_frac={train_frac}，切分 bar 索引={split}（切分日 {train_split_day}，半开区间 train=[0,{split}) / holdout=[{split},{n}) 无重叠）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 窗净涨跌方向（首尾 close 总收益）");
        let _ = writeln!(rpt, "| 窗 | 净收益 | 方向 |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| 截断全窗 | {full_ret:+.2}% | {} |", if full_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| train | {train_ret:+.2}% | {} |", if train_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| holdout | {hold_ret:+.2}% | {} |", if hold_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| 子窗1（前半） | {w1_ret:+.2}% | {} |", if w1_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt, "| 子窗2（后半） | {w2_ret:+.2}% | {} |", if w2_ret > 0.0 { "涨" } else { "跌" });
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务1：level0卖 train vs holdout（关键——正负决定 winner's curse 判定）");
        let _ = writeln!(rpt, "| 窗 | n | Σactual_pnl | n_act+/n（胜率） | Σab_rev |");
        let _ = writeln!(rpt, "|---|---|---|---|---|");
        let _ = writeln!(rpt, "| train | {tr_n} | {tr_pnl:.4e} | {tr_pos}/{tr_n} ({:.0}%) | {tr_ab:.4e} |", winrate(tr_pos, tr_n));
        let _ = writeln!(rpt, "| **holdout** | {hd_n} | **{hd_pnl:.4e}** | {hd_pos}/{hd_n} ({:.0}%) | {hd_ab:.4e} |", winrate(hd_pos, hd_n));
        let _ = writeln!(rpt, "| holdout 对照·买(δ+1) | {hd_buy_n} | {hd_buy_pnl:.4e} | {hd_buy_pos}/{hd_buy_n} ({:.0}%) | — |", winrate(hd_buy_pos, hd_buy_n));
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**判定**：holdout level0卖 actual_pnl = {hd_pnl:.4e} {} 0", if hd_pnl > 0.0 { ">" } else { "≤" });
        if oos_robust {
            let _ = writeln!(rpt, "→ **OOS 初步稳健**：holdout 卖仍正，+3.47e4 不是纯挑赢家产物。**但仅 BTC 单标的单切分 L2，非 L3**——holdout 窗净涨跌={hold_ret:+.2}%，若 holdout 仍跌段则方向效应未排除（见任务2）。");
        } else {
            let _ = writeln!(rpt, "→ **否证 alpha（winner's curse 坐实）**：holdout 卖 actual_pnl≤0 ⟹ in-sample +3.47e4 是窗口/挑赢家产物。这是有价值的否定性结果（缩小有效域边界，161/formalization-validity-domain）——照实报，不粉饰。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务2：方向不对称（2 个不重叠子窗，结构性 vs 窗口效应）");
        let _ = writeln!(rpt, "| 子窗 | 净涨跌 | 卖(δ−1) actual_pnl | n卖 | 买(δ+1) actual_pnl | n买 | 卖>买? |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|");
        let _ = writeln!(rpt, "| 1（前半） | {w1_ret:+.2}% | {w1_sell:.4e} | {w1_s_n} | {w1_buy:.4e} | {w1_b_n} | {} |", if sell_beats_buy_w1 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 2（后半） | {w2_ret:+.2}% | {w2_sell:.4e} | {w2_s_n} | {w2_buy:.4e} | {w2_b_n} | {} |", if sell_beats_buy_w2 { "是" } else { "否" });
        let _ = writeln!(rpt);
        if asym_structural {
            let _ = writeln!(rpt, "**判定**：两个不重叠子窗 level0卖均优于买。");
            if w1_ret * w2_ret < 0.0 {
                let _ = writeln!(rpt, "→ **结构性（跨涨跌都卖优）**：子窗1/子窗2 净涨跌符号相反（一涨一跌）卖仍都优 ⟹ 卖优势非单纯下跌段做空产物，是结构性方向不对称。L2 单标的。");
            } else {
                let _ = writeln!(rpt, "→ **方向同号，未充分隔离**：两子窗净涨跌同号（{w1_ret:+.2}%/{w2_ret:+.2}%），卖优势可能仍含方向效应——结构性结论需跨涨跌子窗或 L3 跨标的。");
            }
        } else {
            let _ = writeln!(rpt, "**判定**：卖优势在子窗间不一致（子窗1卖>买={sell_beats_buy_w1}，子窗2={sell_beats_buy_w2}）。");
            let _ = writeln!(rpt, "→ **窗口效应**：卖优势依赖特定子窗（很可能是下跌子窗做空），非结构性。否定性结果照实报。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 结果包六要素");
        let _ = writeln!(rpt, "1. **结论**：holdout level0卖 actual_pnl={hd_pnl:.4e}（{}），方向不对称={}。", if oos_robust { "OOS 初步稳健" } else { "winner's curse 否证" }, if asym_structural { "结构性候选" } else { "窗口效应" });
        let _ = writeln!(rpt, "2. **定义依据**：level0卖=663 in-sample 最强正类（level=0/δ=−1/顶背驰卖空反转腿）；actual_pnl=δ(Pτout−Pτin)−Ce 真实成交口径（664-Q3）。");
        let _ = writeln!(rpt, "3. **边界条件**：holdout 窗净涨跌={hold_ret:+.2}%；若 holdout 为下跌段（做空天然赚），OOS 正不足以证 alpha（需跨涨跌子窗，见任务2）。train_frac={train_frac} 改变切分点结论可能翻转（单切分脆弱）。");
        let _ = writeln!(rpt, "4. **下游推论**：{}", if oos_robust { "level0卖可作信号层 entry 候选，但须 L3 跨标的 + 涨段验证后才升基座（防方向效应）。" } else { "level0卖不可单独作 entry——in-sample 正是窗口产物，下游策略勿基于此类升基座。" });
        let _ = writeln!(rpt, "5. **谱系引用**：663 econpositive 推论；664 对象错配修复（δ=反转腿方向）；formalization-validity-domain（L2 有效域 < 定义域）；161（务实=把缺口留后面）。");
        let _ = writeln!(rpt, "6. **影响声明**：新增 Dataset::slice_bar_range（半开区间切片，复用 source_index 重置契约）+ acc_level0sell_oos 测试；不改 decompose_capturable_spread/TradeRecord/Order。");

        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econpositive-oos-level0sell-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：train+holdout 信号数之和与全窗同口径无丢失（半开切分无重叠无遗漏 ⟹ 级别涌现局部性除外，
        // 切窗会改变级别涌现 ⟹ 不强求 n 守恒；仅断言切片本身非空、actual_pnl 与逐信号和一致）。
        let recomputed_hold: f64 = decomps_hold.iter().filter(|d| d.level == 0 && d.delta == -1)
            .map(|d| d.actual_pnl).sum();
        assert!((hd_pnl - recomputed_hold).abs() < 1e-6, "holdout level0卖 Σactual_pnl 聚合不一致");
        assert!(ds_train.bars.len() + ds_hold.bars.len() == n, "train+holdout bar 数 ≠ 全窗（半开区间应无重叠无遗漏）");
    }

    fn winrate(pos: usize, n: usize) -> f64 {
        if n > 0 { 100.0 * pos as f64 / n as f64 } else { 0.0 }
    }

    /// **PDF §11 正确 OOS 验收（除 codex Q2 BIAS-FATAL 选择偏差）**：train-only 挑类 → 锁 holdout
    /// 评估 + 多窗滚动 walk-forward + neff + block bootstrap + 剔最大赢家。
    ///
    /// ★663 降级（判据错误更正）：neff/LCB/block_bootstrap/剔赢家/符号检验 = 统计显著性判据，
    /// 663 裁定非 alpha 判据（统计显著性误当可交易性，系统性惩罚低频高级别）。真判据 = 逐信号
    /// mu_hat>0（见 l2_btc_capturable_spread_diagnosis）。本测试保留作参考数据（过拟合诊断维度），
    /// 不作 alpha 确认判据。ponytail: 不删（663 明确可保留作参考），仅标注判据降级。
    ///
    /// 与 `acc_level0sell_oos`（被否证：从含 holdout 全样本挑 level0卖）的关键差异：
    /// **选类只用 train 段**（codex/PDF§6：用挑赢家同一数据验证不能反驳挑赢家）。train 赢家可能 ≠ level0卖。
    ///
    /// `#[ignore]`：同上需 BTC 全量 + O(n²) 重分类 ×（1 主切分 + K 滚动窗），`--release`。
    ///
    /// **L2 纪律（161/formalization-validity-domain）**：任一结果照实报——
    /// train 赢家=level0卖 且 holdout 正 ⟹ 缠论买卖点首个干净 OOS alpha 证据；
    /// train 赢家≠level0卖 或 holdout 翻负 ⟹ +7.87e3 是选择偏差产物（诚实否证，缩有效域）。
    #[test]
    #[ignore]
    fn acc_walkforward_trainonly() {
        use super::super::data;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS);
        let ds = if n_full > max_bars { ds_full.slice_bar_range(n_full - max_bars, n_full) } else { ds_full };
        let n = ds.bars.len();
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        // ══ 任务1+2：主切分 train-only 挑类 → 锁 holdout 评估（除 Q2 选择偏差核心）══
        let train_frac = std::env::var("ECON_L2_TRAIN_FRAC").ok()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.6);
        let split = (n as f64 * train_frac) as usize;
        let ds_train = ds.slice_bar_range(0, split);
        let ds_hold = ds.slice_bar_range(split, n);
        let split_day = ds.dates[split].get(..10).unwrap_or("").to_string();

        let (decomps_train, _) = decompose_capturable_spread(&ds_train, &config);
        let (decomps_hold, _) = decompose_capturable_spread(&ds_hold, &config);

        // train-only 挑赢家（不看 holdout）。记录是否 level0卖。
        let winner = train_winner_class(&decomps_train);
        let is_level0_sell = matches!(winner, Some((0, -1, _)));
        let train_l0sell = class_actual_pnl_agg(&decomps_train, 0, -1);

        // 锁 holdout 评估 train 选出的类。
        let (hd_n, hd_pnl, hd_pos, hd_ab, hd_pnls) = match winner {
            Some((lv, dl, bc)) => {
                let (n_, pnl, npos, ab) = class_actual_pnl(&decomps_hold, lv, dl, bc);
                let pnls: Vec<f64> = decomps_hold.iter()
                    .filter(|d| d.level == lv && d.delta == dl && d.bsp_class == bc).map(|d| d.actual_pnl).collect();
                (n_, pnl, npos, ab, pnls)
            }
            None => (0, 0.0, 0, 0.0, Vec::new()),
        };
        let oos_mu_positive = hd_pnl > 0.0 && winner.is_some(); // μ_OOS>0（弱：未扣不确定性）

        // 任务4：neff vs nraw（holdout 赢家类逐笔 PnL 自相关）。
        let nraw = hd_pnls.len();
        let (neff, sum_rho) = neff_autocorr(&hd_pnls, 20);
        let (mu_oos, se_oos, lcb_oos) = mean_se_lcb(&hd_pnls, neff);

        // 任务5/Q4：剔最大赢家 + block bootstrap p。
        let (q4_full, q4_d1, q4_d3, q4_d5) = drop_top_winners(&hd_pnls);
        let q4_p = block_bootstrap_pvalue(&hd_pnls, 20, 2000);

        // ══ 任务3：多窗滚动 walk-forward（除 Q5 单切分脆弱）══
        let k_windows: usize = std::env::var("ECON_WF_WINDOWS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(4);
        let wf_train_frac = 0.6;
        let win_len = n / k_windows;
        let mut wf_rows: Vec<(Option<(u32, i8, u8)>, bool, usize, f64, f64, bool)> = Vec::new();
        let mut lcb_pos_count = 0usize;
        let mut l0sell_winner_count = 0usize;
        for w in 0..k_windows {
            let w_start = w * win_len;
            let w_end = if w + 1 == k_windows { n } else { (w + 1) * win_len };
            let w_mid = w_start + ((w_end - w_start) as f64 * wf_train_frac) as usize;
            if w_mid <= w_start || w_end <= w_mid { continue; }
            let dtr = decompose_capturable_spread(&ds.slice_bar_range(w_start, w_mid), &config).0;
            let dos = decompose_capturable_spread(&ds.slice_bar_range(w_mid, w_end), &config).0;
            let wwin = train_winner_class(&dtr);
            let w_is_l0 = matches!(wwin, Some((0, -1, _)));
            if w_is_l0 { l0sell_winner_count += 1; }
            let (on, opnl, lcb) = match wwin {
                Some((lv, dl, bc)) => {
                    let pnls: Vec<f64> = dos.iter().filter(|d| d.level == lv && d.delta == dl && d.bsp_class == bc)
                        .map(|d| d.actual_pnl).collect();
                    let (nf, _) = neff_autocorr(&pnls, 20);
                    let (_, _, lcb) = mean_se_lcb(&pnls, nf);
                    (pnls.len(), pnls.iter().sum::<f64>(), lcb)
                }
                None => (0, 0.0, 0.0),
            };
            if lcb > 0.0 { lcb_pos_count += 1; }
            wf_rows.push((wwin, w_is_l0, on, opnl, lcb, opnl > 0.0));
        }
        let n_wf = wf_rows.len();
        let lcb_pos_ratio = if n_wf > 0 { 100.0 * lcb_pos_count as f64 / n_wf as f64 } else { 0.0 };

        // PDF §7.3 强判据 χ=1[LCB>θ]（θ=0）：μ>0 不够，须扣除不确定性后仍正。
        // §11 全验收 = LCB>0 ∧ Q4 剔最大5仍正 ∧ bootstrap p<0.05 ∧ 多窗 LCB>0 占比高。
        let oos_robust = lcb_oos > 0.0 && q4_d5 > 0.0 && q4_p < 0.05 && lcb_pos_ratio >= 100.0;

        // ══ 报告 ══
        let cls_str = |c: Option<(u32, i8, u8)>| c.map(|(l, d, b)| format!("(level={l},δ={d:+},cls={b:#04x})")).unwrap_or_else(|| "None(train无正类)".into());
        let mut rpt = String::new();
        let _ = writeln!(rpt, "# PDF §11 walk-forward 验收：train-only 挑类（除 codex Q2 BIAS-FATAL 选择偏差）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的，train-only 选类已除选择偏差；仍单标的，非 L3 跨品种）。");
        let _ = writeln!(rpt, "**与被否证 acc_level0sell_oos 的差异**：选类只用 train 段（PDF§6/codex：用挑赢家同一数据验证不能反驳挑赢家）。");
        let _ = writeln!(rpt, "**复算**：`cargo test -p <crate> --release acc_walkforward_trainonly -- --ignored --nocapture`（确定性，含固定种子 bootstrap）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据 / 窗口");
        let _ = writeln!(rpt, "- BTC 全量 {n_full} bar；截断窗 [{win_start}→{win_end}]，bars={n}（最后 {max_bars}，OOM 边界=显式有效域）。");
        let _ = writeln!(rpt, "- 主切分 train_frac={train_frac}，split={split}（{split_day}，半开 train=[0,{split}) / holdout=[{split},{n}) 无重叠）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务1+2：train-only 挑类 → 锁 holdout 评估（核心：除 Q2）");
        let _ = writeln!(rpt, "- **train 期最强正类 = {}**", cls_str(winner));
        let _ = writeln!(rpt, "- **train 赢家是否 level0卖？{}**（level0卖 train 期 Σpnl={:.4e}/n={}）", if is_level0_sell { "是" } else { "否（⟹ level0卖不是 train-only 赢家！）" }, train_l0sell.1, train_l0sell.0);
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "| holdout 评估 train 选定类 {} | 值 |", cls_str(winner));
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| holdout n (=nraw) | {hd_n} |");
        let _ = writeln!(rpt, "| **holdout Σactual_pnl** | **{hd_pnl:.4e}** |");
        let _ = writeln!(rpt, "| holdout 胜率 | {hd_pos}/{hd_n} ({:.0}%) |", winrate(hd_pos, hd_n));
        let _ = writeln!(rpt, "| holdout Σab_rev | {hd_ab:.4e} |");
        let _ = writeln!(rpt, "| μ_OOS | {mu_oos:.4e} |");
        let _ = writeln!(rpt, "| se（用 neff） | {se_oos:.4e} |");
        let _ = writeln!(rpt, "| **LCB（μ−1.645·se，单侧5%）** | **{lcb_oos:.4e}** |");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**判定**：holdout μ_OOS {} 0，**LCB {} 0**（PDF§7.3 强判据 χ=1[LCB>0]={}）", if mu_oos > 0.0 { ">" } else { "≤" }, if lcb_oos > 0.0 { ">" } else { "≤" }, lcb_oos > 0.0);
        if !is_level0_sell {
            let _ = writeln!(rpt, "→ **train 赢家变了（level0卖被否证为选择偏差产物）**：train-only 选出的是 {}，不是 level0卖。+7.87e3 是「从含 holdout 全样本挑 level0卖」的选择偏差产物（codex Q2/PDF§6 坐实）。", cls_str(winner));
        } else if !oos_mu_positive {
            let _ = writeln!(rpt, "→ **holdout 翻负（否证）**：level0卖虽是 train-only 赢家，但 holdout Σpnl≤0 ⟹ 无 OOS alpha。+7.87e3 是窗口/选择偏差产物（诚实否证，缩有效域，161）。");
        } else if lcb_oos > 0.0 {
            let _ = writeln!(rpt, "→ **Q2 消除 + μ_OOS 正 + LCB>0**：level0卖是 train-only 赢家、holdout μ 正且扣除不确定性（neff 修正）后仍正 ⟹ 初步通过 §7.3 强判据。但 §11 全验收还需 Q4/bootstrap/多窗全过（见下，综合判定在结果包）。仍 BTC 单标的 L2。");
        } else {
            let _ = writeln!(rpt, "→ **Q2 消除，但 μ_OOS 正而 LCB≤0（未通过 §7.3 强判据）**：level0卖确是 train-only 赢家（选择偏差消除，这一点为正），但 holdout μ_OOS={mu_oos:.2e} 扣除不确定性后 LCB={lcb_oos:.2e}≤0 ⟹ **+7.87e3 不达可交易阈值**。neff/nraw 见下（事件聚集致有效样本远小于 nraw）。这不是「干净 OOS 正」——是「选择偏差消除后，弱正信号被不确定性吞没」。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务4：neff vs nraw（PDF§6 条件二，自相关修正）");
        let _ = writeln!(rpt, "| 量 | 值 |");
        let _ = writeln!(rpt, "|---|---|");
        let _ = writeln!(rpt, "| nraw（holdout 赢家类原始信号数） | {nraw} |");
        let _ = writeln!(rpt, "| Σρk（k=1..20，仅正自相关） | {sum_rho:.4} |");
        let _ = writeln!(rpt, "| **neff = nraw/(1+2Σρk)** | **{neff:.1}** |");
        let _ = writeln!(rpt, "| neff/nraw | {:.2} |", if nraw > 0 { neff / nraw as f64 } else { 0.0 });
        let _ = writeln!(rpt, "（neff≪nraw ⟹ 事件高度聚集，有效检验力远低于原始 n；se 已用 neff）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务5/Q4：赢家集中度（剔最大赢家 + block bootstrap）");
        let _ = writeln!(rpt, "| 剔除 | 剩余 Σpnl | 仍正? |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| 不剔（全量） | {q4_full:.4e} | {} |", if q4_full > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 1 | {q4_d1:.4e} | {} |", if q4_d1 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 3 | {q4_d3:.4e} | {} |", if q4_d3 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "| 剔最大 5 | {q4_d5:.4e} | {} |", if q4_d5 > 0.0 { "是" } else { "否" });
        let _ = writeln!(rpt, "- **block bootstrap p（H0:μ≤0，block_len=20，B=2000，固定种子）= {q4_p:.4}**（p<0.05 ⟹ 正均值稳健远离 0）。");
        if q4_d5 <= 0.0 && q4_full > 0.0 {
            let _ = writeln!(rpt, "→ **Q4 坐实少数大赢家驱动**：剔最大 5 后转负 ⟹ holdout 正总和由极少数大赢家撑起，非稳健 alpha。");
        } else if q4_d5 > 0.0 {
            let _ = writeln!(rpt, "→ **Q4 排除少数大赢家**：剔最大 5 仍正 ⟹ 正收益非单一赢家驱动。");
        } else {
            let _ = writeln!(rpt, "→ holdout 全量已非正，Q4 剔赢家不适用（无正可剔）。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 任务3：多窗滚动 walk-forward（除 Q5 单切分脆弱，K={k_windows}）");
        let _ = writeln!(rpt, "每窗：窗内 train(前{:.0}%) 挑类 → OOS(后{:.0}%) 评估该类（每 train 只用窗内过去）。", wf_train_frac * 100.0, (1.0 - wf_train_frac) * 100.0);
        let _ = writeln!(rpt, "| 窗 | train赢家 | =level0卖? | OOS n | OOS Σpnl | OOS LCB | LCB>0? |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|");
        for (i, (w, isl0, on, opnl, lcb, _)) in wf_rows.iter().enumerate() {
            let _ = writeln!(rpt, "| {} | {} | {} | {on} | {opnl:.4e} | {lcb:.4e} | {} |",
                i + 1, cls_str(*w), if *isl0 { "是" } else { "否" }, if *lcb > 0.0 { "是" } else { "否" });
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "- **各窗 OOS LCB>0 占比 = {lcb_pos_count}/{n_wf} ({lcb_pos_ratio:.0}%)**");
        let _ = writeln!(rpt, "- level0卖为 train 赢家的窗数 = {l0sell_winner_count}/{n_wf}");
        if lcb_pos_ratio >= 100.0 && n_wf > 0 {
            let _ = writeln!(rpt, "→ 全窗 LCB>0 ⟹ Q5 单切分脆弱被排除，OOS 跨窗稳健正（仍单标的）。");
        } else if lcb_pos_count > 0 {
            let _ = writeln!(rpt, "→ 部分窗 LCB>0（{lcb_pos_ratio:.0}%）⟹ 非全窗稳健，单切分脆弱未完全排除（Q5 部分成立）。");
        } else {
            let _ = writeln!(rpt, "→ 无窗 LCB>0 ⟹ 扣除不确定性后无窗稳健正，OOS 不稳健（Q5 坐实）。");
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 结果包六要素（完整版）");
        let _ = writeln!(rpt, "1. **结论**：train-only 赢家={}（{}level0卖，Q2 选择偏差{}），holdout μ_OOS={mu_oos:.4e}/**LCB={lcb_oos:.4e}**；多窗 LCB>0 占比={lcb_pos_ratio:.0}%（{lcb_pos_count}/{n_wf}）；neff/nraw={:.2}（{neff:.0}/{nraw}）；剔最大5 {}；bootstrap p={q4_p:.3}。**综合判定：{}**",
            cls_str(winner), if is_level0_sell { "=" } else { "≠" },
            if is_level0_sell { "消除" } else { "坐实——非 train 赢家" },
            if nraw > 0 { neff / nraw as f64 } else { 0.0 },
            if q4_d5 > 0.0 { "仍正" } else { "转负" },
            if oos_robust { "通过 §11 全验收（缠论买卖点首个干净稳健 OOS alpha 证据，单标的 L2）" }
            else if is_level0_sell && oos_mu_positive { "Q2 选择偏差消除（level0卖确为 train-only 赢家），但 §11 稳健性验收未过——LCB≤0/少数大赢家驱动/多窗不稳之一以上成立 ⟹ +7.87e3 不达可交易 alpha 阈值（诚实否证，161/formalization-validity-domain）" }
            else { "level0卖 +7.87e3 是选择偏差/窗口产物（否证）" });
        let _ = writeln!(rpt, "2. **定义依据**：actual_pnl=δ(Pτout−Pτin)−Ce（664-Q3 真实成交口径）；train-only 挑类=PDF§11「train 决定规则」；neff=nraw/(1+2Σρk)=PDF§6 条件二；LCB=μ−1.645se=PDF§7.3。");
        let _ = writeln!(rpt, "3. **边界条件**：单标的 BTC L2——结论翻转条件：(a) L3 跨标的若 level0卖不普遍赢则 BTC 是品种特例；(b) train_frac/窗数 K 改变 train 赢家身份则选类不稳；(c) holdout 仍为下跌段则卖优势含方向效应。");
        let _ = writeln!(rpt, "4. **下游推论**：{}", if oos_robust { "level0卖可作信号层 entry 候选（§11 全验收过），但升基座仍需 L3 跨标的。" } else { "level0卖不可单独作 entry——§11 稳健性验收未过（LCB≤0/赢家集中/多窗不稳），下游策略勿基于 +7.87e3 升基座。Q2 消除只证「不是挑赢家产物」，不证「是可交易 alpha」——二者独立。" });
        let _ = writeln!(rpt, "5. **谱系引用**：663 econpositive；664-Q3 真实成交口径；codex Q2 BIAS-FATAL（codex-oos-level0sell-audit-20260630.md）；PDF§6/§11（overfit-consult-20260630.txt）；161（务实=留缺口）；formalization-validity-domain（L2 有效域<定义域）。");
        let _ = writeln!(rpt, "6. **影响声明**：新增 acc_walkforward_trainonly 测试 + train_winner_class/neff_autocorr/mean_se_lcb/drop_top_winners/block_bootstrap_pvalue helper（均纯函数，L1 自检 walkforward_helpers_l1）；复用 slice_bar_range/decompose_capturable_spread/class_actual_pnl；不改生产代码/TradeRecord/Order。被否证的 acc_level0sell_oos 保留（谱系：选择偏差的发生史）。");

        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econpositive-walkforward-20260630.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告 {out:?} 失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：切片无重叠无遗漏 + holdout 聚合一致 + neff≤nraw。
        assert!(ds_train.bars.len() + ds_hold.bars.len() == n, "train+holdout bar 数 ≠ 全窗（半开应无重叠无遗漏）");
        let recomputed: f64 = hd_pnls.iter().sum();
        assert!((hd_pnl - recomputed).abs() < 1e-6, "holdout 赢家类 Σpnl 聚合不一致");
        assert!(neff <= nraw as f64 + 1e-9, "neff 应 ≤ nraw");
    }

    // ── PDF §11 walk-forward 验收 helper（纯函数，L1 自检见 mod 末 #[test]）──

    /// train 段 per-class 选最强正类（PDF §11「train 决定规则」）：扫所有出现的 `(level, δ, bsp_class)`
    /// （P4/codex E-3：含类型 ⟹ 与 estimand 桶键一致，选类不跨 buy1/buy2/buy3 稀释），
    /// 取 Σactual_pnl 最大者；若全非正返 None（train 期无可交易候选）。
    /// **关键反偏差**：选类只用 train 段 decomps，holdout 信息不进入选择环节（消 codex Q2 BIAS-FATAL）。
    fn train_winner_class(decomps_train: &[SignalDecomp]) -> Option<(u32, i8, u8)> {
        use std::collections::BTreeMap;
        let mut sums: BTreeMap<(u32, i8, u8), f64> = BTreeMap::new();
        for d in decomps_train {
            *sums.entry((d.level, d.delta, d.bsp_class)).or_default() += d.actual_pnl;
        }
        sums.into_iter()
            .filter(|(_, pnl)| *pnl > 0.0)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(cls, _)| cls)
    }

    /// 有效样本数 neff = nraw/(1+2Σρk)（PDF §6 条件二）。ρk=逐笔 PnL 序列样本自相关，
    /// k=1..=lag_max，仅累加 ρk>0（正自相关才膨胀方差；负自相关不缩 neff 以保守）。
    /// 退化：n<2 或方差≈0 ⟹ neff=nraw（无相关信息）。返回 (neff, sum_rho_pos)。
    fn neff_autocorr(pnls: &[f64], lag_max: usize) -> (f64, f64) {
        let n = pnls.len();
        if n < 2 { return (n as f64, 0.0); }
        let mean = pnls.iter().sum::<f64>() / n as f64;
        let var = pnls.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        if var <= 1e-12 { return (n as f64, 0.0); }
        let mut sum_rho = 0.0;
        for k in 1..=lag_max.min(n - 1) {
            let cov: f64 = (0..n - k).map(|i| (pnls[i] - mean) * (pnls[i + k] - mean)).sum::<f64>() / n as f64;
            let rho = cov / var;
            if rho > 0.0 { sum_rho += rho; }
        }
        let neff = n as f64 / (1.0 + 2.0 * sum_rho);
        (neff.max(1.0).min(n as f64), sum_rho)
    }

    /// 单侧 5% LCB = μ − 1.645·se，se=σ/√neff（PDF §7.3，neff 而非 nraw）。返回 (mu, se, lcb)。
    fn mean_se_lcb(pnls: &[f64], neff: f64) -> (f64, f64, f64) {
        let n = pnls.len();
        if n == 0 { return (0.0, 0.0, 0.0); }
        let mu = pnls.iter().sum::<f64>() / n as f64;
        let var = pnls.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64;
        let se = if neff > 1.0 { (var / neff).sqrt() } else { f64::INFINITY };
        (mu, se, mu - 1.645 * se)
    }

    /// Q4 剔最大赢家：降序剔除 top-k 后剩余 Σ（>0 ⟹ 非少数大赢家驱动）。返回 (sum_full, sum_drop1, sum_drop3, sum_drop5)。
    fn drop_top_winners(pnls: &[f64]) -> (f64, f64, f64, f64) {
        let mut v = pnls.to_vec();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap()); // 降序
        let full: f64 = v.iter().sum();
        let drop = |k: usize| -> f64 { v.iter().skip(k.min(v.len())).sum() };
        (full, drop(1), drop(3), drop(5))
    }

    /// Block bootstrap 右尾 p-value（H0:μ≤0，移动块重采样保留逐笔 PnL 自相关）：
    /// 对原始序列做循环移动块重采样得 B 个 boot_mean，p = (#{boot_mean ≤ 0}+1)/(B+1)
    /// = 重采样分布落在 0 及以下的占比 ⟹ 正均值越稳健远离 0 则 p 越小（PDF §11 block bootstrap）。
    /// block_len 保块内时间相关（缠论同趋势段多信号相关，PDF §6）。确定性：固定 LCG（bit-exact 可复算）。
    fn block_bootstrap_pvalue(pnls: &[f64], block_len: usize, b_iters: usize) -> f64 {
        let n = pnls.len();
        if n < 2 { return 1.0; }
        let blk = block_len.max(1).min(n);
        let mut seed: u64 = 0x9E3779B97F4A7C15; // 固定种子（确定性，无外部 rng）
        let next = |seed: &mut u64| -> usize {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((*seed >> 33) as usize) % n
        };
        let mut le_zero = 0usize;
        for _ in 0..b_iters {
            let (mut acc, mut cnt) = (0.0f64, 0usize);
            while cnt < n {
                let start = next(&mut seed);
                for j in 0..blk {
                    if cnt >= n { break; }
                    acc += pnls[(start + j) % n]; // 循环移动块（原始序列，未居中）
                    cnt += 1;
                }
            }
            if acc / n as f64 <= 0.0 { le_zero += 1; }
        }
        (le_zero as f64 + 1.0) / (b_iters as f64 + 1.0)
    }

    /// b2 完整状态 Z 分桶键 L1 自检（task #83，升级自 W4 bsp_class 三键版）：
    /// 桶键 = 完整 [`MuClass`] z（含 I_γ + σ_p/role），同 (level,δ) 按 I_γ **和** σ_p 均细分。
    ///
    /// 五个合成信号：buy1×2 + buy2×1（同 σ_p=0 Root）验 I_γ 细分；再 buy1×2 中一条改 σ_p=+1 Child
    /// 验 σ_p 细分（b2 相对旧 Y=(level,δ,bsp_class) 的新增维——同 (0,+1,buy1) 按 σ_p 再分两桶）。
    #[test]
    fn bucket_key_is_full_z_l1() {
        use std::collections::BTreeMap;

        // z 由 (level,δ,bsp_class,parent_dir,position) 构（合成路径 horizontal=None，同一路径恒定不改分桶）。
        let mk = |bsp_class: u8, parent_dir: i8, position: PositionState, actual_pnl: f64| SignalDecomp {
            entry_bar: 0, exit_bar: 1, level: 0, delta: 1, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0,
            actual_pnl, sigma_higher: 0, bsp_class,
            z: MuClass::from_certificate(0, 1, BspBits::from_class_index(bsp_class), parent_dir, position),
            exit_decision: ExitDecision::Hold,
            trigger: NestTrigger::Type1TrendDivergence,
        };
        // buy1 Root ×2, buy2 Root ×1, buy1 Child(σ_p=+1) ×1：同 (level=0,δ=+1)，按 I_γ+σ_p 应分 3 桶。
        let decomps = vec![
            mk(0x01, 0, PositionState::Root, 1.0),
            mk(0x02, 0, PositionState::Root, 2.0),
            mk(0x01, 0, PositionState::Root, 3.0),
            mk(0x01, 1, PositionState::Child, 5.0), // σ_p=+1：与 buy1 Root 同 Y 桶、异 Z 桶（b2 细分）
        ];

        type Bucket = (usize, f64);
        let mut buckets: BTreeMap<MuClass, Bucket> = BTreeMap::new();
        for d in &decomps {
            let e = buckets.entry(d.z).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += d.actual_pnl;
        }

        assert_eq!(buckets.len(), 3, "buy1-Root / buy2-Root / buy1-Child(σ_p+1) 应分 3 桶（I_γ+σ_p 细分）");
        let buy1_root = MuClass::from_certificate(0, 1, BspBits::from_class_index(0x01), 0, PositionState::Root);
        let e = buckets.get(&buy1_root).expect("桶 buy1-Root 应存在");
        assert_eq!(e.0, 2, "buy1-Root 桶 n=2");
        assert!((e.1 - 4.0).abs() < 1e-9, "buy1-Root Σactual_pnl=1+3=4");
        let buy1_child = MuClass::from_certificate(0, 1, BspBits::from_class_index(0x01), 1, PositionState::Child);
        assert_eq!(buckets.get(&buy1_child).expect("桶 buy1-Child 应存在").0, 1,
            "buy1-Child(σ_p=+1) 独立成桶——b2 相对旧 Y 桶的新增 σ_p 细分维");
    }

    /// neff/LCB/bootstrap helper L1 自检（合成数据，验证算术，零信息增量但保非平凡逻辑不破）。
    #[test]
    fn walkforward_helpers_l1() {
        // train_winner：(0,-1) Σ=+5 最强，(1,1) Σ=−2 被滤。
        let synth = |level: u32, delta: i8, pnl: f64| SignalDecomp {
            entry_bar: 0, exit_bar: 1, level, delta, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0, actual_pnl: pnl,
            sigma_higher: 0, bsp_class: 0,
            z: MuClass::from_certificate(level, delta, BspBits::from_class_index(0), 0, PositionState::Root),
            exit_decision: ExitDecision::Hold,
            trigger: NestTrigger::XiaoZhuanDa,
        };
        let ds = vec![synth(0, -1, 3.0), synth(0, -1, 2.0), synth(1, 1, -2.0)];
        assert_eq!(train_winner_class(&ds), Some((0, -1, 0)), "train 应选 Σpnl 最大正类 (0,-1,cls=0)");
        let all_neg = vec![synth(0, -1, -1.0)];
        assert_eq!(train_winner_class(&all_neg), None, "全非正应返 None");

        // neff 边界：始终 ∈[1, nraw]。块状正相关 ⟹ neff<nraw（事件聚集缩有效样本，PDF§6）。
        let pos_corr = vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]; // 块状正相关 lag1>0
        let (neff_pc, sr_pc) = neff_autocorr(&pos_corr, 4);
        assert!(neff_pc < 8.0 && neff_pc >= 1.0 && sr_pc > 0.0,
            "正自相关应缩 neff∈[1,nraw)，实得 neff={neff_pc} sr={sr_pc}");
        // 常量序列方差≈0 ⟹ 退化 neff=nraw（无相关信息可缩）。
        let (neff_const, _) = neff_autocorr(&[2.0; 6], 4);
        assert!((neff_const - 6.0).abs() < 1e-9, "零方差退化 neff=nraw，实得 {neff_const}");

        // LCB：μ>0 但 se 大 ⟹ LCB 可能<0（压制噪声赢家）。
        let (mu, se, lcb) = mean_se_lcb(&[2.0, -1.0, 3.0, -2.0], 4.0);
        assert!((mu - 0.5).abs() < 1e-9 && se > 0.0 && lcb < mu, "LCB=μ−1.645se<μ，实得 mu={mu} lcb={lcb}");

        // drop_top：[10,1,1,1] 剔最大后 Σ=3>0（非单一赢家）；[10,-1,-1,-1] 剔后 Σ=−3<0（单一赢家驱动）。
        let (f1, d1, _, _) = drop_top_winners(&[10.0, 1.0, 1.0, 1.0]);
        assert!((f1 - 13.0).abs() < 1e-9 && (d1 - 3.0).abs() < 1e-9, "剔最大后剩余");
        let (_, d1b, _, _) = drop_top_winners(&[10.0, -1.0, -1.0, -1.0]);
        assert!(d1b < 0.0, "单一大赢家驱动：剔后转负");

        // bootstrap：强正信号 p 小；纯噪声 p 接近 0.5（确定性，固定种子）。
        let strong_pos = vec![5.0; 20];
        let p_pos = block_bootstrap_pvalue(&strong_pos, 3, 500);
        assert!(p_pos <= 0.05, "强正信号 block bootstrap p 应小，实得 {p_pos}");
    }

    /// **全历史多级别信号分布 + level2+ 逐级别 train-only holdout μ̂/LCB/p（acc-multilevel-sample + acc-highlevel-mu）**。
    ///
    /// 同一次全量跑（ECON_L2_MAX_BARS=5000000，BTC 461万 bar）产出两条 acceptance 所需数据，避免重跑。
    ///
    /// ## acc-multilevel-sample（信号数分布）
    /// 输出 level0-5 全历史信号数分布表。Le Cam 硬墙判定：
    /// - level3+ 全历史 n < 30 ⟹ 结构性稀疏（高级别本身低频，Le Cam 硬墙；有效域 < 定义域）。
    /// - level3+ n ≥ 50 ⟹ 窗口截断证实可 OOS 验（若截断窗则为截断窗结论，全量则为全历史结论）。
    ///
    /// ## acc-highlevel-mu（level2+ 逐级别 train-only holdout alpha）
    /// 对 level>=2 的每个 (level,δ)：
    /// - train_frac=0.6 按 bar 顺序切分（半开区间无重叠）。
    /// - train 段全量跑 decompose，holdout 段全量跑 decompose（**不**从 train-only 挑类——每个 (level,δ) 独立评估）。
    /// - holdout neff_autocorr + mean_se_lcb (LCB 单侧5%) + block_bootstrap_pvalue(block=20, B=2000)。
    /// - 照实报 LCB>0 占比 + p 值（否定性结果 = 有效域收窄，161/formalization-validity-domain）。
    ///
    /// **认识论等级 L2**（真实数据单标的，可产否定性结果，非 L3 跨品种）。
    /// **testing-override 生成态例外**：测试目的是产出可证伪经济结果，不是通过。
    ///
    /// `#[ignore]`：需 BTC 全量数据（314M）+ O(n²) 重分类 × 2（train/holdout 各跑一次），`--release`。
    /// 命令：`ECON_L2_MAX_BARS=5000000 cargo test --release acc_multilevel_highlevel_mu -- --ignored --nocapture`。
    #[test]
    #[ignore]
    fn acc_multilevel_highlevel_mu() {
        use super::super::data;
        use std::collections::BTreeMap;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();

        // 显式有效域：全量为 ECON_L2_MAX_BARS=5000000（>4.6M=不截断）。
        const MAX_BARS_DEFAULT: usize = 300_000; // 默认截断窗（时间墙保护）
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS_DEFAULT);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let n = ds.bars.len();
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        eprintln!("全量跑：bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");

        // ── Step1：全窗跑一次，产 level0-5 信号数分布表（acc-multilevel-sample）。 ──
        eprintln!("[Step1] 全窗 decompose（信号分布表）...");
        let (decomps_full, agg_full) = decompose_capturable_spread(&ds, &config);
        eprintln!("[Step1] 完成：n_signals={}", agg_full.n_signals);

        // per-(level,δ) 信号数 + μ̂（全窗，仅用于信号分布表，不作 OOS alpha）。
        let mut dist: BTreeMap<(u32, i8), (usize, f64)> = BTreeMap::new();
        for d in &decomps_full {
            let e = dist.entry((d.level, d.delta)).or_default();
            e.0 += 1;
            e.1 += d.actual_pnl;
        }
        let max_level = dist.keys().map(|(l, _)| *l).max().unwrap_or(0);

        // ── Step2：train/holdout 切分，分别跑 decompose（acc-highlevel-mu 防选择偏差）。 ──
        let train_frac = std::env::var("ECON_L2_TRAIN_FRAC").ok()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.6);
        let split = (n as f64 * train_frac) as usize;
        let split_day = if split < ds.dates.len() {
            ds.dates[split].get(..10).unwrap_or("").to_string()
        } else { "末端".to_string() };
        eprintln!("[Step2] train=[0,{split})，holdout=[{split},{n})，切分日={split_day}");

        let ds_train = ds.slice_bar_range(0, split);
        let ds_hold  = ds.slice_bar_range(split, n);
        let (decomps_train, _) = decompose_capturable_spread(&ds_train, &config);
        let (decomps_hold,  _) = decompose_capturable_spread(&ds_hold,  &config);
        eprintln!("[Step2] train n_signals={}，holdout n_signals={}", decomps_train.len(), decomps_hold.len());

        // level2+ 每个 (level,δ)：holdout 段的 μ̂/neff/LCB/p。
        // 遍历全窗出现过的 level>=2 的 (level,δ)（train 中未出现的 holdout 也可能出现，照报）。
        let high_keys: Vec<(u32, i8)> = {
            let mut ks: std::collections::BTreeSet<(u32, i8)> = Default::default();
            for d in decomps_full.iter().chain(decomps_train.iter()).chain(decomps_hold.iter()) {
                if d.level >= 2 { ks.insert((d.level, d.delta)); }
            }
            ks.into_iter().collect()
        };

        // per-(level,δ) holdout 评估（train 段只用于「知道该 level 是否出现」，不挑赢家——每级独立评估）。
        struct HighLevelResult {
            level: u32,
            delta: i8,
            tr_n: usize,
            tr_pnl: f64,
            hd_n: usize,
            hd_pnl: f64,
            hd_pos: usize,
            mu_oos: f64,
            se_oos: f64,
            lcb: f64,
            neff: f64,
            nraw: usize,
            sum_rho: f64,
            drop_d5: f64,
            bootstrap_p: f64,
        }
        let mut hl_results: Vec<HighLevelResult> = Vec::new();
        for &(lv, dl) in &high_keys {
            let (tr_n, tr_pnl, _, _) = class_actual_pnl_agg(&decomps_train, lv, dl);
            let (hd_n, hd_pnl, hd_pos, _) = class_actual_pnl_agg(&decomps_hold, lv, dl);
            let pnls: Vec<f64> = decomps_hold.iter()
                .filter(|d| d.level == lv && d.delta == dl)
                .map(|d| d.actual_pnl).collect();
            let nraw = pnls.len();
            let (neff, sum_rho) = neff_autocorr(&pnls, 20);
            let (mu_oos, se_oos, lcb) = mean_se_lcb(&pnls, neff);
            let (_, _, _, drop_d5) = drop_top_winners(&pnls);
            let bootstrap_p = block_bootstrap_pvalue(&pnls, 20, 2000);
            hl_results.push(HighLevelResult {
                level: lv, delta: dl, tr_n, tr_pnl, hd_n, hd_pnl, hd_pos,
                mu_oos, se_oos, lcb, neff, nraw, sum_rho, drop_d5, bootstrap_p,
            });
        }

        // ── 报告 ──
        let mut rpt = String::new();
        let _ = writeln!(rpt, "# acc-multilevel-sample + acc-highlevel-mu：BTC 全历史多级别 alpha 检验");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实数据单标的 BTC 2017-2026 全历史，可产否定性结果，非 L3 跨品种）。");
        let _ = writeln!(rpt, "**目的**：同一次全量跑产出两条 acceptance 判定，避免重跑。");
        let _ = writeln!(rpt, "**复算**：`ECON_L2_MAX_BARS=5000000 cargo test --release acc_multilevel_highlevel_mu -- --ignored --nocapture`");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 数据");
        let _ = writeln!(rpt, "- 品种：BTC（btc_1m_full.json，全量 {n_full} bar，2017-08→2026-05）");
        let _ = writeln!(rpt, "- 本次窗口：bars={n}（{win_start}→{win_end}，max_bars={max_bars}）");
        let _ = writeln!(rpt, "- train_frac={train_frac}，切分 bar={split}（{split_day}），train=[0,{split}) / holdout=[{split},{n}) 无重叠");
        let _ = writeln!(rpt, "- 全窗 n_signals={}", agg_full.n_signals);
        let _ = writeln!(rpt);

        // ── acc-multilevel-sample：信号数分布表 ──
        let _ = writeln!(rpt, "## acc-multilevel-sample：level0-{max_level} 全历史信号数分布");
        let _ = writeln!(rpt, "Le Cam 硬墙判定（level3+）：n<30 ⟹ 结构性稀疏；n≥50 ⟹ 可 OOS 验。");
        let _ = writeln!(rpt, "| level | δ | n（全窗） | μ̂=Σactual_pnl/n（全窗，含选择偏差，仅参考） | Le Cam 判定（level3+）|");
        let _ = writeln!(rpt, "|---|---|---|---|---|");
        for ((lvl, dlt), (n_cls, pnl_cls)) in &dist {
            let mu = if *n_cls > 0 { pnl_cls / *n_cls as f64 } else { 0.0 };
            let lecam = if *lvl >= 3 {
                if *n_cls < 30 { "结构性稀疏（<30，Le Cam 硬墙）" }
                else if *n_cls >= 50 { "可 OOS 验（≥50）" }
                else { "边界（30-49）" }
            } else { "—" };
            let _ = writeln!(rpt, "| {lvl} | {dlt:+} | {n_cls} | {mu:.4e} | {lecam} |");
        }
        let _ = writeln!(rpt);

        // acc-multilevel-sample 判定
        let high_level_sparse = {
            let level3plus: Vec<_> = dist.iter().filter(|((l, _), _)| *l >= 3).collect();
            level3plus.iter().all(|(_, (n, _))| *n < 30)
        };
        let _ = writeln!(rpt, "**acc-multilevel-sample 判定**：level3+ 全历史 n 分布——");
        if high_level_sparse {
            let _ = writeln!(rpt, "→ **level3+ 均 <30（Le Cam 硬墙成立）**：高级别结构性稀疏，全历史长窗也无足够样本做 OOS 验（有效域：高级别 alpha 可存在但不可功效检验）。");
        } else {
            let _ = writeln!(rpt, "→ **level3+ 存在 ≥30 的级别**：窗口截断导致的稀疏已排除（全量足够），level3+ 具备 OOS 验证能力。");
        }
        let _ = writeln!(rpt);

        // ── acc-highlevel-mu：level2+ 逐级别 holdout alpha ──
        let _ = writeln!(rpt, "## acc-highlevel-mu：level2+ 逐级别 train-only holdout μ̂/LCB/p");
        let _ = writeln!(rpt, "train_frac={train_frac}，切分日 {split_day}。每个 (level,δ) 独立评估（不从 train-only 挑类——无跨级别选择偏差）。");
        let _ = writeln!(rpt, "LCB=μ−1.645·se（单侧5%，se 用 neff），block bootstrap p（H0:μ≤0，block=20，B=2000，固定种子）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "| level | δ | train_n | train_Σpnl | holdout_n | holdout_Σpnl | μ̂_OOS | se | LCB | neff/nraw | drop_d5 | bootstrap_p | LCB>0? |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        let mut lcb_pos_count = 0usize;
        let mut total_hl = 0usize;
        for r in &hl_results {
            let wr = |pos: usize, n: usize| if n > 0 { format!("{pos}/{n} ({:.0}%)", 100.0 * pos as f64 / n as f64) } else { "0/0".to_string() };
            let neff_ratio = if r.nraw > 0 { format!("{:.2}/{}", r.neff, r.nraw) } else { "—".to_string() };
            let _ = writeln!(rpt, "| {} | {:+} | {} | {:.4e} | {} | {:.4e} | {:.4e} | {:.4e} | **{:.4e}** | {} | {:.4e} | {:.3} | {} |",
                r.level, r.delta, r.tr_n, r.tr_pnl, r.hd_n, r.hd_pnl,
                r.mu_oos, r.se_oos, r.lcb, neff_ratio, r.drop_d5, r.bootstrap_p,
                if r.lcb > 0.0 { "✓" } else { "✗" });
            let _ = writeln!(rpt, "  胜率={}", wr(r.hd_pos, r.hd_n));
            total_hl += 1;
            if r.lcb > 0.0 { lcb_pos_count += 1; }
        }
        let _ = writeln!(rpt);
        let lcb_pos_ratio = if total_hl > 0 { 100.0 * lcb_pos_count as f64 / total_hl as f64 } else { 0.0 };
        let _ = writeln!(rpt, "**acc-highlevel-mu 判定**：");
        let _ = writeln!(rpt, "- level2+ holdout LCB>0 占比 = {lcb_pos_count}/{total_hl} ({lcb_pos_ratio:.0}%)");

        // 判定逻辑：LCB>0 ⟹ 高级别 holdout alpha 成立；LCB≤0 ⟹ 否证（照实报）
        if lcb_pos_count == 0 {
            let _ = writeln!(rpt, "→ **否证（level2+ 全部 LCB≤0）**：高级别 holdout μ̂ 扣除不确定性后无一正 ⟹ 高级别无稳健 OOS alpha（否定性结果，缩小有效域边界，161/formalization-validity-domain）。");
            let _ = writeln!(rpt, "→ 否定性结果价值：高级别 alpha 不存在于 BTC 单标的 L2，仅低级别候选（level0/1 待 L3 验）。");
        } else if lcb_pos_count == total_hl {
            let _ = writeln!(rpt, "→ **全部 LCB>0**：level2+ 每个 (level,δ) holdout μ̂ 均通过 §7.3 强判据 ⟹ 高级别具备稳健 OOS alpha 候选（仍 BTC 单标的 L2，需 L3 跨品种确认）。");
        } else {
            let _ = writeln!(rpt, "→ **部分 LCB>0（{lcb_pos_count}/{total_hl}）**：高级别 alpha 非全域稳健——部分 (level,δ) 通过强判据，其余否证。分别报告（见上表）。");
        }
        let _ = writeln!(rpt);

        // ── 结果包六要素 ──
        let _ = writeln!(rpt, "## 结果包六要素");
        let _ = writeln!(rpt, "1. **结论**：");
        let _ = writeln!(rpt, "   - acc-multilevel-sample：level0-{max_level} 全历史信号数分布见上表。level3+ {}。",
            if high_level_sparse { "全部 <30（Le Cam 硬墙，结构性稀疏）" } else { "存在 ≥30（可 OOS 验）" });
        let _ = writeln!(rpt, "   - acc-highlevel-mu：level2+ holdout LCB>0 占比={lcb_pos_count}/{total_hl}（{lcb_pos_ratio:.0}%）。{}",
            if lcb_pos_count == 0 { "全否证，高级别无稳健 OOS alpha（L2）。" }
            else { "部分或全部通过 §7.3 强判据（见明细表）。" });
        let _ = writeln!(rpt, "2. **定义依据**：actual_pnl=δ(Pτout−Pτin)−Ce（664-Q3 真实成交口径）；LCB=μ−1.645se（PDF §7.3，se 用 neff_autocorr 修正）；train-only 每级独立评估（无跨级选择偏差）；Le Cam 硬墙判定（n<30）。");
        let _ = writeln!(rpt, "3. **边界条件**：单标的 BTC L2——结论翻转条件：(a) max_bars={max_bars}（若<461万则为截断窗结论）；(b) train_frac={train_frac} 改变切分点结论可能变化；(c) 高级别 n 过少（Le Cam 约束）时 LCB 由 se 主导而非 μ；(d) 不同 bootstrap 种子理论上可影响 p（本实现固定 LCG 种子确定性）。");
        let _ = writeln!(rpt, "4. **下游推论**：若 level2+ 全否证 ⟹ 策略信号层仅可依赖 level0/1（需 L3 跨品种升基座）。若存在 LCB>0 ⟹ 该 (level,δ) 可作 entry 候选（仍需 L3）。否定性结果意味着高级别配额应为 0 或纯结构性（不依赖 alpha 预期）。");
        let _ = writeln!(rpt, "5. **谱系引用**：663 econpositive（可交易性=μ(z,a)>0）；664-Q3（真实成交口径）；formalization-validity-domain（L2 有效域<定义域，L3 待 L2 完成后）；testing-override 生成态例外（测试目的=产出可证伪结果，非通过）；161（否定性结果照实报）。不确定是否有 level2+ alpha 的独立谱系记录，保守声明无关联已知谱系。");
        let _ = writeln!(rpt, "6. **影响声明**：新增 acc_multilevel_highlevel_mu 测试；复用 decompose_capturable_spread/class_actual_pnl/neff_autocorr/mean_se_lcb/drop_top_winners/block_bootstrap_pvalue；不改任何生产代码/TradeRecord/Order/信号收集逻辑。报告落盘 econ-multilevel-mu-20260701.md。");

        // 落盘
        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/econ-multilevel-mu-20260701.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封：切片无重叠 + neff≤nraw 逐结果。
        assert_eq!(ds_train.bars.len() + ds_hold.bars.len(), n,
            "train+holdout bar 数 ≠ 全窗（半开区间应无重叠无遗漏）");
        for r in &hl_results {
            assert!(r.neff <= r.nraw as f64 + 1e-9,
                "neff({:.1}) 应 ≤ nraw({})：level={} δ={}", r.neff, r.nraw, r.level, r.delta);
            // holdout Σpnl 与逐信号和一致（聚合完整性）。
            let recomp: f64 = decomps_hold.iter()
                .filter(|d| d.level == r.level && d.delta == r.delta)
                .map(|d| d.actual_pnl).sum();
            assert!((r.hd_pnl - recomp).abs() < 1e-6,
                "holdout Σpnl 聚合不一致：level={} δ={}", r.level, r.delta);
        }
    }

    /// **Task #7 acc-classification：level 塔中间级空洞 H1/H2/H3 判别（诊断，不预设）**。
    ///
    /// 反演：Π_full 全历史 level 分布 7741→0→0→0→0→1 高度反常（level1-4 全空但 level5=1）。
    /// w-verify 提交 8d45dc449b 假设真稀疏(H1)但**未做判别实验**。本测试三路 instrument 信号收集循环
    /// （bit-exact 复制 decompose_capturable_spread 的收集路径），在 N^δ 门**前后**分级别计数：
    ///
    /// | 计数点 | 语义 | 判别 |
    /// |---|---|---|
    /// | (a) bsp_pre[lvl] | classifier 该 level 提取的 bsp 总数（门前，按第一/二/三类分） | 门前是否有信号 |
    /// | (b) gamma_nonflat[lvl] | Γ 组装非 Flat 候选数（N^δ 门前） | dir 是否可判 |
    /// | (c) sig_post[lvl] | 通过 N^δ 门 push 进 signals 的数（门后） | 门滤了多少 |
    /// | (d) tower_segs[lvl] | tower[lvl] 段数（结构存在性） | level 塔是否构造到该级 |
    ///
    /// **判别规则**（三路，不预设 H1/H2 二分）：
    /// - 中间级 bsp_pre>0 但 sig_post=0 ⟹ **H1**（N^δ 门滤空——[J_{ℓ-1}⊆J_ℓ] 嵌套链严格）。
    /// - 中间级 bsp_pre=0 ⟹ **H3**（架构：原「上级层只产第二类 B2/S2」前提已被裁定A+#123 作废——
    ///   级别≥1 现产一/二/三类；若仍 bsp_pre=0 则是该窗结构稀疏，非提取禁闭）。
    /// - 300K bsp_pre/sig_post≫0 但全历史=0（跨窗对比，两次跑）⟹ **H2**（全历史路径 bug）。
    ///
    /// **结构 sanity（team-lead 要求③）**：level5 唯一信号的 rungs 链——它的 tower 各级 rung 存在吗？
    /// 若 level5 结构含 level1-4 子结构却没算作 level1-4 信号 = 计数 bug。本测试 dump level5 信号的
    /// N^δ 证书 rungs 层数 + 各级 cand。
    ///
    /// **认识论 L2**：真实 BTC 数据逐信号分级别计数，可产否定性结果（H1 缩有效域 / H2 复活高级别）。
    /// `#[ignore]`：需 BTC 全量 + O(n²) 重分类，`--release`。
    /// 命令：`ECON_L2_MAX_BARS=<N> cargo test --release acc_classification_level_hole_dx -- --ignored --nocapture`
    /// （默认 300K；跑全历史用 ECON_L2_MAX_BARS=5000000）。
    #[test]
    #[ignore]
    fn acc_classification_level_hole_dx() {
        use super::super::data;
        use super::super::super::classifier::divergence::compute_macd;
        use super::super::super::strategy::interp::assemble_gamma_with_tower;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::Side;
        use super::super::incremental::IncrementalClassifier;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS_DEFAULT: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS_DEFAULT);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let bars = &ds.bars;
        let n = bars.len();
        let tick = config.tick.tick_size;
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        eprintln!("[level-hole-dx] bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");

        let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
        let macd_hist = compute_macd(&closes, &config.macd).hist;

        // 分级别计数器（L=8 上限足够；实测最高 level5）。
        const LMAX: usize = 8;
        let mut bsp_pre = [0usize; LMAX];      // (a) 门前 bsp 提取总数
        let mut bsp_pre_first = [0usize; LMAX]; // 门前第一类（buy1/sell1）
        let mut bsp_pre_second = [0usize; LMAX]; // 门前第二类（buy2/sell2）
        let mut bsp_pre_third = [0usize; LMAX]; // 门前第三类（buy3/sell3）
        let mut gamma_nonflat = [0usize; LMAX]; // (b) Γ 非 Flat 候选
        let mut sig_post = [0usize; LMAX];     // (c) 通过 N^δ 门（bsp/Γ 通道，PanDiv 外计）
        // Q4（#147）：PanDiv 承接通过门计数（Γ 外通道——不经 assemble_gamma，混入 sig_post 会
        // 污染 H1 门滤诊断的 gamma_nonflat−sig_post 列）。真封①口径 = sig_post + sig_post_pan。
        let mut sig_post_pan = [0usize; LMAX];
        let mut tower_segs_max = [0usize; LMAX]; // (d) tower[lvl] 段数（末次分类快照）
        let mut levels_seen_max = 0usize;       // cls.levels.len() 最大值

        // 结构 sanity：level≥1 通过门的信号，记其 (source_index, δ, bits, rung 层数)。
        let mut highlevel_hits: Vec<(usize, usize, i8, u8, usize)> = Vec::new(); // (lvl, src, δ, bits_u8, n_rungs)

        // ── P1 FullNest 验收（task #22）：通过门信号的**有效跨级深度**分布（含 level0）。 ──
        // effective_nest_depth(cert) = n_delta 递归实际穿越的连续 cand=true 跨级层数。
        // depth==0 ⟹ 纯 base-case confirm_side（退化，等价单 bit，无区间套跨级触达）；
        // depth>=1 ⟹ 真跨级 [J_{ℓ-1}⊆J_ℓ] 触达（≥2 层证书 e→ℓ）。用 build_nest_certificate
        // （与生产门共用构造）⟹ bit-exact 同源，非外部近似重算。索引=深度（LMAX 足够，实测塔≤6 级）。
        let mut nest_depth_hist_pass = [0usize; LMAX + 1]; // 通过门信号（n_delta=true）的有效深度分布
        let mut nest_depth_by_level_pass = [[0usize; LMAX + 1]; LMAX]; // [exec_level][depth]
        let mut n_gate_pass_total = 0usize; // 通过门信号总数（应=sig_post_sum）
        let mut n_xzd_pass = 0usize; // 小转大通道通过（无区间套 depth，设计 §2.3）——depth 直方图外计
        // StructBreak 收紧测量（task #62，codex 终局裁决A）：零 bit（bsp_class==0）候选中通过门的条数
        // ——这批此前经旧 else=>Type3 分派可能走 Nest/Xzd 通过门，收紧后 bsp_cand_type 恒 StructBreak
        // ⟹ build_gate_certificate 恒 None ⟹ 本计数器在新代码下恒为 0（收紧生效的直接证据）。
        let mut n_zerobit_gate_pass = 0usize;
        // 小转大分项（消歧「严格门真 0」vs「C3 死门伪影」）：路由到 Xzd 的总数 + C2/C3 单项命中。
        let mut n_xzd_routed = 0usize; // build_gate_certificate 返回 Xzd 的信号数（=小转大域触达）
        let mut n_xzd_c2 = 0usize;     // 其中 C2（type2_confirmed）成立
        let mut n_xzd_c3 = 0usize;     // 其中 C3（sub_last_zs_type3，诊断字段，不参门）成立
        // ── C3 死门诊断探针（codex 终局裁定 §5.4，task #41）：按 level 聚合 + lvl==1/lvl>=2 分裂断点 ──
        let mut xzd_routed_by_level = [0usize; LMAX];
        let mut xzd_c2_by_level = [0usize; LMAX];
        let mut xzd_c3_by_level = [0usize; LMAX]; // = same_side_same_center
        let mut xzd_l1_last_zs_exists = 0usize;
        let mut xzd_l1_same_side_l0_type3_any = 0usize;
        let mut xzd_l1_same_center_any = 0usize;
        let mut xzd_l1_same_side_causal_ok = 0usize;
        let mut xzd_lge2_sub_bsp_type3_total = 0usize; // 死门重封（裁定A+#123）：lvl>=2 sub_bsp 可含 Type3，锁基线
        // C3 新判据命中率探针（task #47，codex #44(c) 终局裁定）：level==1 子集「新中枢+突破」命中率。
        let mut xzd_l1_c3_new_center_exists = 0usize;
        let mut xzd_l1_c3_new_center_breakout_ok = 0usize;
        // XZD C2-only 口径复审探针（task #170，#148 后 C3 脱 0）：lvl>=2 子集同两字段命中数——
        // 供「level>=2 C2-only 保留 vs C3 硬门全 level 启用」codex 裁定的量化输入（报数不裁断，
        // 无断言不参门；#41 裁定 level>=2 C2-only 的前提是旧判据结构性死门，#148 升级重切后该
        // 前提在 level==1 已实证失效，level>=2 是否同失效由本探针测量）。
        let mut xzd_lge2_c3_new_center_exists = 0usize;
        let mut xzd_lge2_c3_new_center_breakout_ok = 0usize;
        // C3 L1 零命中根因判别探针（codex #55 终局裁定(5)，task #56）：默认关闭，只读旁路
        // （不写 Classification.levels[*].centers/tower/正常输出）。开关：ECON_C3_OVERLAP_PROBE=1。
        let overlap_probe_enabled = std::env::var("ECON_C3_OVERLAP_PROBE").ok().as_deref() == Some("1");
        let mut xzd_l1_overlap_rows: Vec<(usize, usize, usize, usize, Option<(usize, usize)>)> = Vec::new();

        let mut classifier_incr = IncrementalClassifier::new(bars, &config);
        let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
        // C1：dx 手写门循环复用为 collect_signals 的对拍源——门后 push 与生产同序同字段的信号元组。
        let mut signals_dx: Vec<RawSignal> = Vec::new();
        // Q4（task #145）：盘整背驰承接 dx 镜像去重集（与生产 seen_pan 同键同序）。
        let mut seen_pan_dx: std::collections::HashSet<(usize, usize, u8)> =
            std::collections::HashSet::new();
        // C2：与生产 collect_signals 同改——Rc::ptr_eq 命中跳级（同 Rc ⟹ 全键已 seen ⟹ 内层
        // bsp_pre/gamma_nonflat/sig_post 等计数均在 seen.insert 成功后，跳过 bit-exact）。
        let mut prev_bsp = Vec::new();

        for i in 0..n {
            let bar = &bars[i];
            if bar.untradable || bar.close <= 0 {
                continue;
            }
            let (cls_i, tower_i) = classifier_incr.classify_at(i);
            levels_seen_max = levels_seen_max.max(cls_i.levels.len());
            for (l, rc) in tower_i.iter().enumerate() {
                if l < LMAX {
                    tower_segs_max[l] = tower_segs_max[l].max(rc.len());
                }
            }
            for (lvl, ls) in cls_i.levels.iter().enumerate() {
                if prev_bsp.get(lvl).map_or(false, |prev| Rc::ptr_eq(prev, &ls.bsp)) {
                    continue; // C2：同 Rc ⟹ 全键已 seen，跳级
                }
                if lvl < prev_bsp.len() {
                    prev_bsp[lvl] = Rc::clone(&ls.bsp);
                } else {
                    prev_bsp.push(Rc::clone(&ls.bsp));
                }
                for p in ls.bsp.iter() {
                    let bsp_class = bsp_disc(&p.bits);
                    if !seen.insert((lvl, p.source_index, bsp_class)) {
                        continue;
                    }
                    if lvl < LMAX {
                        bsp_pre[lvl] += 1;
                        if p.bits.buy1 || p.bits.sell1 { bsp_pre_first[lvl] += 1; }
                        if p.bits.buy2 || p.bits.sell2 { bsp_pre_second[lvl] += 1; }
                        if p.bits.buy3 || p.bits.sell3 { bsp_pre_third[lvl] += 1; }
                    }
                    // Γ 组装（bit-exact 复制生产路径）。
                    let single = super::super::super::classifier::Classification {
                        levels: cls_i.levels.iter().enumerate()
                            .map(|(l2, _)| super::super::super::classifier::LevelState {
                                moves: Vec::new(), centers: Rc::new(Vec::new()),
                                bsp: Rc::new(if l2 == lvl { vec![p.clone()] } else { Vec::new() }),
                                pan_div: Rc::new(Vec::new()), // Q4：dx 与生产 single 同形（无盘整背驰载荷）
                            })
                            .collect(),
                    };
                    let sigma_higher = sigma_higher_at(&tower_i, bars, lvl); // 666 号：与生产 collect_signals 同口径
                    for c in &assemble_gamma_with_tower(&single, &tower_i) {
                        if c.dir == VoiceSide::Flat { continue; }
                        if lvl < LMAX { gamma_nonflat[lvl] += 1; }
                        let delta_side = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => continue,
                        };
                        // 二通道准入门（与生产 collect_signals 同源）：Nest→n_delta+depth；Xzd→gate_pass（无 depth）。
                        let sub_centers: &[Center] = if lvl > 0 { &cls_i.levels[lvl - 1].centers } else { &[] };
                        let sub_bsp: &[BspPoint] = if lvl > 0 { &cls_i.levels[lvl - 1].bsp } else { &[] };
                        let gate_cert = build_gate_certificate(&tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist, i, &ls.bsp, sub_centers, sub_bsp);
                        let (pass, nest_depth, rungs_len) = match &gate_cert {
                            Some(GateCertificate::Nest(cert)) => (cert.n_delta(), Some(effective_nest_depth(cert)), cert.rungs.len()),
                            Some(GateCertificate::Xzd(ev)) => {
                                n_xzd_routed += 1; // 小转大域触达（消歧死门用）
                                if ev.type2_confirmed { n_xzd_c2 += 1; }
                                if ev.sub_last_zs_type3 { n_xzd_c3 += 1; }
                                // C3 死门诊断探针（§5.4）：按 level 聚合 + lvl==1/lvl>=2 分裂断点。
                                if lvl < LMAX {
                                    xzd_routed_by_level[lvl] += 1;
                                    if ev.type2_confirmed { xzd_c2_by_level[lvl] += 1; }
                                    if ev.sub_last_zs_type3 { xzd_c3_by_level[lvl] += 1; }
                                }
                                if lvl == 1 {
                                    if ev.last_zs_exists { xzd_l1_last_zs_exists += 1; }
                                    if ev.same_side_l0_type3_any { xzd_l1_same_side_l0_type3_any += 1; }
                                    if ev.same_center_any { xzd_l1_same_center_any += 1; }
                                    if ev.same_side_causal_ok { xzd_l1_same_side_causal_ok += 1; }
                                    if ev.c3_new_center_exists { xzd_l1_c3_new_center_exists += 1; }
                                    if ev.c3_new_center_breakout_ok { xzd_l1_c3_new_center_breakout_ok += 1; }
                                    // 探针（只读旁路，见函数头注）：lvl==1 时 sub_units=L0 段账本（tower[0]），
                                    // sub_moves 同 build_gate_certificate 内部消费的 tower[lvl-1]。
                                    if overlap_probe_enabled {
                                        let l0_moves: &[LeveledMove] =
                                            tower_i.get(0).map(|m| m.as_slice()).unwrap_or(&[]);
                                        let sub_units = l0_units_from_tower(l0_moves);
                                        let probe = xzd_c3_overlap_window_probe(
                                            p.source_index, i, delta_side, &sub_units, l0_moves,
                                        );
                                        xzd_l1_overlap_rows.push((
                                            p.source_index,
                                            i,
                                            probe.overlapping_new_center_count,
                                            probe.overlapping_breakout_count,
                                            probe.first_overlapping_center,
                                        ));
                                    }
                                } else if lvl >= 2 {
                                    xzd_lge2_sub_bsp_type3_total += ev.sub_bsp_type3_count; // 死门重封：裁定A+#123 后可非 0，尾部锁基线
                                    // task #170 复审探针：lvl>=2 的 C3 新判据命中（gate_pass 在 level!=1 不读此二字段——
                                    // 计数只测「若并入硬门会怎样」的量化空间，零行为影响）。
                                    if ev.c3_new_center_exists { xzd_lge2_c3_new_center_exists += 1; }
                                    if ev.c3_new_center_breakout_ok { xzd_lge2_c3_new_center_breakout_ok += 1; }
                                }
                                (ev.gate_pass(), None, 0) // 小转大无区间套 depth
                            }
                            None => (false, None, 0),
                        };
                        if bsp_class == 0 && pass { n_zerobit_gate_pass += 1; }
                        if !pass {
                            continue;
                        }
                        if lvl < LMAX { sig_post[lvl] += 1; }
                        // C1：与生产 collect_signals 同序同字段（含 b2 完整 z——z_of_candidate
                        // 同一桥，保证 dx 手写门与生产收集 bit-exact，尾部 signals_dx vs signals_prod 逐条对拍含 z）。
                        // P0-1：与生产 collect_signals 同源触发分类（pass ⟹ gate_cert Some）。
                        let trigger_dx = nest_trigger(
                            gate_cert.as_ref().expect("pass ⟹ gate_cert Some"),
                            bsp_cand_type(&p.bits, delta_side),
                        );
                        // ★G3 ext（#138，与生产装配 bit-exact 同源）：Nest→depth=rungs.len()（pass ⟹
                        // effective_nest_depth==rungs.len()，前缀定理）+ origin=lvl+depth；Xzd→无下沉。
                        let ext_dx = match gate_cert.as_ref().expect("pass ⟹ gate_cert Some") {
                            GateCertificate::Nest(_) => ZExt {
                                cand_channel: Some(trigger_dx),
                                nest_depth: Some(rungs_len as u8),
                                origin_level: Some(lvl as u32 + rungs_len as u32),
                                risk_mode: None,
                                t_stage: None, // #149：与生产同源，统计层诚实 None
                                eta_bucket: None, // #175：与生产同源，统计层诚实 None
                            },
                            GateCertificate::Xzd(_) => ZExt {
                                cand_channel: Some(trigger_dx),
                                nest_depth: None,
                                origin_level: None,
                                risk_mode: None,
                                t_stage: None, // #149：与生产同源，统计层诚实 None
                                eta_bucket: None, // #175：与生产同源，统计层诚实 None
                            },
                        };
                        signals_dx.push(RawSignal {
                            entry_bar: i,
                            dir: c.dir,
                            pivot_bar: p.source_index,
                            level: lvl as u32,
                            sigma_higher,
                            bsp_class,
                            z: z_of_candidate(c, &tower_i, bars, &ext_dx), // ★force(c.force 透传 A6)+σ_higher+G3 ext（与生产同源）
                            trigger: trigger_dx,
                        });
                        match nest_depth {
                            Some(d) => {
                                // 区间套通过 ⟹ 所有 rung cand=true ⟹ depth=rungs.len()。
                                let depth = d.min(LMAX);
                                nest_depth_hist_pass[depth] += 1;
                                if lvl < LMAX { nest_depth_by_level_pass[lvl][depth] += 1; }
                            }
                            None => { n_xzd_pass += 1; } // 小转大通道通过（depth 直方图外计，设计 §2.3）
                        }
                        n_gate_pass_total += 1;
                        if lvl >= 1 {
                            let delta: i8 = if c.dir == VoiceSide::Long { 1 } else { -1 };
                            highlevel_hits.push((lvl, p.source_index, delta, bsp_class, rungs_len));
                        }
                    }
                }

                // ── Q4（task #145）盘整背驰承接 dx 镜像（bit-exact 复制生产 collect_signals 的
                // pan_div 消费块——尾部 signals_dx vs signals_prod 逐条对拍含 PanDiv 信号）。──
                for cert in ls.pan_div.iter() {
                    let side_disc = match cert.side {
                        Side::Long => 0u8,
                        Side::Short => 1u8,
                    };
                    if !seen_pan_dx.insert((lvl, cert.source_index, side_disc)) {
                        continue;
                    }
                    let sub_centers: &[Center] =
                        if lvl > 0 { &cls_i.levels[lvl - 1].centers } else { &[] };
                    let sub_bsp: &[BspPoint] =
                        if lvl > 0 { &cls_i.levels[lvl - 1].bsp } else { &[] };
                    if !pan_div_gate_pass(
                        &tower_i, lvl, cert, &macd_hist, i, &ls.bsp, sub_centers, sub_bsp,
                    ) {
                        continue;
                    }
                    if lvl < LMAX { sig_post_pan[lvl] += 1; } // Q4（#147）：门后记账与 push 同步
                    let (dir, delta_i8): (VoiceSide, i8) = match cert.side {
                        Side::Long => (VoiceSide::Long, 1),
                        Side::Short => (VoiceSide::Short, -1),
                    };
                    let sigma_higher = sigma_higher_at(&tower_i, bars, lvl);
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
                    signals_dx.push(RawSignal {
                        entry_bar: i,
                        dir,
                        pivot_bar: cert.source_index,
                        level: lvl as u32,
                        sigma_higher,
                        bsp_class: 0,
                        z,
                        trigger: NestTrigger::PanDivConsolidation,
                    });
                }
            }
        }

        // ── 报告 ──
        let mut rpt = String::new();
        let _ = writeln!(rpt, "# Task #7 acc-classification：level 塔中间级空洞 H1/H2/H3 判别");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "**认识论等级**：L2（真实 BTC 数据逐信号分级别计数，可产否定性结果）。");
        let _ = writeln!(rpt, "**窗口**：bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}。");
        let _ = writeln!(rpt, "**判别**：三路 instrument N^δ 门前后分级别计数（bit-exact 复制 decompose_capturable_spread 收集路径）。");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 分级别计数表（cls.levels.len() 最大={levels_seen_max}）");
        let _ = writeln!(rpt, "| level | tower段数 | bsp_pre(门前) | 第一类 | 第二类 | 第三类 | Γ非Flat | sig_post(门后) | 门滤除 |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|---|");
        for l in 0..LMAX {
            if tower_segs_max[l] == 0 && bsp_pre[l] == 0 && gamma_nonflat[l] == 0 && sig_post[l] == 0 {
                continue;
            }
            let filtered = gamma_nonflat[l].saturating_sub(sig_post[l]);
            let _ = writeln!(rpt, "| {l} | {} | {} | {} | {} | {} | {} | {} | {} |",
                tower_segs_max[l], bsp_pre[l], bsp_pre_first[l], bsp_pre_second[l], bsp_pre_third[l],
                gamma_nonflat[l], sig_post[l], filtered);
        }
        let _ = writeln!(rpt);

        // ── 三路判别判定 ──
        let _ = writeln!(rpt, "## H1/H2/H3 判定（中间级 = level 1..levels_seen_max-1）");
        let mid_hi = levels_seen_max.saturating_sub(1).min(LMAX);
        let mut any_mid_bsp = false;      // 中间级门前有 bsp?
        let mut any_mid_gate_filter = false; // 中间级门前有 Γ 但门后=0?
        for l in 1..mid_hi {
            if bsp_pre[l] > 0 { any_mid_bsp = true; }
            if gamma_nonflat[l] > 0 && sig_post[l] == 0 { any_mid_gate_filter = true; }
            let _ = writeln!(rpt, "- level{l}: bsp_pre={} (一/二/三={}/{}/{}) Γ非Flat={} sig_post={} → {}",
                bsp_pre[l], bsp_pre_first[l], bsp_pre_second[l], bsp_pre_third[l],
                gamma_nonflat[l], sig_post[l],
                if bsp_pre[l] == 0 { "门前空(H3候选:架构)" }
                else if sig_post[l] == 0 { "门滤空(H1候选)" }
                else { "有信号" });
        }
        let verdict = if !any_mid_bsp {
            "**H3（架构性，非运行时 bug）**：中间级 bsp_pre 全 0。注意：原「level≥1 只产第二类 B2/S2」\
             提取禁闭前提已被裁定A+#123（0a35f0167c）作废——级别≥1 现产一/二/三类；\
             此判仅意味着该窗中间级结构稀疏。**不是 N^δ 门滤空，不是全历史路径 bug**。"
        } else if any_mid_gate_filter {
            "**H1（N^δ 门滤空）**：中间级 bsp_pre>0 但门后 sig_post=0 ⟹ [J_{ℓ-1}⊆J_ℓ] 嵌套链\
             严格滤掉中间级。需 codex 异质确认（约束4）。"
        } else {
            "**中间级有信号**：与反演不符，可能 300K 窗与全历史窗差异（跨窗对比 H2 判别）。"
        };
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "### 判定：{verdict}");
        let _ = writeln!(rpt);

        // ── 结构 sanity（team-lead ③）：level≥1 通过门信号的 rung 链 ──
        let _ = writeln!(rpt, "## 结构 sanity：level≥1 通过门信号的 rung 链（team-lead ③）");
        if highlevel_hits.is_empty() {
            let _ = writeln!(rpt, "- 无 level≥1 通过 N^δ 门的信号（本窗）。");
        } else {
            let _ = writeln!(rpt, "| lvl | source_index | δ | bits(u8) | rung层数(tower[lvl+1..]含src) |");
            let _ = writeln!(rpt, "|---|---|---|---|---|");
            for (lvl, src, delta, bits, n_rungs) in &highlevel_hits {
                let _ = writeln!(rpt, "| {lvl} | {src} | {delta} | {bits:#04x} | {n_rungs} |");
            }
            let _ = writeln!(rpt, "\n**读解**：level_L 信号的 rung 层数 = 它上方 tower 各级含该 source_index 的段数。\
                这些是**上级语境**（N^δ 递归链），不是「该信号也算作 level_(L-1) 信号」——每个 bsp 只在\
                提取它的那个 classifier level 计一次（mod.rs 分级提取），rung 链是 N^δ 门的准入语境，非计数重复。");
        }

        // ── P1 FullNest 验收（task #22）：通过门信号的有效跨级深度分布（含 level0，bit-exact 同源）──
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## P1 FullNest 验收：通过门信号有效跨级深度分布（task #22，含 level0）");
        let _ = writeln!(rpt, "**有效跨级深度** = n_delta 递归实际穿越的连续 cand=true 跨级层数（build_nest_certificate 与生产门共用构造，bit-exact）。");
        let _ = writeln!(rpt, "- depth=0 ⟹ 纯 base-case confirm_side（**退化**：等价单 bit 检查，**无区间套跨级触达**，与旧单级门弱化）。");
        let _ = writeln!(rpt, "- depth≥1 ⟹ 真跨级 [J_{{ℓ-1}}⊆J_ℓ] 触达（N^δ ≥2 层证书 e→ℓ，Q5 验收要求）。");
        let _ = writeln!(rpt, "| 有效深度 | 通过门信号数 | 占比 |");
        let _ = writeln!(rpt, "|---|---|---|");
        for d in 0..=LMAX {
            if nest_depth_hist_pass[d] == 0 { continue; }
            let p = if n_gate_pass_total > 0 { 100.0 * nest_depth_hist_pass[d] as f64 / n_gate_pass_total as f64 } else { 0.0 };
            let _ = writeln!(rpt, "| {d} | {} | {p:.2}% |", nest_depth_hist_pass[d]);
        }
        let n_depth0 = nest_depth_hist_pass[0];
        let n_depth_ge1: usize = nest_depth_hist_pass[1..].iter().sum();
        let _ = writeln!(rpt, "\n**depth=0（退化 base-case）：{n_depth0}/{n_gate_pass_total}；depth≥1（真跨级触达）：{n_depth_ge1}/{n_gate_pass_total}**");
        let _ = writeln!(rpt, "\n### 逐执行级 × 深度矩阵");
        let _ = writeln!(rpt, "| exec_level | depth0 | depth1 | depth2 | depth3 | depth≥4 |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|");
        for l in 0..LMAX {
            let row = &nest_depth_by_level_pass[l];
            if row.iter().sum::<usize>() == 0 { continue; }
            let ge4: usize = row[4..].iter().sum();
            let _ = writeln!(rpt, "| {l} | {} | {} | {} | {} | {} |", row[0], row[1], row[2], row[3], ge4);
        }
        let p1_verdict = if n_gate_pass_total == 0 {
            "**无信号通过门（本窗）**——无法判定触达深度。"
        } else if n_depth_ge1 == 0 {
            "**退化坐实（FullNest 未触达）**：全部通过门信号 depth=0 ⟹ N^δ 门在本窗**完全退化为 base-case confirm_side**（单 bit 方向确认），\
             **区间套跨级 [J_{ℓ-1}⊆J_ℓ] 从未被消费**。代码是真递归但有效域内 rungs 恒空——「区间套已接入」是声明膨胀（090/spec-execution-gap）。"
        } else if n_depth0 == 0 {
            "**FullNest 全触达**：全部通过门信号 depth≥1 ⟹ 每个信号都消费了跨级区间套（≥2 层证书）。"
        } else {
            "**FullNest 部分触达（混合）**：部分信号跨级（depth≥1），部分退化（depth=0）。区间套在有效域内**真触达但非全覆盖**——\
             如实标注：depth=0 那部分等价 base-case，depth≥1 那部分是真 N^δ。"
        };
        let _ = writeln!(rpt, "\n### P1 判定：{p1_verdict}");
        let _ = writeln!(rpt);
        // ── 小转大通道（task #47：**C2+C3(breakout) xzd**——level==1 硬门改为 C2∧新中枢突破，
        // level>=2 维持 #41 的 C2-only）：门通过数 = 新增可交易信号（区间套之外，输入域 Some/None 不相交）──
        let _ = writeln!(rpt, "## 小转大通道命中（**C2+C3(breakout) xzd**——codex #44 终局裁定(c)：level==1 硬门=C2∧C3新中枢突破，level>=2 维持 C2-only）");
        let _ = writeln!(rpt, "**小转大通过**：{n_xzd_pass} 条（占通过门总数 {n_gate_pass_total} 的 {:.2}%）。level==1 的通过数已隐含 C3(新中枢+突破) 硬门；level>=2 仍为 C2-only（旧 C3 same_side_same_center 字段降为诊断，不参门）。",
            if n_gate_pass_total > 0 { 100.0 * n_xzd_pass as f64 / n_gate_pass_total as f64 } else { 0.0 });
        let n_nest_pass: usize = nest_depth_hist_pass.iter().sum();
        let _ = writeln!(rpt, "- 通道分离：区间套（descend=Some，depth 直方图 {n_nest_pass} 条）与小转大（descend=None，{n_xzd_pass} 条）输入域 Some/None 互斥（codex §6-4 构造同义反复，非经验重叠）。");
        let _ = writeln!(rpt, "- **小转大域触达/C2/旧C3 分项（诊断，旧 C3=same_side_same_center 不参门）**：路由到 Xzd={n_xzd_routed}，其中 C2 成立={n_xzd_c2}，旧 C3 成立={n_xzd_c3}，门通过={n_xzd_pass}。");
        let _ = writeln!(rpt, "- **StructBreak 收紧测量（task #62，codex 终局裁决A）**：零 bit（bsp_class==0）候选中通过门={n_zerobit_gate_pass} 条（新代码下恒 0——`bsp_cand_type` 六 bit 全零恒 StructBreak ⟹ `build_gate_certificate` 恒 None，不再经旧 else=>Type3 分派误入 Nest/Xzd 通道）。");
        let _ = writeln!(rpt);

        // ── C3 死门诊断探针（codex 终局裁定 §5.4，task #41）：按 level 聚合 + lvl==1/lvl>=2 分裂断点 ──
        let _ = writeln!(rpt, "## C3 死门诊断探针（task #41，裁定 §5.4 精确规格）");
        let _ = writeln!(rpt, "| level | Xzd routed | C2 成立 | C3 成立(same_side_same_center) | C3 命中率 |");
        let _ = writeln!(rpt, "|---|---|---|---|---|");
        for l in 0..LMAX {
            if xzd_routed_by_level[l] == 0 { continue; }
            let rate = 100.0 * xzd_c3_by_level[l] as f64 / xzd_routed_by_level[l] as f64;
            let _ = writeln!(rpt, "| {l} | {} | {} | {} | {rate:.2}% |",
                xzd_routed_by_level[l], xzd_c2_by_level[l], xzd_c3_by_level[l]);
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "### lvl==1 子集分裂断点（裁定 §5.4：last_zs_exists / same_side_l0_type3_any / same_center_any / same_side_causal_ok）");
        let n_l1 = xzd_routed_by_level.get(1).copied().unwrap_or(0);
        if n_l1 == 0 {
            let _ = writeln!(rpt, "- 本窗无 lvl==1 路由到 Xzd 的信号，无法裁断。");
        } else {
            let pct = |x: usize| 100.0 * x as f64 / n_l1 as f64;
            let _ = writeln!(rpt, "- lvl==1 routed={n_l1}");
            let _ = writeln!(rpt, "- last_zs_exists={} ({:.2}%)：s 跨度内存在次级中枢", xzd_l1_last_zs_exists, pct(xzd_l1_last_zs_exists));
            let _ = writeln!(rpt, "- same_side_l0_type3_any={} ({:.2}%)：存在同向 L0 Type3（不问 center）", xzd_l1_same_side_l0_type3_any, pct(xzd_l1_same_side_l0_type3_any));
            let _ = writeln!(rpt, "- same_center_any={} ({:.2}%)：存在任意侧 Type3 其 center==last_zs", xzd_l1_same_center_any, pct(xzd_l1_same_center_any));
            let _ = writeln!(rpt, "- same_side_causal_ok={} ({:.2}%)：同向 Type3 候选中存在 source_index<=confirm_index 者（否则=时间确认问题）", xzd_l1_same_side_causal_ok, pct(xzd_l1_same_side_causal_ok));
            let n_l1_c3 = xzd_c3_by_level.get(1).copied().unwrap_or(0);
            let l1_verdict = if n_l1_c3 == 0 {
                "**level==1 也是死门（本窗实测）**：same_side_same_center=0/lvl==1 routed ⟹ C2-only 全域退化处置在本窗成立，无需分级处置。".to_string()
            } else {
                format!(
                    "**level==1 命中率显著 >0（same_side_same_center={n_l1_c3}/{n_l1}={:.2}%）**：按裁定 §边界条件，\
                     处置应翻转为分级——lvl>=2 走 C2-only、lvl==1 保留 C2∧C3 硬门，而非本次全域退化。\
                     本探针只报数，是否翻转由后续工位/编排者裁定。",
                    pct(n_l1_c3)
                )
            };
            let _ = writeln!(rpt, "- {l1_verdict}");
        }
        let _ = writeln!(rpt);

        // ── XZD C2-only 口径复审探针（task #170）：lvl>=2 子集 C3 新判据命中数 ──
        // 分母 = lvl>=2 routed（与下方死门重封同源 xzd_routed_by_level[2..]）。报数不裁断：
        // 「C2-only 保留 vs C3 硬门全 level 启用」由 codex 裁定，本探针供其量化输入。
        {
            let n_lge2_probe: usize = xzd_routed_by_level[2..].iter().sum();
            let _ = writeln!(rpt, "### XZD C2-only 复审探针（task #170）：lvl>=2 C3 新判据命中");
            let _ = writeln!(
                rpt,
                "- lvl>=2 routed={n_lge2_probe}：c3_new_center_exists={xzd_lge2_c3_new_center_exists} / c3_new_center_breakout_ok={xzd_lge2_c3_new_center_breakout_ok}（不参门，codex 裁定输入）"
            );
            eprintln!("[c3-breakout-dx] lge2 routed={n_lge2_probe} new_center_exists={xzd_lge2_c3_new_center_exists} breakout_ok={xzd_lge2_c3_new_center_breakout_ok}");
        }
        let _ = writeln!(rpt);

        // ── C3 新判据命中率（task #47，codex #44(c) 终局裁定）：level==1 子集「新中枢+突破」命中率 ──
        let _ = writeln!(rpt, "### C3 新判据（新中枢+突破）level==1 命中率（task #47，codex #44 终局裁定(c)）");
        if n_l1 == 0 {
            let _ = writeln!(rpt, "- 本窗无 lvl==1 路由到 Xzd 的信号，无法裁断。");
        } else {
            let pct = |x: usize| 100.0 * x as f64 / n_l1 as f64;
            let _ = writeln!(rpt, "- c3_new_center_exists={} ({:.2}%)：source_index~confirm_index 间存在新确认次级中枢", xzd_l1_c3_new_center_exists, pct(xzd_l1_c3_new_center_exists));
            let _ = writeln!(rpt, "- c3_new_center_breakout_ok={} ({:.2}%)：新中枢被其后次级走势反向突破（level==1 硬门参门项）", xzd_l1_c3_new_center_breakout_ok, pct(xzd_l1_c3_new_center_breakout_ok));
            eprintln!("[c3-breakout-dx] lvl1 routed={n_l1} new_center_exists={xzd_l1_c3_new_center_exists} breakout_ok={xzd_l1_c3_new_center_breakout_ok}");

            // ── C3 L1 零命中根因判别探针报告落盘（codex #55 终局裁定(5)，task #56）──
            // 放在下方终局不变量 assert **之前**：assert 现为 `assert_eq!(breakout_ok, 0)`（#56 终局，
            // 见下），命中恒 0 时 assert **通过**不 panic；探针报告先落盘以在任何情形（含未来 rate>0
            // 触发 assert 失败）下都保留诊断证据（探针纯只读旁路，独立于终局不变量真封）。
            if overlap_probe_enabled {
                let overlap_new_center_signals = xzd_l1_overlap_rows.iter().filter(|r| r.2 >= 1).count();
                let overlap_hit_signals = xzd_l1_overlap_rows.iter().filter(|r| r.3 >= 1).count();
                let mut probe_rpt = String::new();
                let _ = writeln!(probe_rpt, "# C3 L1 零命中根因判别探针（codex #55 终局裁定(5)，task #56）");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "**认识论等级**：L2（真实 BTC 数据，逐 level==1 信号滑动重叠窗口扫描，可产否定性结果）。");
                let _ = writeln!(probe_rpt, "**窗口**：bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 结论");
                let _ = writeln!(probe_rpt, "- level==1 Xzd routed 样本数={}", xzd_l1_overlap_rows.len());
                let _ = writeln!(probe_rpt, "- 存在 ≥1 个滑动重叠窗口 post-source 新中枢的信号数={overlap_new_center_signals}");
                let _ = writeln!(probe_rpt, "- 存在 ≥1 个滑动重叠窗口新中枢被突破（overlapping_breakout_count>=1）的信号数={overlap_hit_signals}");
                let judgement = if overlap_hit_signals >= 1 {
                    "**判别=候选(1)成立**：overlapping_breakout_count>=1 出现 ⟹ 推翻当前生产中枢扫描策略\
                     （非重叠三段窗口是命中率为0的算法性根因），需转向实装可表达延伸/滑动确认的新中枢，用\
                     golden digest 重验全部受影响路径（codex #55 边界条件，回 codex 复审，大动作）。"
                } else {
                    "**判别=候选(2)坐实**：滑动重叠窗口下 overlapping_breakout_count 恒为 0（`overlapping_new_center_count`\
                     是否非零不改变本判别——边界条件二支均归候选(2)）⟹ 排除「非重叠窗口压掉新中枢」的算法限制假设，\
                     level==1 走势尚未确认背驰反转期间（source_index 之后）在真实数据窗口下几何上确无可反向突破的\
                     次级中枢——level==1 小转大通道在当前定义下实践关闭，不再为它改定义（codex #55 边界条件，终局）。"
                };
                let _ = writeln!(probe_rpt, "- {judgement}");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 定义依据");
                let _ = writeln!(probe_rpt, "codex #55 终局裁定(5) 精确规格（`.chanlun/review-results/codex-decide-20260702-201450-8032.md`）：\
                    `xzd_c3_overlap_window_probe` 对 `sub_units.windows(3)` 做滑动一格扫描（非 `detect_centers_with` 的非重叠\
                    三段消费），复用同一中枢构造函数 `center_from_segments`（方向交替+全三段核心非空）+ 同一突破规则\
                    `xzd_c3_new_center_breakout`（`Side::Long⟹hi>zg` / `Side::Short⟹lo<zd`）；只统计\
                    `start_index>=source_index && end_index<=confirm_index` 的 post-source 新中枢。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 边界条件（codex 原文，决定本裁定是否被推翻）");
                let _ = writeln!(probe_rpt, "- 若 `overlapping_breakout_count>=1`（本窗任一信号）：推翻本判别，转候选(1)——先修 `detect_centers_with`/`center.rs` 支持延伸/重叠中枢，全受影响路径 golden digest 重验。");
                let _ = writeln!(probe_rpt, "- 若 `overlapping_new_center_count==0`，或有滑动新中枢但 `overlapping_breakout_count==0`：候选(2)坐实，终局。");
                let _ = writeln!(probe_rpt, "- 若未来权威规格把「旧中枢 source 后延伸并突破」定义为 C3：重开候选(3)（本探针不判别候选(3)，候选(3)另需形式化裁定）。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 下游推论");
                let _ = writeln!(probe_rpt, "#13（W-VERIFY alpha 全量重测）的最终签收阻塞到本探针结果。候选(2)坐实 ⟹ Xzd 有效通道收窄为 level>=2 的 C2-only（30 条）+ Nest 862，#13 可基于该口径解锁；候选(1)成立 ⟹ #13 继续阻塞，需先完成 center/tower 延伸中枢实装+golden digest 重验全部受影响路径。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 谱系引用");
                let _ = writeln!(probe_rpt, "#44（`codex-decide-20260702-193853-5bbe.md`，C3 判据从「center 精确匹配」重设计为「新中枢+突破」）、\
                    #47（`c3-breakout-impl-20260702.md`，判据实装）、678（center 对象身份分裂——本次判定不适用于新判据，见 codex #55 原文 §678 摘要）、\
                    codex #55（`codex-decide-20260702-201450-8032.md`，本探针精确规格来源+判别边界条件来源）。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 影响声明");
                let _ = writeln!(probe_rpt, "新增 `XzdC3OverlapProbeDiag`/`xzd_c3_overlap_window_probe`/`l0_units_from_tower`（`rust/src/theta_v0/backtest/econ_positive.rs`），\
                    纯只读旁路，默认不调用，`ECON_C3_OVERLAP_PROBE=1` 门控。不改 `gate_pass()`、不改 `Classification.levels[*].centers`、\
                    不改 `tower`、不改任何生产 `collect_signals`/正常输出 digest——仅本诊断测试内新增探针调用+独立报告落盘\
                    （`.chanlun/review-results/c3-overlap-probe-20260702.md`），不影响既有 `acc-classification-level-hole-20260701.md` 报告内容。");
                let _ = writeln!(probe_rpt);
                let _ = writeln!(probe_rpt, "## 逐信号统计表（level==1 routed={}）", xzd_l1_overlap_rows.len());
                let _ = writeln!(probe_rpt, "| # | source_index | confirm_index | overlapping_new_center_count | overlapping_breakout_count | first_overlapping_center |");
                let _ = writeln!(probe_rpt, "|---|---|---|---|---|---|");
                for (idx, (src, confirm, new_cnt, brk_cnt, first)) in xzd_l1_overlap_rows.iter().enumerate() {
                    let first_str = first.map(|(s, e)| format!("({s},{e})")).unwrap_or_else(|| "—".to_string());
                    let _ = writeln!(probe_rpt, "| {} | {src} | {confirm} | {new_cnt} | {brk_cnt} | {first_str} |", idx + 1);
                }
                let probe_out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent().expect("rust/ 父目录 = 项目根")
                    .join(".chanlun/review-results/c3-overlap-probe-20260702.md");
                std::fs::write(&probe_out, &probe_rpt).unwrap_or_else(|e| panic!("写探针报告失败：{e}"));
                eprintln!(
                    "\n[c3-overlap-probe] 探针报告已落盘：{probe_out:?}（routed={} new_center_signals={overlap_new_center_signals} breakout_hit_signals={overlap_hit_signals}）",
                    xzd_l1_overlap_rows.len()
                );
            }

            // ★#148 死门重封（#137 先例：锁默认窗确定性基线）。旧终局不变量 `breakout_ok == 0`
            // （codex #55 裁定(5) + #56 候选(2)：「恒 0 = 市场几何事实」）的翻转条件由其自身预设：
            // 「center/tower 延伸中枢实装」⟹ 转候选(1)。#142（§5 延伸吸收）+ #148（第33课升级重切，
            // codex-decide-20260704-001933 裁定 A1/B-II/C1/D1）正是该条件的合法触发：升级重切改变
            // canonical 中枢链（≥9 段窗口拆为每 3 段子中枢）⟹ source~confirm 间可存在新确认次级
            // 中枢（exists 0→13）且可被反向突破（breakout_ok 0→3，2026-07-04 默认窗实测）。#56
            // 候选(2) 的前提结构（无升级语义的中枢链）已合法作废——这是语义变更，非逻辑漂移。
            // 重封形式 = 锁默认窗确定性基线（同下 lvl>=2 先例：基线是窗口函数，仅默认窗断言；
            // 窗口/数据/定义变 ⟹ 重测重锁，不放宽为范围断言）。
            // ★下游口径待裁（上浮 Lead，非本层自决）：Xzd level≥2 C2-only 信号口径因 C3 脱 0 须回
            // codex 复审（#56 边界条件转候选(1)：C3 并入硬门的可行性）——见 #148 结果包下游推论。
            let _ = writeln!(rpt, "- **判据健康度（#148 重封）**：延伸+升级实装后 level==1 C3 可命中（默认窗基线 exists=13/breakout_ok=3）；基线漂移 ⟹ 硬失败重推导；Xzd C2-only 口径复审已上浮。");
            if max_bars == MAX_BARS_DEFAULT {
                assert_eq!(
                    (xzd_l1_c3_new_center_exists, xzd_l1_c3_new_center_breakout_ok),
                    (13, 3),
                    "C3 新判据 level==1 计数漂移（exists={xzd_l1_c3_new_center_exists} breakout_ok=\
                     {xzd_l1_c3_new_center_breakout_ok}，默认窗基线 13/3，#148 升级重切后 2026-07-04 实测）\
                     ——固定数据+默认窗下漂移只能来自 center/tower/C3 逻辑变更，须重推导重封（推导链见上注）。"
                );
            }
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "### lvl>=2 死门重封（裁定A+#123：sub_bsp 可含 Type3，锁基线数值）");
        let n_lge2_routed: usize = xzd_routed_by_level[2..].iter().sum();
        let _ = writeln!(rpt, "- lvl>=2 routed={n_lge2_routed}，sub_bsp Type3 点总数={xzd_lge2_sub_bsp_type3_total}");
        eprintln!("[deadgate-reseal] lvl>=2 routed={n_lge2_routed} sub_bsp_type3_total={xzd_lge2_sub_bsp_type3_total}");
        // 死门重封（#137）：原前提「lvl>=2 sub_bsp 恒无 Type3（extract_second_for_level 只产 B2/S2）」
        // 已被 codex-t1 裁定A + #123（0a35f0167c，级别≥1 一/三类候选生成，三类净增 2364）合法作废。
        // 重封形式 = 锁默认窗确定性基线（BTC 冻结数据全量 4613599 bar，末 300K 窗 2025-11-04→2026-05-31，
        // 2026-07-03 实测）。基线是窗口函数 ⟹ 仅默认窗断言；窗口/数据变 ⟹ 重测重锁，不放宽为范围断言。
        if max_bars == MAX_BARS_DEFAULT {
            // Q7-#1 裁定C 重封（codex-q7-fallback-20260703，2026-07-03 实测）：fallback 单元非
            // 方向锚 ⟹ 上级一/三类收缩（一类 1057→614、type3 总数 3150→657），XZD 路由候选构成
            // 随中枢/信号结构变化 routed 75→160（codex 裁决预告「不保证单调」的实证）。
            // ★#148 重锁（2026-07-04 实测）：升级重切（第33课 ≥9 段拆子中枢，codex-decide-
            // 20260704-001933）改变 canonical 中枢链 ⟹ 全塔 BSP/路由级联变化，routed 160→217、
            // sub_bsp_type3_total 657→433——中枢变多（重切）且外缘变窄（子窗聚合）双向作用的净效应。
            assert_eq!(
                (n_lge2_routed, xzd_lge2_sub_bsp_type3_total), (217, 433),
                "lvl>=2 死门基线漂移（#148 升级重切后基线：routed=217/sub_bsp_type3_total=433）——\
                 上游 BSP 生产链（extract/hl13 级别-N 判定/Q7 锚门）或 Xzd 路由变化，需重测重锁并审计来源"
            );
            let _ = writeln!(rpt, "- **重封通过**：默认 300K 窗基线锁定 routed=217 / sub_bsp_type3_total=433（#148 升级重切后新真值，2026-07-04 测定）。");
        } else {
            let _ = writeln!(rpt, "- 非默认窗（max_bars={max_bars}），基线断言跳过（基线仅对默认 300K 窗定义）。");
        }
        let _ = writeln!(rpt);

        // ── 配对后 decomps level 分布（关键：sig_post 是门后配对前，decomps 是配对后）──
        // 分水岭：若中间级 sig_post>0 但 decomps 该级=0 ⟹ 配对阶段丢失（非门滤空 H1，是右删失/跨级混合配对）。
        // ── C1（algo-opt-plan-20260702 泳道 C）：配对复用 signals_dx，省二次 O(bar²) 全量重收集。──
        // 原 decompose_capturable_spread(&ds,&config) = collect_signals(O bar²) + pair_signals(O 信号)；
        // 复用后 pair_signals 直接吃 dx 手写循环自建的 signals_dx（与生产 collect_signals 同序同门），
        // 全窗只跑一次 classify_at 收集 —— 本项 dx harness ~2x 提速来源。
        //
        // 真封退化诚实声明（no-patch）：agg_prod 改由 signals_dx 派生后，真封① sig_post_sum>=n_signals
        // 从「dx 循环 vs 独立 decompose」交叉验证退化为 dx 收集内自证。恢复交叉验证需再跑一次
        // collect_signals（O bar²）—— 与提速目标互斥。替代对拍（plan verdict 首选）：默认窗
        // （≤300K，correctness tier）跑全量 collect_signals 逐元组对拍 signals_dx，坐实 dx 手写门
        // （build_nest_certificate.n_delta）与生产门（build_multilevel_nest_cert）bit-exact 同收集；
        // 全历史基准窗（ECON_L2_MAX_BARS 放大，perf tier）跳过对拍换取 2x，收集正确性由默认窗对拍背书。
        let fee_rate =
            (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
        if max_bars <= MAX_BARS_DEFAULT {
            let signals_prod = collect_signals(&ds, &config);
            assert_eq!(
                signals_dx, signals_prod,
                "dx 手写收集循环 ≠ 生产 collect_signals：门判定/字段漂移（n_delta vs build_multilevel_nest_cert，或 sigma_higher/bsp_class）"
            );
        }

        // ★force_state 生产热路由 fill-rate 断言（beta-route #115 step7；perm_test.rs:207-210 指明属 L2
        // 回测报告消费点，非 lib 单元层）。防「生产全 None 静默」：一类信号（i_class 含 buy1|sell1，
        // bit0|bit3）⟹ 必有 A/C 段配对（macd_c_lt_a 判据前提）⟹ z.force_state 须 Some（A4 支配序已填）。
        // 若路由断裂（增量 dif 空 / 透传丢失），一类信号 force_state 全 None ⟹ 此断言 fire。
        // δ-共线核对（memory「方向性维进桶键=检验自毁」）：force_state 由 A/C 段**绝对量**算，δ-free
        // （perm_test.rs:220 置于 base_of δ-free 键，置换 δ 时恒定）⟹ 非方向性 A4 支配序，不引入 δ-共线。
        let type1_sigs = signals_dx.iter().filter(|s| s.z.i_class & 0b001_001 != 0).count();
        let type1_forced = signals_dx
            .iter()
            .filter(|s| s.z.i_class & 0b001_001 != 0 && s.z.force_state.is_some())
            .count();
        eprintln!(
            "[force_state 生产路由] 信号总数={} 一类(buy1|sell1)={} force_state=Some={} 填充率={:.0}%",
            signals_dx.len(), type1_sigs, type1_forced,
            if type1_sigs > 0 { 100.0 * type1_forced as f64 / type1_sigs as f64 } else { 0.0 }
        );
        if type1_sigs > 0 {
            assert_eq!(
                type1_forced, type1_sigs,
                "force_state 生产热路由：每个一类信号（A/C 段可配对）z.force_state 须 Some（防增量 dif 空/透传丢失致全 None 静默）"
            );
        }
        let (decomps_prod, agg_prod) = pair_signals(&signals_dx, bars, tick, fee_rate);
        let mut decomp_by_level = [0usize; LMAX];
        for d in &decomps_prod {
            if (d.level as usize) < LMAX { decomp_by_level[d.level as usize] += 1; }
        }
        let _ = writeln!(rpt, "## 配对后 decomps level 分布（sig_post=门后配对前 vs decomps=配对后）");
        let _ = writeln!(rpt, "| level | sig_post(门后Γ) | pan(承接) | decomps(配对后) | 配对丢失 |");
        let _ = writeln!(rpt, "|---|---|---|---|---|");
        let mut any_pairing_loss_mid = false;
        for l in 0..LMAX {
            let gate_total = sig_post[l] + sig_post_pan[l]; // 门后全通道（Γ 二通道 + PanDiv 承接）
            if gate_total == 0 && decomp_by_level[l] == 0 { continue; }
            let lost = gate_total.saturating_sub(decomp_by_level[l]);
            if l >= 1 && decomp_by_level[l] == 0 && gate_total > 0 { any_pairing_loss_mid = true; }
            let _ = writeln!(rpt, "| {l} | {} | {} | {} | {} |",
                sig_post[l], sig_post_pan[l], decomp_by_level[l], lost);
        }
        let _ = writeln!(rpt, "\n- n_unpaired（无配对出场反转信号，右删失剔除）={}", agg_prod.n_unpaired);
        let _ = writeln!(rpt, "- **配对机制**：next_opp 表跨级别混合（所有 level 信号按 entry_bar 排序）——\
            中间级信号找「下一个反向信号」时不分级别，几乎总配 level0（信号 {}/{} 是 level0）。\
            decomp.level=入场信号 level（配对出场 level 不影响）。", decomp_by_level[0], decomps_prod.len());
        if any_pairing_loss_mid {
            let _ = writeln!(rpt, "- **⚠ 配对丢失坐实**：中间级 sig_post>0 但 decomps=0 ⟹ 通过 N^δ 门的中间级信号\
                在退出配对阶段被 n_unpaired（右删失）剔除。**H1 门滤空判定被修正**——中间级空洞不仅是门滤，\
                还有配对丢失叠加（门后配对前 sig_post 已含中间级，配对后消失）。");
        }
        let _ = writeln!(rpt);

        // 落盘
        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/acc-classification-level-hole-20260701.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告失败：{e}"));
        eprintln!("\n报告已落盘：{out:?}");

        // 真封①：分级别门后计数之和 >= n_signals（门后信号含未配对出场者）。门后全通道 =
        // sig_post（Γ 二通道）+ sig_post_pan（PanDiv 承接，#145 新入口，#147 补记账——此前漏计
        // 致 sig_post_sum < n_signals 假红）。C1 后 agg_prod 源自 signals_dx（同收集），此断言退化
        // 为收集内自证（n_signals = 其配对子集）；跨实现交叉验证由上方 max_bars≤300K 的
        // collect_signals 逐元组对拍承接（perf 窗跳过对拍，收集正确性由 correctness 窗背书）。
        let sig_post_sum: usize = sig_post.iter().sum();
        let pan_pass_sum: usize = sig_post_pan.iter().sum();
        // 收集闭合（#147 防再漂移）：每条 push 恰有一次门后计数 ⟹ 新增信号通道漏记账即红。
        assert_eq!(sig_post_sum + pan_pass_sum, signals_dx.len(),
            "门后计数和({sig_post_sum}+{pan_pass_sum}) 应 = signals_dx.len({})：信号通道记账漏计",
            signals_dx.len());
        assert!(sig_post_sum + pan_pass_sum >= agg_prod.n_signals,
            "sig_post_sum({sig_post_sum})+pan({pan_pass_sum}) 应 >= n_signals({})：门后信号数含未配对出场者", agg_prod.n_signals);
        // 真封②：配对后 decomps 逐级和 = n_signals（聚合完整性）。
        let decomp_sum: usize = decomp_by_level.iter().sum();
        assert_eq!(decomp_sum, agg_prod.n_signals,
            "decomp 逐级和({decomp_sum}) 应 = n_signals({})", agg_prod.n_signals);
        eprintln!("真封：sig_post_sum={sig_post_sum}+pan={pan_pass_sum} >= n_signals={} = decomp_sum={decomp_sum}",
            agg_prod.n_signals);
        // 真封③（P1 FullNest + 小转大二通道）：区间套深度直方图和 + 小转大通过 = 通过门总数 = sig_post_sum
        // （build_gate_certificate 二通道 bit-exact 同源；小转大通道无区间套 depth，depth 直方图外计 n_xzd_pass）。
        let depth_hist_sum: usize = nest_depth_hist_pass.iter().sum();
        assert_eq!(depth_hist_sum + n_xzd_pass, n_gate_pass_total,
            "区间套深度直方图和({depth_hist_sum})+小转大通过({n_xzd_pass}) 应 = 通过门总数({n_gate_pass_total})");
        assert_eq!(n_gate_pass_total, sig_post_sum,
            "通过门总数({n_gate_pass_total}) 应 = sig_post_sum({sig_post_sum})（build_gate_certificate 二通道 bit-exact == 生产 collect_signals）");
        eprintln!("真封③（P1+小转大）：depth_hist_sum={depth_hist_sum}+n_xzd_pass={n_xzd_pass} = n_gate_pass_total={n_gate_pass_total} = sig_post_sum={sig_post_sum}");
        eprintln!("[structbreak-tighten] 零 bit（bsp_class==0）候选通过门={n_zerobit_gate_pass}（task #62 收紧测量）");
    }

    /// 阶段0 对拍（bottomup-nest task #101，PDF §二/§四/§十二问题①）：现行生产 descend（source_index
    /// **点包含**）vs PDF bottom-up（`J_{k-1}⊆I(c)` **子区间包含** + Sel_Θ 作用于包含集）在真实 BTC
    /// 逐信号对拍——Γ 成员（n_delta）差异计数 + 有效深度分布差异 + rung 结构差异。
    ///
    /// **假设**（PDF §三.1）：recursive_tower Compose 是良式分解 refinement（tower[k] 边界 ⊆
    /// tower[k-1] 边界），故含段唯一、Sel 平凡、点包含 ≡ 区间包含 ⟹ 二者逐信号 bit-exact，差异=0。
    /// 差异=0 ⟹ 本测试即**等价固化**（NO-SHIP，生产 descend 无需改）；差异>0 ⟹ 塔非严格 refinement，
    /// 须把生产改成 bottom-up（PDF §五裁决 b），届时本测试的 assert 会红，暴露分歧信号。
    ///
    /// **认识论 L2**：真实 BTC 单标的全历史逐信号对拍，可产否定性结果（差异>0 = 现口径非最严格）。
    /// `#[ignore]`：需 BTC 全量 + O(n²) 重分类，`--release`。
    /// 命令：`ECON_L2_MAX_BARS=<N> cargo test --release acc_bottomup_nest_parity_probe -- --ignored --nocapture`
    /// （默认 300K；全历史用 ECON_L2_MAX_BARS=5000000）。
    #[test]
    #[ignore]
    fn acc_bottomup_nest_parity_probe() {
        use super::super::data;
        use super::super::super::classifier::divergence::compute_macd;
        use super::super::super::strategy::interp::assemble_gamma_with_tower;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::Side;
        use super::super::incremental::IncrementalClassifier;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS_DEFAULT: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS_DEFAULT);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let bars = &ds.bars;
        let n = bars.len();
        let tick = config.tick.tick_size;
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        eprintln!("[bottomup-parity] bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");

        let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
        let macd_hist = compute_macd(&closes, &config.macd).hist;

        const LMAX: usize = 8;
        let mut n_cert_attempt = 0usize;   // 到达 cert 构造点的候选总数（gamma 非 Flat）
        let mut n_prod_some = 0usize;      // 生产 cert = Some
        let mut n_bu_some = 0usize;        // bottom-up cert = Some
        let mut n_some_mismatch = 0usize;  // Some/None 不一致
        let mut n_gamma_diff = 0usize;     // n_delta（Γ 成员）不一致
        let mut n_depth_diff = 0usize;     // effective_nest_depth 不一致（两者均 Some）
        let mut n_rungs_len_diff = 0usize; // rungs.len() 不一致
        let mut n_interval_diff = 0usize;  // 任一 rung 区间/base 不一致（结构差，即便 n_delta 同）
        let mut gamma_prod = 0usize;       // 生产 Γ 规模（n_delta=true）
        let mut gamma_bu = 0usize;         // bottom-up Γ 规模
        let mut depth_prod = [0usize; LMAX + 1];
        let mut depth_bu = [0usize; LMAX + 1];
        // 分歧样本前 20 条（诊断用）：(bar_i, lvl, source_index, δ, prod_ndelta, bu_ndelta, prod_depth, bu_depth)
        let mut diff_samples: Vec<(usize, usize, usize, i8, bool, bool, usize, usize)> = Vec::new();

        let mut classifier_incr = IncrementalClassifier::new(bars, &config);
        let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
        let mut prev_bsp: Vec<Rc<Vec<BspPoint>>> = Vec::new();

        for i in 0..n {
            let bar = &bars[i];
            if bar.untradable || bar.close <= 0 {
                continue;
            }
            let (cls_i, tower_i) = classifier_incr.classify_at(i);
            for (lvl, ls) in cls_i.levels.iter().enumerate() {
                if prev_bsp.get(lvl).map_or(false, |prev| Rc::ptr_eq(prev, &ls.bsp)) {
                    continue;
                }
                if lvl < prev_bsp.len() {
                    prev_bsp[lvl] = Rc::clone(&ls.bsp);
                } else {
                    prev_bsp.push(Rc::clone(&ls.bsp));
                }
                for p in ls.bsp.iter() {
                    let bsp_class = bsp_disc(&p.bits);
                    if !seen.insert((lvl, p.source_index, bsp_class)) {
                        continue;
                    }
                    let single = super::super::super::classifier::Classification {
                        levels: cls_i.levels.iter().enumerate()
                            .map(|(l2, _)| super::super::super::classifier::LevelState {
                                moves: Vec::new(), centers: Rc::new(Vec::new()),
                                bsp: Rc::new(if l2 == lvl { vec![p.clone()] } else { Vec::new() }),
                                pan_div: Rc::new(Vec::new()), // Q4：dx 与生产 single 同形（无盘整背驰载荷）
                            })
                            .collect(),
                    };
                    for c in &assemble_gamma_with_tower(&single, &tower_i) {
                        let delta_side = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => continue,
                        };
                        n_cert_attempt += 1;
                        // 两口径 cert 构造（同一 tower/lvl/source_index/δ/bits/hist）——唯一变量是 rung 锚定口径。
                        let prod = build_nest_certificate(&tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist);
                        let bu = build_nest_certificate_bottomup(&tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist);
                        let (prod_nd, prod_depth) = match &prod {
                            Some(cert) => { n_prod_some += 1; (cert.n_delta(), effective_nest_depth(cert)) }
                            None => (false, 0),
                        };
                        let (bu_nd, bu_depth) = match &bu {
                            Some(cert) => { n_bu_some += 1; (cert.n_delta(), effective_nest_depth(cert)) }
                            None => (false, 0),
                        };
                        if prod.is_some() != bu.is_some() { n_some_mismatch += 1; }
                        if prod_nd { gamma_prod += 1; }
                        if bu_nd { gamma_bu += 1; }
                        if prod.is_some() { depth_prod[prod_depth.min(LMAX)] += 1; }
                        if bu.is_some() { depth_bu[bu_depth.min(LMAX)] += 1; }
                        if prod_nd != bu_nd { n_gamma_diff += 1; }
                        if prod.is_some() && bu.is_some() && prod_depth != bu_depth { n_depth_diff += 1; }
                        // 结构差：rungs.len 或任一 interval（含 base）不同。
                        if let (Some(pc), Some(bc)) = (&prod, &bu) {
                            if pc.rungs.len() != bc.rungs.len() { n_rungs_len_diff += 1; }
                            let struct_diff = pc.base_interval != bc.base_interval
                                || pc.rungs.len() != bc.rungs.len()
                                || pc.rungs.iter().zip(bc.rungs.iter()).any(|(a, b)| a.interval != b.interval || a.cand != b.cand);
                            if struct_diff { n_interval_diff += 1; }
                        }
                        if prod_nd != bu_nd || (prod.is_some() && bu.is_some() && prod_depth != bu_depth) {
                            if diff_samples.len() < 20 {
                                let dl: i8 = if delta_side == Side::Long { 1 } else { -1 };
                                diff_samples.push((i, lvl, p.source_index, dl, prod_nd, bu_nd, prod_depth, bu_depth));
                            }
                        }
                    }
                }
            }
        }

        eprintln!("\n════════ bottomup-nest 阶段0 对拍结果（BTC {n} bar，{win_start}→{win_end}）════════");
        eprintln!("到达 cert 构造点候选数 n_cert_attempt = {n_cert_attempt}");
        eprintln!("cert=Some：生产 {n_prod_some} / bottom-up {n_bu_some}（Some/None 不一致 n_some_mismatch={n_some_mismatch}）");
        eprintln!("Γ 规模（n_delta=true）：生产 gamma_prod={gamma_prod} / bottom-up gamma_bu={gamma_bu}");
        eprintln!("── 差异计数（全 0 ⟹ 两口径 bit-exact 等价）──");
        eprintln!("Γ 成员差异 n_gamma_diff      = {n_gamma_diff}");
        eprintln!("有效深度差异 n_depth_diff    = {n_depth_diff}");
        eprintln!("rungs.len 差异 n_rungs_len_diff = {n_rungs_len_diff}");
        eprintln!("结构差异 n_interval_diff     = {n_interval_diff}");
        eprint!("有效深度分布（生产）：");
        for (d, c) in depth_prod.iter().enumerate() { if *c > 0 { eprint!("d{d}={c} "); } }
        eprintln!();
        eprint!("有效深度分布（bottom-up）：");
        for (d, c) in depth_bu.iter().enumerate() { if *c > 0 { eprint!("d{d}={c} "); } }
        eprintln!();
        if !diff_samples.is_empty() {
            eprintln!("── 分歧样本（前 {}）(bar,lvl,src,δ,prod_nd,bu_nd,prod_depth,bu_depth) ──", diff_samples.len());
            for s in &diff_samples { eprintln!("  {s:?}"); }
        }
        eprintln!("════════════════════════════════════════════════════════════════════\n");

        // 等价固化（PDF §三.1 良式分解定位天然唯一）：差异全 0 ⟹ 生产点包含 descend ≡ PDF bottom-up
        // 区间包含 descend。此 assert 是等价的机器守卫——将来塔构造改动若破坏 refinement，本测试转红，
        // 逼出 bottom-up 实装（不静默把非严格 refinement 当等价，formalization-validity-domain L2）。
        assert_eq!(n_some_mismatch, 0, "Some/None 分歧：生产 descend 与 bottom-up 定位存在性不一致");
        assert_eq!(n_gamma_diff, 0, "Γ 成员分歧：n_delta 不一致——现口径非 PDF bottom-up 等价，须实装 bottom-up");
        assert_eq!(n_depth_diff, 0, "有效深度分歧：区间套跨级层数不一致");
        assert_eq!(n_interval_diff, 0, "rung 结构分歧：定位区间/cand 不一致（点包含选段 ≠ 区间包含 Sel 选段）");
    }

    /// P0-1 验收（p01-nest-trigger task #108，codex-f2 #1/#2）：per-`NestTrigger` 桶的候选数 + μ̂
    /// 质量对照——**证明「高级别背驰段前置门」只作用于 Type1，Xzd 不受一票否决**。
    ///
    /// 「过滤前后」框架（codex #1）：若把 f1 提议的单一 bool「须有高级别背驰段」门套到**全通道**，只有
    /// `Type1TrendDivergence` 存活，`Type23SublevelType1`+`XiaoZhuanDa` 被一票否决。本探针量化被否决桶的
    /// 候选数与 μ̂——若这些桶 μ̂ 非负/可观，一票否决即误杀有质量信号（codex 裁决「Xzd 不受否决」的 L2 证据）。
    /// **不绑 depth≥2**（codex #2：加门只减候选不制造深 rungs）。
    ///
    /// **认识论 L2**：真实 BTC 单标的全历史，per-trigger μ̂ 含选择偏差（全窗，仅质量对照非 OOS alpha）。
    /// `#[ignore]`：需 BTC 全量 + O(n²) 重分类。命令：
    /// `ECON_L2_MAX_BARS=<N> cargo test --release acc_nest_trigger_quality_probe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn acc_nest_trigger_quality_probe() {
        use super::super::data;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS_DEFAULT: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS_DEFAULT);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();

        let (decomps, _agg) = decompose_capturable_spread(&ds, &config);
        let total = decomps.len();

        // per-trigger 桶：(count, Σactual_pnl)。
        let mut bucket: std::collections::BTreeMap<u8, (usize, f64)> = std::collections::BTreeMap::new();
        let key = |t: NestTrigger| match t {
            NestTrigger::Type1TrendDivergence => 0u8,
            NestTrigger::Type23SublevelType1 => 1u8,
            NestTrigger::XiaoZhuanDa => 2u8,
            NestTrigger::PanDivConsolidation => 3u8, // Q4 #145：盘整背驰承接桶（独立归因，不并入他桶）
        };
        for d in &decomps {
            let e = bucket.entry(key(d.trigger)).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += d.actual_pnl;
        }
        // 交叉校验（防 trigger 标注 bug）：直接数 decomp.bsp_class 的一类位（bit0=buy1 / bit3=sell1）。
        let first_class_n = decomps.iter().filter(|d| d.bsp_class & (1 << 0) != 0 || d.bsp_class & (1 << 3) != 0).count();
        let get = |k: u8| bucket.get(&k).copied().unwrap_or((0, 0.0));
        let (t1_n, t1_pnl) = get(0);
        let (t23_n, t23_pnl) = get(1);
        let (xzd_n, xzd_pnl) = get(2);
        let (pan_n, pan_pnl) = get(3); // Q4 #145：盘整背驰承接桶
        let mu = |n: usize, s: f64| if n > 0 { s / n as f64 } else { 0.0 };
        let vetoed_n = t23_n + xzd_n;
        let vetoed_pnl = t23_pnl + xzd_pnl;

        eprintln!("\n════════ P0-1 NestTrigger 质量对照（BTC {total} 配对信号，{win_start}→{win_end}）════════");
        eprintln!("| trigger | n | Σactual_pnl | μ̂ |");
        eprintln!("| Type1TrendDivergence | {t1_n} | {t1_pnl:.4e} | {:.4e} |", mu(t1_n, t1_pnl));
        eprintln!("| Type23SublevelType1  | {t23_n} | {t23_pnl:.4e} | {:.4e} |", mu(t23_n, t23_pnl));
        eprintln!("| XiaoZhuanDa          | {xzd_n} | {xzd_pnl:.4e} | {:.4e} |", mu(xzd_n, xzd_pnl));
        eprintln!("| PanDivConsolidation  | {pan_n} | {pan_pnl:.4e} | {:.4e} |", mu(pan_n, pan_pnl));
        eprintln!("── 过滤前后（codex #1：单一 bool 背驰门若套全通道）──");
        eprintln!("交叉校验：decomp.bsp_class 含一类位(buy1/sell1)的条数 = {first_class_n}（应≈Type1TrendDivergence 桶 n={t1_n}）");
        eprintln!("过滤前候选总数 = {total}");
        eprintln!("过滤后（仅 Type1 存活）= {t1_n}；被一票否决 = {vetoed_n}（Type23 {t23_n} + Xzd {xzd_n}）");
        eprintln!("被否决桶 μ̂ = {:.4e}（Σpnl={vetoed_pnl:.4e}）——非负/可观 ⟹ 一票否决误杀有质量信号（codex 裁决 Xzd 不受否决）", mu(vetoed_n, vetoed_pnl));
        eprintln!("════════════════════════════════════════════════════════════════════\n");

        // 自检（partition 不变量）：四桶计数和 = 配对信号总数（trigger 标注无遗漏无重复；Q4 #145 增 PanDiv 桶）。
        assert_eq!(t1_n + t23_n + xzd_n + pan_n, total, "NestTrigger 四桶未完全覆盖配对信号——trigger 标注有洞");
    }

    /// H2 样本级验证（task #8）：level1-4 第二类信号的 N^δ 门拒绝阶段分解 + 互斥链实证。
    ///
    /// **来源**：codex H2 裁决（`.chanlun/review-results/codex-h1-ndelta-gate-20260702.md`）的
    /// L0 推导需 L2 样本级收口。codex 断言：`build_nest_certificate` 在 rung k=lvl+1 把 `Cand^δ`
    /// 操作化为 `div_cand` 的 **Extreme**（Long: `s.lo<s_prev.lo`），与第二类分类前提
    /// `retrace_no_break`（Long: `m2.lo>=m1.lo`）在 `s=m2 / s_prev=m1` 时结构性互斥。
    /// **边界条件缺口**（codex 自留）：若 B1/B2 间有多段同向子腿，`div_cand` 条件2的 `rfind` 命中
    /// 比 m1 更近的 q≠m1，`m2.lo>=m1.lo`（分类前提）与 `m2.lo<q.lo`（Extreme）可同真，互斥链不
    /// 必然成立。codex 给出**等价可测判据**：「s_prev 的低点仍高于/接近 m2 使 Extreme 必假」
    /// ⟹ 直接测 rung k=lvl+1 的 Extreme 真假即等价于「s_prev 使互斥成立」。
    ///
    /// **级别对齐（本测试的关键前提，非假设）**：level-lvl 第二类信号由 `extract_second_for_level`
    /// 从 level-(lvl+1) 的 `parent`（`RMove::Compose`）产出——`m1`/`m2` 是 `descend(parent)` 的
    /// **level-lvl** 次级别走势，`source_index = m2.end_index`。而 `build_nest_certificate` 的 rung
    /// k=lvl+1 的 `knode` **正是**该 `parent`（`tower[lvl+1]` 中含 source_index 的段），
    /// `knode.sub_moves = [m1, m2, …]`，`target_idx = i2`（m2 位置），`s = m2`。故 codex 的
    /// 「s_prev vs m1」是同级别对象比较，无级别偏移。
    ///
    /// **产出**（三 deliverable）：
    /// - (a) rung k=lvl+1 到达 cond3 的信号中 **Extreme必假占比**（codex 等价 s_prev 使互斥成立）；
    /// - (b) **Extreme可满足（gap）**样本（`s_prev=q≠m1` 且 m2 创新极值）的门通过/拒绝分布；
    /// - (c) 结论：level1-4 100%归零是否**完全**由 cond3 互斥链解释（还是 cond1 方向/cond2 无前段/
    ///   base 等其他机制叠加）。
    ///
    /// **bit-exact 铁律**：门判定用 `build_nest_certificate` + `n_delta`（与生产 `build_multilevel_nest_cert`
    /// 共用构造），rung 分解的 `div_cand`/`rmove_dir` 与生产同源；cond4(Weak) 用「cond1∧2∧3 通过但
    /// div_cand=false ⟹ cond4 失败」推断（不触碰私有 `segment_macd_area`）。macd_hist 与生产 econ 路径
    /// 同口径（全 bar 域 close）。
    ///
    /// **认识论 L2**：真实 BTC 全历史逐信号分解，可产否定性结果（互斥链不完全解释 ⟹ codex 裁决部分转向）。
    /// `#[ignore]`：需 BTC 全量 + O(n²) 重分类，`--release`。
    /// 命令：`ECON_L2_MAX_BARS=5000000 cargo test --release h2_sample_exclusion_dx -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn h2_sample_exclusion_dx() {
        use super::super::data;
        use super::super::super::classifier::divergence::compute_macd;
        use super::super::super::classifier::cand_predicate::{div_cand, DivCandInput, rmove_dir};
        use super::super::super::strategy::interp::assemble_gamma_with_tower;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{Side, Direction};
        use super::super::incremental::IncrementalClassifier;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        const MAX_BARS_DEFAULT: usize = 300_000;
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(MAX_BARS_DEFAULT);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let bars = &ds.bars;
        let n = bars.len();
        let tick = config.tick.tick_size;
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        eprintln!("[h2-exclusion-dx] bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");

        let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
        let macd_hist = compute_macd(&closes, &config.macd).hist;

        // ── 目标域：level ∈ 1..=4（acc 报告的 1473 中间级第二类信号）。 ──
        const LMIN: usize = 1;
        const LMAX: usize = 4;
        let in_scope = |lvl: usize| (LMIN..=LMAX).contains(&lvl);

        // ── 分解计数器（rung k=lvl+1 div_cand 拒绝阶段，互斥优先序 = div_cand 检查序）。 ──
        let mut n_total = 0usize;                 // 进入 Γ 的 level1-4 信号（每 (信号,dir) 一次，同 acc 口径）
        let mut n_by_level = [0usize; LMAX + 1];  // 各级 n_total
        // 拒绝阶段（当 gate=false 时归入首个失败检查）+ gate_pass。互斥穷举。
        let mut st_base_none = 0usize;      // build_nest_certificate=None（tower[lvl] 无 end==src 候选段）
        let mut st_no_upper = 0usize;       // rung lvl+1 无 knode（tower[lvl+1] 无含 src 段 ⟹ rungs 空，退化 base-case）
        let mut st_no_target = 0usize;      // knode.sub_moves 无 end==src（target_idx 缺失）
        let mut st_cond1_dir = 0usize;      // dir(s=m2) ≠ −δ（m2 方向不是背驰段要求方向）
        let mut st_cond2_noprev = 0usize;   // 无前序同向段（s_prev 不存在）
        let mut st_cond3_extreme = 0usize;  // ★Extreme 假（m2 未创新极值）= codex 互斥链
        let mut st_cond4_weak = 0usize;     // cond1∧2∧3 通过但 div_cand=false ⟹ cond4(Weak) 失败
        let mut st_reject_elsewhere = 0usize; // rung lvl+1 div_cand=true 但 gate=false（更高 rung / is_sub / base）
        let mut st_gate_pass = 0usize;      // n_delta=true（通过门）

        // ── 互斥链核心统计（cond3 到达域）。 ──
        let mut cond3_reached = 0usize;     // rung lvl+1 到达 cond3（cond1∧cond2 通过）
        let mut extreme_true_pass = 0usize; // Extreme 真 ∧ gate 通过
        let mut extreme_true_reject = 0usize; // Extreme 真 ∧ gate 拒绝（cond4 或 elsewhere）= codex 缺口坐实
        // cond3 到达但 cert=None（落 base_none 短路，绕过 extreme 三桶）。673 号段2 landing 后新增：
        // Type2/3 信号下沉次级别无 Type1 锚点（小转大）⟹ build_nest_certificate 段2 门 return None（line 599），
        // 而 cond3_reach（tower[lvl+1] cond1∧cond2）独立成立 ⟹ 该信号在 cond3 到达域内但不入 extreme 分桶。
        let mut extreme_cert_none = 0usize;
        // cond3 到达 ∧ gate 通过 ∧ r_extreme 假：生产门 n_delta 经 base-case/其他 rung 通过，
        // 但 test 侧 lvl+1 rung 的 Extreme 假 ⟹ 落 gate_pass 但不入 extreme_true_pass（只计 cond3∧r_extreme）。
        // 与段2 正交的第二个潜在漏计（段2 前即存在，只是历史窗口未触发；段2 改门语义后暴露）。
        let mut extreme_gate_noext = 0usize;
        // 路径A 判别（塔一致性回归探针）：cond3_reach ⟹ knode.sub_moves 有 end==src ⟹ tower[lvl] 必有 end==src
        // （sub_moves 是 tower[lvl] 窗口的携坐标副本，recursive_tower 不变量）⟹ base 定位（line 580）必成功。
        // 若 base 定位失败 ⟹ 塔不一致（B4 换装/增量塔重标定引入的真回归），计入 cert_none_path_a。
        let mut cert_none_path_a = 0usize;
        let mut leg_gap_hist = [0usize; 16]; // cond3 到达域的 target_idx−j（s_prev 与 s 间距，=2 ⟹ 单条反向腿=codex 常见结构 s_prev==m1）
        let mut base_conf_false = 0usize;   // confirm_side(δ) 假（δ 与 bits 侧不符 ⟹ base 拒，非互斥）

        // ── 阶段0 诊断探针（区间套问题①：端点相等 vs 区间包含 base 级定位）。 ──
        // 端点相等 find_move_by_end_index(tower[lvl],src) vs 区间包含 start≤src≤end（同一 exec_moves）。
        // false negative = 区间包含命中 ∧ 端点相等未命中（=区间套.pdf §一 3 反例的系统性表现）。
        // 若 base_false_neg=0 ⟹ 差异为零 ⟹ NO-SHIP（端点相等结论获区间包含加固，depth 归零非本口径伪影）。
        let mut base_ep_hit = 0usize;              // 端点相等口径命中
        let mut base_ct_hit = 0usize;              // 区间包含口径命中
        let mut base_false_neg = 0usize;           // 包含命中 ∧ 端点未命中 = 问题① false negative
        let mut base_fn_by_level = [0usize; LMAX + 1];

        // 逐信号明细（前 60 条拒绝样本，spot-check）。
        let mut detail: Vec<String> = Vec::new();
        const DETAIL_CAP: usize = 60;

        let mut classifier_incr = IncrementalClassifier::new(bars, &config);
        let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();

        for i in 0..n {
            let bar = &bars[i];
            if bar.untradable || bar.close <= 0 {
                continue;
            }
            let (cls_i, tower_i) = classifier_incr.classify_at(i);
            for (lvl, ls) in cls_i.levels.iter().enumerate() {
                if !in_scope(lvl) {
                    continue;
                }
                for p in ls.bsp.iter() {
                    let bsp_class = bsp_disc(&p.bits);
                    if !seen.insert((lvl, p.source_index, bsp_class)) {
                        continue;
                    }
                    // 只看第二类（buy2/sell2）——acc 报告 level1-4 全为第二类，防御性过滤。
                    if !(p.bits.buy2 || p.bits.sell2) {
                        continue;
                    }
                    // Γ 组装（bit-exact 复制生产路径）。
                    let single = super::super::super::classifier::Classification {
                        levels: cls_i.levels.iter().enumerate()
                            .map(|(l2, _)| super::super::super::classifier::LevelState {
                                moves: Vec::new(), centers: Rc::new(Vec::new()),
                                bsp: Rc::new(if l2 == lvl { vec![p.clone()] } else { Vec::new() }),
                                pan_div: Rc::new(Vec::new()), // Q4：dx 与生产 single 同形（无盘整背驰载荷）
                            })
                            .collect(),
                    };
                    for c in &assemble_gamma_with_tower(&single, &tower_i) {
                        let delta = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => continue,
                        };
                        let src = p.source_index;
                        n_total += 1;
                        n_by_level[lvl] += 1;
                        if !p.bits.confirm_side(delta) {
                            base_conf_false += 1;
                        }

                        // 阶段0 探针：base 级两口径对照（无条件，覆盖全信号）。
                        {
                            let exec_moves = tower_i[lvl].as_slice();
                            let ep = find_move_by_end_index(exec_moves, src).is_some();
                            let ct = exec_moves.iter().any(|m| m.start_index <= src && src <= m.end_index);
                            if ep { base_ep_hit += 1; }
                            if ct { base_ct_hit += 1; }
                            if ct && !ep { base_false_neg += 1; base_fn_by_level[lvl] += 1; }
                        }

                        // 门判定（生产同源）。
                        let cert_opt = build_nest_certificate(&tower_i, lvl, src, delta, &p.bits, &macd_hist);
                        let gate = cert_opt.as_ref().map(|c| c.n_delta()).unwrap_or(false);

                        // rung k=lvl+1 div_cand 分解（独立于 gate，用于阶段归因 + 互斥统计）。
                        let mut r_knode = false;
                        let mut r_target = false;
                        let mut r_cond1 = false;
                        let mut r_cond2 = false;
                        let mut r_extreme = false; // cond3
                        let mut r_divcand = false; // rung lvl+1 完整 div_cand
                        let mut r_leggap = 0usize;
                        let mut s_ext = 0i64;      // s 的极值（Long=lo / Short=hi），明细用
                        let mut sp_ext = 0i64;     // s_prev 的极值，明细用
                        if let Some(upper) = tower_i.get(lvl + 1) {
                            if let Some(knode) = upper.iter().find(|m| m.start_index <= src && src <= m.end_index) {
                                r_knode = true;
                                // C3 后新形态：直读 LeveledMove（rmove_dir + rmove.lo()/hi() 惰性派生），与生产 div_cand 同源。
                                let subs = knode.sub_moves.as_slice();
                                if let Some(tidx) = subs.iter().position(|m| m.end_index == src) {
                                    r_target = true;
                                    let s = &subs[tidx];
                                    let s_dir = rmove_dir(&s.rmove);
                                    let expected = match delta {
                                        Side::Long => Direction::Down,
                                        Side::Short => Direction::Up,
                                    };
                                    r_cond1 = s_dir == expected;
                                    if r_cond1 {
                                        if let Some((j, sp)) = subs[..tidx].iter().enumerate().rev()
                                            .find(|(_, m)| rmove_dir(&m.rmove) == s_dir)
                                        {
                                            r_cond2 = true;
                                            r_leggap = tidx - j;
                                            r_extreme = match delta {
                                                Side::Long => s.rmove.lo() < sp.rmove.lo(),
                                                Side::Short => s.rmove.hi() > sp.rmove.hi(),
                                            };
                                            s_ext = match delta { Side::Long => s.rmove.lo(), Side::Short => s.rmove.hi() };
                                            sp_ext = match delta { Side::Long => sp.rmove.lo(), Side::Short => sp.rmove.hi() };
                                        }
                                    }
                                    r_divcand = div_cand(&DivCandInput {
                                        context: subs, target_idx: tidx, hist: &macd_hist, delta,
                                    });
                                }
                            }
                        }

                        let cond3_reach = r_knode && r_target && r_cond1 && r_cond2;
                        if cond3_reach {
                            cond3_reached += 1;
                            leg_gap_hist[r_leggap.min(15)] += 1;
                        }

                        // 阶段归因（互斥）。
                        let stage: &str;
                        if gate {
                            st_gate_pass += 1;
                            stage = "gate_pass";
                            if cond3_reach && r_extreme { extreme_true_pass += 1; }
                            else if cond3_reach { extreme_gate_noext += 1; }
                        } else if cert_opt.is_none() {
                            st_base_none += 1;
                            stage = "base_none";
                            if cond3_reach {
                                extreme_cert_none += 1;
                                if find_move_by_end_index(tower_i[lvl].as_slice(), src).is_none() {
                                    cert_none_path_a += 1; // base 定位失败 = 塔不一致回归
                                }
                            }
                        } else if !r_knode {
                            st_no_upper += 1;
                            stage = "no_upper";
                        } else if !r_target {
                            st_no_target += 1;
                            stage = "no_target";
                        } else if !r_cond1 {
                            st_cond1_dir += 1;
                            stage = "cond1_dir";
                        } else if !r_cond2 {
                            st_cond2_noprev += 1;
                            stage = "cond2_noprev";
                        } else if !r_extreme {
                            st_cond3_extreme += 1; // ★互斥链
                            stage = "cond3_extreme";
                        } else if !r_divcand {
                            st_cond4_weak += 1;
                            extreme_true_reject += 1; // cond3 通过（Extreme 真）但 cond4 拒
                            stage = "cond4_weak";
                        } else {
                            st_reject_elsewhere += 1;
                            extreme_true_reject += 1; // rung lvl+1 cand 真但 gate 拒（更高 rung/is_sub）
                            stage = "reject_elsewhere";
                        }

                        if stage != "gate_pass" && detail.len() < DETAIL_CAP {
                            let dbits = bsp_class;
                            detail.push(format!(
                                "| {lvl} | {src} | {} | {dbits:#04x} | {stage} | c1={} c2={} ext={} dc={} | gap={} | s={} sp={} |",
                                if delta == Side::Long { "+1" } else { "-1" },
                                r_cond1 as u8, r_cond2 as u8, r_extreme as u8, r_divcand as u8,
                                r_leggap, s_ext, sp_ext,
                            ));
                        }
                    }
                }
            }
        }

        // ── 报告 ──
        let mut rpt = String::new();
        let _ = writeln!(rpt, "# H2 样本级验证原始数据：level1-4 第二类信号 N^δ 门拒绝阶段分解");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "- task: #8（codex H2 边界条件 L2 收口）");
        let _ = writeln!(rpt, "- **认识论等级**：L2（真实 BTC 全历史逐信号分解，可产否定性结果）");
        let _ = writeln!(rpt, "- **窗口**：bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");
        let _ = writeln!(rpt, "- **目标域**：level {LMIN}..={LMAX} 第二类(buy2/sell2)信号，进入 Γ 后逐信号分解");
        let _ = writeln!(rpt);

        let _ = writeln!(rpt, "## 1. 信号计数（各级进入 Γ 的第二类信号数）");
        let _ = writeln!(rpt, "| level | 进入Γ信号数 |");
        let _ = writeln!(rpt, "|---|---|");
        for l in LMIN..=LMAX {
            let _ = writeln!(rpt, "| {l} | {} |", n_by_level[l]);
        }
        let _ = writeln!(rpt, "| **合计** | **{n_total}** |");
        let _ = writeln!(rpt);

        let _ = writeln!(rpt, "## 2. N^δ 门拒绝阶段分解（rung k=lvl+1 div_cand，互斥优先序）");
        let _ = writeln!(rpt, "| 阶段 | 信号数 | 占比 | 含义 |");
        let _ = writeln!(rpt, "|---|---|---|---|");
        let pct = |x: usize| if n_total > 0 { 100.0 * x as f64 / n_total as f64 } else { 0.0 };
        let _ = writeln!(rpt, "| base_none | {} | {:.2}% | tower[lvl] 无 end==src 候选段（无定位） |", st_base_none, pct(st_base_none));
        let _ = writeln!(rpt, "| no_upper | {} | {:.2}% | tower[lvl+1] 无含 src 段 ⟹ rungs 空退化 base-case |", st_no_upper, pct(st_no_upper));
        let _ = writeln!(rpt, "| no_target | {} | {:.2}% | knode.sub_moves 无 end==src |", st_no_target, pct(st_no_target));
        let _ = writeln!(rpt, "| cond1_dir | {} | {:.2}% | dir(m2)≠−δ（m2 非背驰段要求方向） |", st_cond1_dir, pct(st_cond1_dir));
        let _ = writeln!(rpt, "| cond2_noprev | {} | {:.2}% | 无前序同向段 s_prev |", st_cond2_noprev, pct(st_cond2_noprev));
        let _ = writeln!(rpt, "| **cond3_extreme** | **{}** | **{:.2}%** | **Extreme 假（m2 未创新极值）= codex 互斥链** |", st_cond3_extreme, pct(st_cond3_extreme));
        let _ = writeln!(rpt, "| cond4_weak | {} | {:.2}% | cond1∧2∧3 过但 div_cand=false ⟹ MACD 力度未衰减 |", st_cond4_weak, pct(st_cond4_weak));
        let _ = writeln!(rpt, "| reject_elsewhere | {} | {:.2}% | rung lvl+1 cand=true 但 gate=false（更高 rung/is_sub） |", st_reject_elsewhere, pct(st_reject_elsewhere));
        let _ = writeln!(rpt, "| gate_pass | {} | {:.2}% | 通过门 |", st_gate_pass, pct(st_gate_pass));
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "- confirm_side(δ) 假（δ 与 bits 侧不符，base 层拒，与互斥正交）：{base_conf_false}");
        let _ = writeln!(rpt);

        // ── 阶段0 诊断探针：区间套问题① base 级两口径对照 ──
        let _ = writeln!(rpt, "## 2b. 阶段0 诊断探针：区间套问题①（端点相等 vs 区间包含 base 级定位）");
        let _ = writeln!(rpt, "| 口径 | 命中数 | 占比 |");
        let _ = writeln!(rpt, "|---|---|---|");
        let _ = writeln!(rpt, "| 端点相等 find_move_by_end_index(tower[lvl],src) | {base_ep_hit} | {:.2}% |", pct(base_ep_hit));
        let _ = writeln!(rpt, "| 区间包含 start≤src≤end（同 exec_moves） | {base_ct_hit} | {:.2}% |", pct(base_ct_hit));
        let _ = writeln!(rpt, "| **false negative（包含命中∧端点未命中）** | **{base_false_neg}** | **{:.2}%** |", pct(base_false_neg));
        for l in LMIN..=LMAX {
            let _ = writeln!(rpt, "| ↳ level {l} FN | {} | |", base_fn_by_level[l]);
        }
        let _ = writeln!(rpt, "- **判据**：base_false_neg=0 ⟹ 两口径无差异 ⟹ **NO-SHIP**（端点相等结论获区间包含加固，depth 归零非本口径伪影，无须改生产）。");
        let _ = writeln!(rpt, "- base_false_neg>0 ⟹ 端点相等系统性漏检坐实 ⟹ 按区间套.pdf §一 4 改区间包含定位（bottom-up + Sel_Θ 唯一），重跑三件套。");
        let _ = writeln!(rpt);

        // ── deliverable (a)：Extreme必假占比（cond3 到达域）──
        let _ = writeln!(rpt, "## 3. deliverable (a)：Extreme 必假占比（codex 等价 s_prev 使互斥成立）");
        let extreme_false = st_cond3_extreme; // cond3 到达且 Extreme 假 ⟹ 必然归入 cond3_extreme（全拒）
        let extreme_true = cond3_reached.saturating_sub(extreme_false);
        let ext_false_pct = if cond3_reached > 0 { 100.0 * extreme_false as f64 / cond3_reached as f64 } else { 0.0 };
        let _ = writeln!(rpt, "- 到达 cond3 的信号数（cond1∧cond2 通过）：{cond3_reached}");
        let _ = writeln!(rpt, "- 其中 **Extreme 必假**（互斥成立，s_prev 使 m2 无法创新极值）：{extreme_false}（{ext_false_pct:.2}%）");
        let _ = writeln!(rpt, "- 其中 **Extreme 可满足**（m2 创新极值 vs 最近同向 s_prev=q）：{extreme_true}");
        let _ = writeln!(rpt, "\n**s_prev 与 s 间距（leg_gap=target_idx−j）分布**（=2 ⟹ 单条反向腿=codex「常见结构 s_prev==m1」；>2 ⟹ 多同向腿=可能 q≠m1）：");
        let _ = writeln!(rpt, "| leg_gap | 信号数 |");
        let _ = writeln!(rpt, "|---|---|");
        for g in 0..16 {
            if leg_gap_hist[g] == 0 { continue; }
            let _ = writeln!(rpt, "| {}{} | {} |", if g == 15 { "≥" } else { "" }, g, leg_gap_hist[g]);
        }
        let _ = writeln!(rpt);

        // ── deliverable (b)：Extreme 可满足（gap）样本的门分布 ──
        let _ = writeln!(rpt, "## 4. deliverable (b)：s_prev≠m1（Extreme 可满足 gap）样本的门通过/拒绝分布");
        let _ = writeln!(rpt, "- Extreme 可满足样本总数：{extreme_true}");
        let _ = writeln!(rpt, "  - 门**通过**（gate_pass）：{extreme_true_pass}");
        let _ = writeln!(rpt, "  - 门**拒绝**（cond4_weak / reject_elsewhere）：{extreme_true_reject}");
        let _ = writeln!(rpt, "\n**读解**：若 extreme_true=0 ⟹ 无 gap 样本，互斥链在 cond3 到达域内完全成立（Extreme 必假）。\
            若 extreme_true>0 且门拒 ⟹ 这些 gap 样本被 cond4/更高 rung/is_sub 拒（非 cond3 互斥）——\
            互斥链**不**是它们归零的原因。");
        let _ = writeln!(rpt);

        // ── deliverable (c)：100% 归零是否完全由互斥链解释 ──
        let _ = writeln!(rpt, "## 5. deliverable (c)：level1-4 100%归零是否完全由 cond3 互斥链解释");
        let reject_total = n_total.saturating_sub(st_gate_pass);
        let excl_pct = if reject_total > 0 { 100.0 * st_cond3_extreme as f64 / reject_total as f64 } else { 0.0 };
        let _ = writeln!(rpt, "- 被门拒信号数：{reject_total} / {n_total}（gate_pass={st_gate_pass}）");
        let _ = writeln!(rpt, "- 其中 cond3 互斥链（Extreme 必假）解释：{}（占拒绝 {excl_pct:.2}%）", st_cond3_extreme);
        let other = reject_total.saturating_sub(st_cond3_extreme);
        let _ = writeln!(rpt, "- 其他机制（base_none/no_upper/no_target/cond1/cond2/cond4/elsewhere）解释：{other}");
        let verdict = if st_gate_pass > 0 {
            "**部分归零**：存在通过门的 level1-4 信号 ⟹ 与 acc「100%归零」不符（窗口差异，跨窗核对）。"
        } else if other == 0 {
            "**完全由互斥链解释**：所有拒绝都是 cond3 Extreme 必假（s_prev 使 m2 无法创新极值）——codex H2 互斥链 100% 坐实，边界缺口未实现（无 s_prev≠m1 使 Extreme 可满足的样本）。"
        } else if st_cond3_extreme == 0 {
            "**完全不由互斥链解释**：无信号止于 cond3——归零由其他机制（方向/无前段/base/更高级 rung）主导，codex 互斥链在本域不是主因。"
        } else {
            "**混合（互斥链非唯一机制）**：cond3 互斥链解释部分归零，其余由 cond1 方向/cond2 无前段/base/cond4/更高 rung 解释——codex H2 互斥链在 level1-4 **部分成立**，边界缺口（其他机制）实证存在，裁决需标注「互斥链是子集机制，非全部」。"
        };
        let _ = writeln!(rpt, "\n### 判定：{verdict}");
        let _ = writeln!(rpt);

        // ── 明细（前 {DETAIL_CAP} 条拒绝样本）──
        let _ = writeln!(rpt, "## 6. 拒绝样本明细（前 {} 条，spot-check）", detail.len());
        let _ = writeln!(rpt, "| lvl | src | δ | bits | 阶段 | cond flags | leg_gap | s极值 s_prev极值 |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|");
        for line in &detail {
            let _ = writeln!(rpt, "{line}");
        }
        let _ = writeln!(rpt);

        // 落盘（原始数据；六要素结果包由 owner 用 Write 工具单独落盘 h2-sample-verification）。
        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/h2-sample-raw-20260702.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告失败：{e}"));
        eprintln!("\n原始数据已落盘：{out:?}");

        // ── 真封（计数不变量）──
        let stage_sum = st_base_none + st_no_upper + st_no_target + st_cond1_dir
            + st_cond2_noprev + st_cond3_extreme + st_cond4_weak + st_reject_elsewhere + st_gate_pass;
        assert_eq!(stage_sum, n_total,
            "阶段分解穷举：Σ阶段({stage_sum}) 应 = n_total({n_total})");
        // 673 号段2 landing 后守恒扩展：cond3 到达域现分四桶（Extreme必假 / Extreme真通过 / Extreme真拒 /
        // cert=None 段2 小转大门拒）。旧三桶守恒（写于段2 前，隐含 cond3_reach⟹cert.is_some）已过时——
        // 段2 门（build_nest_certificate line 599）合法地对 Type2/3 小转大信号 return None，这些信号 cond3 到达
        // 但落 base_none 短路，绕过 extreme 三桶。补第四桶 extreme_cert_none 使分解重新穷举。
        assert_eq!(cert_none_path_a, 0,
            "塔一致性回归探针：cond3_reach 信号 base 定位失败 {cert_none_path_a} 例 ⟹ tower[lvl] 与 knode.sub_moves \
             不一致（B4 换装/增量塔重标定引入的真回归，非段2 小转大）。应为 0。");
        assert_eq!(
            extreme_false + extreme_true_pass + extreme_true_reject + extreme_cert_none + extreme_gate_noext,
            cond3_reached,
            "cond3 到达域守恒（五桶穷举）：Extreme必假({extreme_false})+Extreme真通过({extreme_true_pass})\
             +Extreme真拒({extreme_true_reject})+cert=None段2门拒({extreme_cert_none})\
             +gate通过∧Extreme假({extreme_gate_noext}) 应 = cond3_reached({cond3_reached})");
        eprintln!("真封：Σ阶段={stage_sum}=n_total={n_total}；cond3_reached={cond3_reached}=必假{extreme_false}+真通过{extreme_true_pass}+真拒{extreme_true_reject}+段2门拒{extreme_cert_none}+门通过Ext假{extreme_gate_noext}（path_a回归={cert_none_path_a}）");
    }

    /// **L2-dist（task #23）：段2 全历史 depth/小转大分布收集器**（临时 collector，非交付、不入生产路径）。
    ///
    /// 复用 `h2_sample_exclusion_dx` 的 bit-exact classify 循环，对 level1-4 每条 Type2/3 信号
    /// （per-delta `!is_type1 && (is_type2||is_type3)`）调私有 `descend_type1_anchor_depth`：
    /// `None`=小转大（次级别无一类锚点，精确点无法下沉定位）计数；`Some(d)`=区间套下沉深度直方图。
    /// 另抽样验证锚点正确性（`s.sub_moves` 中 `end_index==source_index` 段方向=−δ 回抽方向）。
    ///
    /// 命令：`ECON_L2_MAX_BARS=100000000 cargo test --release l2_depth_distribution_dx -- --ignored --nocapture`
    /// （默认 max_bars=usize::MAX ⟹ 全历史；段1 全历史先例 ≈39min）。
    #[test]
    #[ignore]
    fn l2_depth_distribution_dx() {
        use super::super::data;
        use super::super::super::classifier::divergence::compute_macd;
        use super::super::super::classifier::cand_predicate::rmove_dir;
        use super::super::super::strategy::interp::assemble_gamma_with_tower;
        use super::super::super::strategy::voice::VoiceSide;
        use super::super::super::types::{Side, Direction};
        use super::super::incremental::IncrementalClassifier;
        use std::fmt::Write as _;

        let config = ThetaConfig::default();
        let ds_full = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => panic!("BTC 加载失败：{e}（DATA BLOCKER，不伪造合成）"),
        };
        let n_full = ds_full.bars.len();
        let max_bars = std::env::var("ECON_L2_MAX_BARS").ok()
            .and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
        let ds = if n_full > max_bars {
            ds_full.slice_bar_range(n_full - max_bars, n_full)
        } else {
            ds_full
        };
        let bars = &ds.bars;
        let n = bars.len();
        let tick = config.tick.tick_size;
        let win_start = ds.dates.first().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        let win_end = ds.dates.last().map(|d| d.get(..10).unwrap_or("").to_string()).unwrap_or_default();
        eprintln!("[l2-depth-dx] bars={n}（{win_start}→{win_end}，全量={n_full}），max_bars={max_bars}");

        let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
        let macd_hist = compute_macd(&closes, &config.macd).hist;

        const LMIN: usize = 1;
        const LMAX: usize = 4;
        const DCAP: usize = 8; // depth 直方图桶上限（level4 下沉理论 ≤4，8 留冗余）
        let in_scope = |lvl: usize| (LMIN..=LMAX).contains(&lvl);

        // Type2/3 主群（per-delta `!is_type1 && (is_type2||is_type3)`）——task 报告口径。
        let mut n_t23 = [0usize; LMAX + 1];         // 各级 Type2/3 信号总数
        let mut base_none_t23 = [0usize; LMAX + 1]; // tower[lvl] 无 end==src 候选段（descent 未触达）
        let mut xzd_t23 = [0usize; LMAX + 1];       // descend=None = 小转大（次级别无一类锚点）
        let mut depth_t23 = [[0usize; DCAP + 1]; LMAX + 1]; // descend=Some(d) 深度直方图
        let mut depth_overflow = 0usize;            // d>DCAP 溢出（clamp 记录）
        let mut max_depth = 0usize;

        // 残差群（`!is_type1` 但既非 type2 也非 type3）——诚实标注：门的 descent 域比 Type2/3 略宽。
        let mut n_other = 0usize;
        let mut none_other = 0usize; // base_none + descend=None 合并
        let mut some_other = 0usize;

        // 锚点正确性抽样（Some(d) 结果的 end==src ∧ 方向=−δ）。
        const SAMPLE_CAP: usize = 200;
        let mut sample_n = 0usize;
        let mut sample_pass = 0usize;
        let mut sample_fail_detail: Vec<String> = Vec::new();

        let mut classifier_incr = IncrementalClassifier::new(bars, &config);
        let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();

        for i in 0..n {
            let bar = &bars[i];
            if bar.untradable || bar.close <= 0 {
                continue;
            }
            let (cls_i, tower_i) = classifier_incr.classify_at(i);
            for (lvl, ls) in cls_i.levels.iter().enumerate() {
                if !in_scope(lvl) {
                    continue;
                }
                for p in ls.bsp.iter() {
                    let bsp_class = bsp_disc(&p.bits);
                    if !seen.insert((lvl, p.source_index, bsp_class)) {
                        continue;
                    }
                    // 预过滤：至少带一个 type2/3 bit（type1-only 与无 bsp bit 信号非本群，省 Γ 组装）。
                    if !(p.bits.buy2 || p.bits.sell2 || p.bits.buy3 || p.bits.sell3) {
                        continue;
                    }
                    // Γ 组装（bit-exact 复制生产路径）取交易方向 δ。
                    let single = super::super::super::classifier::Classification {
                        levels: cls_i.levels.iter().enumerate()
                            .map(|(l2, _)| super::super::super::classifier::LevelState {
                                moves: Vec::new(), centers: Rc::new(Vec::new()),
                                bsp: Rc::new(if l2 == lvl { vec![p.clone()] } else { Vec::new() }),
                                pan_div: Rc::new(Vec::new()), // Q4：dx 与生产 single 同形（无盘整背驰载荷）
                            })
                            .collect(),
                    };
                    for c in &assemble_gamma_with_tower(&single, &tower_i) {
                        let delta = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => continue,
                        };
                        let src = p.source_index;
                        // per-delta 类型（与 build_nest_certificate 的 is_type1 同源）。
                        let is_type1 = match delta { Side::Long => p.bits.buy1, Side::Short => p.bits.sell1 };
                        if is_type1 {
                            continue; // Type1 走本级 div_cand，不下沉——非本群
                        }
                        let is_type2 = match delta { Side::Long => p.bits.buy2, Side::Short => p.bits.sell2 };
                        let is_type3 = match delta { Side::Long => p.bits.buy3, Side::Short => p.bits.sell3 };
                        let is_t23 = is_type2 || is_type3;

                        // 执行级候选段 s（build_nest_certificate line 542 同逻辑）。
                        let s_opt = tower_i.get(lvl).and_then(|mv| mv.iter().find(|m| m.end_index == src));
                        // descent 结果（s 缺失 ⟹ 生产门早退 None，视作 base_none）。
                        let descend = s_opt.map(|s| descend_type1_anchor_depth(s, src, delta, &macd_hist));

                        if is_t23 {
                            n_t23[lvl] += 1;
                            match descend {
                                None => base_none_t23[lvl] += 1,       // 无候选段
                                Some(None) => xzd_t23[lvl] += 1,       // 小转大
                                Some(Some(d)) => {
                                    max_depth = max_depth.max(d);
                                    if d > DCAP { depth_overflow += 1; }
                                    depth_t23[lvl][d.min(DCAP)] += 1;
                                    // 抽样锚点正确性
                                    if sample_n < SAMPLE_CAP {
                                        sample_n += 1;
                                        let s = s_opt.unwrap();
                                        let subs = s.sub_moves.as_slice();
                                        let tidx = subs.iter().position(|m| m.end_index == src)
                                            .expect("Some(d) ⟹ 存在 end==src 锚段");
                                        let cm_dir = rmove_dir(&subs[tidx].rmove);
                                        let expected = match delta { Side::Long => Direction::Down, Side::Short => Direction::Up };
                                        let end_ok = subs[tidx].end_index == src;
                                        let dir_ok = cm_dir == expected;
                                        if end_ok && dir_ok {
                                            sample_pass += 1;
                                        } else if sample_fail_detail.len() < 20 {
                                            sample_fail_detail.push(format!(
                                                "| {lvl} | {src} | {} | end_ok={end_ok} dir_ok={dir_ok}（got {:?} exp {:?}）|",
                                                if delta == Side::Long { "+1" } else { "-1" }, cm_dir, expected));
                                        }
                                    }
                                }
                            }
                        } else {
                            n_other += 1;
                            match descend {
                                Some(Some(_)) => some_other += 1,
                                _ => none_other += 1,
                            }
                        }
                    }
                }
            }
        }

        // ── 报告 ──
        let pct = |x: usize, tot: usize| if tot == 0 { 0.0 } else { 100.0 * x as f64 / tot as f64 };
        let tot_t23: usize = n_t23.iter().sum();
        let tot_base_none: usize = base_none_t23.iter().sum();
        let tot_xzd: usize = xzd_t23.iter().sum();
        let tot_some: usize = tot_t23 - tot_base_none - tot_xzd;

        let mut rpt = String::new();
        let _ = writeln!(rpt, "# L2-dist 原始数据：段2 全历史 depth/小转大分布（level1-4 Type2/3）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "- task: #23（acc-optB-l2dist）");
        let _ = writeln!(rpt, "- **认识论等级**：L2（真实 BTC 全历史逐信号结构下钻，确定性 div_cand，可产否定性计数）");
        let _ = writeln!(rpt, "- 窗口：{win_start}→{win_end}，bars={n}（全量={n_full}），max_bars={max_bars}");
        let _ = writeln!(rpt, "- 群定义：per-delta `!is_type1 && (is_type2||is_type3)`（Type2/3 主群，域=生产门 descent 触发域子集）");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 1. Type2/3 主群：各 level 分布");
        let _ = writeln!(rpt, "| lvl | 信号总数 | base_none(无候选段) | 小转大(descend None) | 有锚(Some d) | 小转大% |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|");
        for lvl in LMIN..=LMAX {
            let some_l = n_t23[lvl] - base_none_t23[lvl] - xzd_t23[lvl];
            let _ = writeln!(rpt, "| {lvl} | {} | {} | {} | {} | {:.2}% |",
                n_t23[lvl], base_none_t23[lvl], xzd_t23[lvl], some_l, pct(xzd_t23[lvl], n_t23[lvl]));
        }
        let _ = writeln!(rpt, "| **合计** | **{tot_t23}** | **{tot_base_none}** | **{tot_xzd}** | **{tot_some}** | **{:.2}%** |",
            pct(tot_xzd, tot_t23));
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 2. depth 直方图（descend=Some(d)，逐 level × 深度）");
        let _ = write!(rpt, "| lvl \\ d ");
        for d in 1..=DCAP { let _ = write!(rpt, "| d={d} "); }
        let _ = writeln!(rpt, "|");
        let _ = write!(rpt, "|---");
        for _ in 1..=DCAP { let _ = write!(rpt, "|---"); }
        let _ = writeln!(rpt, "|");
        for lvl in LMIN..=LMAX {
            let _ = write!(rpt, "| {lvl} ");
            for d in 1..=DCAP { let _ = write!(rpt, "| {} ", depth_t23[lvl][d]); }
            let _ = writeln!(rpt, "|");
        }
        let _ = writeln!(rpt, "\n- max_depth 观测 = {max_depth}；depth>DCAP({DCAP}) 溢出 = {depth_overflow}");
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 3. 锚点正确性抽样（Some(d) 结果 end_index==src ∧ 方向=−δ）");
        let _ = writeln!(rpt, "- 抽样数：{sample_n}（cap={SAMPLE_CAP}）；通过：{sample_pass}；失败：{}",
            sample_n - sample_pass);
        if !sample_fail_detail.is_empty() {
            let _ = writeln!(rpt, "\n失败明细（前 {} 条）：", sample_fail_detail.len());
            let _ = writeln!(rpt, "| lvl | src | δ | 校验 |");
            let _ = writeln!(rpt, "|---|---|---|---|");
            for line in &sample_fail_detail { let _ = writeln!(rpt, "{line}"); }
        }
        let _ = writeln!(rpt);
        let _ = writeln!(rpt, "## 4. 残差群（`!is_type1` 但非 type2/3，门 descent 域内、本报告群外）");
        let _ = writeln!(rpt, "- 总数：{n_other}；有锚(Some d)：{some_other}；无锚(None/base_none)：{none_other}");
        let _ = writeln!(rpt, "- 诚实标注：生产门 descent 触发域=`!is_type1 && lvl>=1`，比 Type2/3 主群宽 {n_other} 条；");
        let _ = writeln!(rpt, "  这些是 Γ 定向为非 Flat、带 δ 但该方向无 type2/3 bit 的信号（多为对侧 bit 或纯结构 voice）。");
        let _ = writeln!(rpt);

        // 落盘原始数据（六要素结果包由 owner 用 Write 单独落盘 l2-depth-distribution）。
        eprint!("{rpt}");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("rust/ 父目录 = 项目根")
            .join(".chanlun/review-results/l2-depth-raw-20260702.md");
        std::fs::write(&out, &rpt).unwrap_or_else(|e| panic!("写报告失败：{e}"));
        eprintln!("\n原始数据已落盘：{out:?}");

        // ── 真封（计数不变量）：主群逐 level 三桶穷举 = 信号总数。 ──
        for lvl in LMIN..=LMAX {
            let sum = base_none_t23[lvl] + xzd_t23[lvl]
                + (0..=DCAP).map(|d| depth_t23[lvl][d]).sum::<usize>();
            assert_eq!(sum, n_t23[lvl],
                "level{lvl} 三桶穷举：base_none+小转大+Σdepth({sum}) 应 = 信号总数({})", n_t23[lvl]);
        }
        assert_eq!(tot_base_none + tot_xzd + tot_some, tot_t23, "合计三桶穷举");
        eprintln!("真封：Type2/3 主群={tot_t23}=base_none{tot_base_none}+小转大{tot_xzd}+有锚{tot_some}；\
            抽样 {sample_pass}/{sample_n} 锚点正确；残差群={n_other}");
    }

    /// **P1 FullNest L1：effective_nest_depth 前缀语义（build_nest_certificate 与门共用构造的读数）**。
    ///
    /// depth = 从 rungs[0]（最高级）起连续 cand=true 的层数。混合 cand 时在首个 false 处截断。
    /// **认识论 L1**（合成证书结构验证，非 alpha）：验证深度读数与 n_delta 短路语义一致。
    #[test]
    fn effective_nest_depth_prefix_semantics() {
        use super::super::super::classifier::nest::{NestCertificate, NestInterval, NestRung};
        let mut bits = BspBits::default();
        bits.buy1 = true;
        let iv = |et: u64| NestInterval { end_time: et, start_time: 0, idx: 0 };
        // 全 cand=true 的 3 级证书 ⟹ depth=3（100⊇80⊇60，均 cand=true，且 base⊆最低 rung）。
        let cert_full = NestCertificate {
            side: Side::Long, terminal: bits, base_interval: iv(50),
            rungs: vec![
                NestRung { interval: iv(100), cand: true },
                NestRung { interval: iv(80),  cand: true },
                NestRung { interval: iv(60),  cand: true },
            ],
        };
        assert_eq!(effective_nest_depth(&cert_full), 3, "全 cand=true ⟹ depth=rungs.len()=3");
        // 空 rungs ⟹ depth=0（纯 base-case，退化）。
        let cert_base = NestCertificate { rungs: vec![], ..cert_full.clone() };
        assert_eq!(effective_nest_depth(&cert_base), 0, "空 rungs ⟹ depth=0（base-case 退化）");
        // 中间 cand=false ⟹ 前缀在首个 false 处截断（depth=1，只数 rungs[0]）。
        let cert_mid = NestCertificate {
            rungs: vec![
                NestRung { interval: iv(100), cand: true },
                NestRung { interval: iv(80),  cand: false }, // 截断处
                NestRung { interval: iv(60),  cand: true },
            ],
            ..cert_full.clone()
        };
        assert_eq!(effective_nest_depth(&cert_mid), 1, "中间 cand=false ⟹ 前缀截断 depth=1");
        // 首级 cand=false ⟹ depth=0（即便有 rungs，n_delta 立即在最高级短路）。
        let cert_top_false = NestCertificate {
            rungs: vec![NestRung { interval: iv(100), cand: false }],
            ..cert_full
        };
        assert_eq!(effective_nest_depth(&cert_top_false), 0, "首级 cand=false ⟹ depth=0");
    }
}
