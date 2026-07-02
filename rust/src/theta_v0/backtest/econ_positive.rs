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
//! 配对链（`TradeRecord` 只有 entry/exit bar），但**信号收集路径**（同 `build_walk_forward_mu`）在确认点
//! 持有 BspPoint——其 `source_index`（bsp.rs:103，L0 原始 K 序的 pivot 端点位置）即信号挂靠的 pivot 端点
//! bar，取其 close 作 P[λ_rev]（入场信号）/P[ρ_rev]（配对出场信号）。故走 μ 路径无需透传 Order 端点价。

use super::data::Dataset;
use super::incremental::IncrementalClassifier;
use super::super::classifier::cand_predicate::ContextMove;
use super::super::classifier::divergence::compute_macd;
use super::super::classifier::nest::{NestCertificate, NestInterval, NestRung};
use super::super::closed_loop::sell::{sell_decision_of, SellDecision};
use super::super::config::ThetaConfig;
use super::super::strategy::interp::assemble_gamma_with_tower;
use super::super::strategy::voice::VoiceSide;
use super::super::types::{Bar, BspBits, Side};

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
    /// **可达性约束（不是补丁）**：上级层走势是 `RMove::Compose`（descend.rs:53），**无 direction 字段**，
    /// 且 `LevelState.moves: Vec<MoveKind>` 的 `MoveKind::Trend` 也丢方向（不分 Up/Down）。走势裁决
    /// `MoveOutcome::Trend(Direction)`（level.rs:37）内部有方向，但在信号收集作用域不可直接读。故取上级
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

/// 逐信号可捕获价差分解（L2）。复用 `build_walk_forward_mu` 的信号收集 + 退出配对模板，
/// 取入场信号与配对出场信号的 pivot 端点（source_index）算 PDF §5 反转交易腿分解（664 号）。
///
/// 返回 `(Vec<SignalDecomp>, SpreadAttribution)`：逐信号分解 + 聚合归因。
///
/// **认识论 L2**：真实数据逐信号分解，可产否定性结果（spread_eaten=true ⟹ 信号集无 alpha）。
pub fn decompose_capturable_spread(data: &Dataset, config: &ThetaConfig) -> (Vec<SignalDecomp>, SpreadAttribution) {
    let bars = &data.bars;
    let n = bars.len();
    let tick = config.tick.tick_size;
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    // ── MACD hist 预计算（DivCand 条件4 Weak 判据所需，W1 工位）。 ──
    // Θ_MACD：全序列一次性计算（O(n)），供 bsp_div_cand 查询 bar 区间面积。
    // ponytail: 预计算一次，信号收集循环 O(1) 查表，无 per-signal 重算。
    let closes: Vec<f64> = bars.iter().map(|b| b.close as f64 / tick as f64).collect();
    let macd_hist = compute_macd(&closes, &config.macd).hist;

    // ── 信号收集（同 build_walk_forward_mu）：逐 bar 因果分类，收新确认买卖点。 ──
    // 664 号：每条信号挂靠的 pivot 端点 = p.source_index（bsp.rs:103，L0 原始 K 序）。
    // 反转交易腿 λ_rev = 入场信号 pivot 端点；ρ_rev 在退出配对时取配对出场信号的 pivot 端点。
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    // 每条信号：(entry_bar=确认 bar τin, dir=交易方向 δ, pivot_bar=信号挂靠 pivot 端点 source_index,
    // lvl=级别, sigma_higher=入场时上级方向态 666 号, bsp_class=bsp_disc(&p.bits) 类型位掩码 W4)。
    // W4 类型透传：bsp_class 是 buy1/2/3+sell1/2/3 的 u8 位掩码（bsp_disc 同口径，seen-set 键已在用），
    // 完整保留一/二/三类+买卖侧信息（可同时置多位，如 buy1+buy3）。纯增字段，不改配对/信号集/识别逻辑。
    let mut signals: Vec<(usize, VoiceSide, usize, u32, i8, u8)> = Vec::new();

    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        let (cls_i, tower_i) = classifier_incr.classify_at(i);
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            for p in &ls.bsp {
                let bsp_class = bsp_disc(&p.bits); // W4：类型位掩码（seen-set 键复用，纯透传）
                if !seen.insert((lvl, p.source_index, bsp_class)) {
                    continue; // 已确认过
                }
                let pivot_bar = p.source_index; // 信号挂靠 pivot 端点（bsp.rs:103）= λ_rev / ρ_rev 取价处
                // dir 经 assemble_gamma 拿（构造仅含该点的单级别分类，同 build_walk_forward_mu）。
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
                            centers: Vec::new(),
                            bsp: if l2 == lvl { vec![p.clone()] } else { Vec::new() },
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
                    if !build_multilevel_nest_cert(&tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist) {
                        continue;
                    }
                    signals.push((i, c.dir, pivot_bar, lvl as u32, sigma_higher, bsp_class));
                }
            }
        }
    }

    // ── 退出配对（664 号反转交易腿）：持有到下一反向新确认信号，ρ_rev = 该配对出场信号的 pivot 端点。 ──
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
        next_long[i] = if signals[i].1 == VoiceSide::Long { i } else { next_long[i + 1] };
        next_short[i] = if signals[i].1 == VoiceSide::Short { i } else { next_short[i + 1] };
    }

    for (idx, &(entry_bar, dir, lambda_rev_bar, level, sigma_higher, bsp_class)) in signals.iter().enumerate() {
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
        if j < ns && signals[j].0 == entry_bar {
            agg.n_same_bar_opposite += 1;
        }
        // 配对须 eb>entry_bar：跳过同 bar opp（平摊 O(1)，同 bar 信号有限）。
        while j < ns && signals[j].0 <= entry_bar {
            j = next_opp[j + 1];
        }
        // 配对出场信号（首个 eb>entry_bar 反向新确认信号，π^bsp owned）：entry_bar(τout) + pivot_bar(ρ_rev) + exit_bsp_class(P7)。
        let (exit_bar, rho_rev_bar, exit_bsp_class) = if j < ns {
            (signals[j].0, signals[j].2, signals[j].5)
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
            exit_decision,
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
    let s = exec_moves.iter().find(|m| m.end_index == source_index)?;
    let base_interval = NestInterval {
        start_time: s.start_index as u64,
        end_time: s.end_index as u64,
        idx: s.id.ordinal,
    };

    // 673 号段1：Cand^δ_ℓ 按 bsp 类型分流。Type1（本级趋势背驰段=区间套原文对象，606 号有效域）
    // 走 div_cand；Type2/Type3 存在性由结构分类前提保证（Type2=一买后回抽不破 第17课L60 完备性，
    // Type3=离开中枢回抽不破 ZG/ZD），免本级背驰段门（消费点见循环内 cand_k）。
    let is_type1 = match delta {
        Side::Long => bits.buy1,
        Side::Short => bits.sell1,
    };

    // rungs：从高级向执行级降序（rungs[0]=最高级，rungs[last]=lvl+1 级）。
    // 对每个上级 k = lvl+1 到 tower.len()-1：
    //   - 找 tower[k] 中包含 source_index 的段（start_index ≤ source_index ≤ end_index）作为区间
    //   - 在 tower[k] 的上级 tower[k+1] 的 sub_moves 中计算 Cand^δ_k
    //   - 若任何级别找不到包含段 ⟹ 提前 false（无上级语境）
    let max_k = tower.len();
    let mut rung_buf: Vec<NestRung> = Vec::new(); // 从低到高先收集，最后反转
    for k in (lvl + 1)..max_k {
        let k_moves = tower[k].as_slice();
        // 找 tower[k] 中包含 source_index 的段（k 级 Compose）。
        let Some(knode) = k_moves.iter().find(|m| {
            m.start_index <= source_index && source_index <= m.end_index
        }) else {
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
        // Cand^δ_k：候选谓词按 bsp 类型分流（673 号）——此分流点即锚定接口，同时容纳两类锚：
        //   Type1 → 本级锚：本级趋势背驰段判据 div_cand（区间套原文对象，606 号有效域；bit-exact 不动）。
        //   Type2/Type3 → 次级别锚：定律一下沉（下次级别找第一类 Type1 区间套），由段2（task #13）实装。
        // 段1 只装存在性（免本级背驰段门），非「Type2/3 永远无 nest」——rung 结构照建，段2 替换分支体。
        let cand_k = if !is_type1 {
            // ponytail: 段1 存在性占位（cand=true）；段2（#13）在此接入次级别 Type1 区间套真锚点。
            true
        } else {
            // k 级 knode 的 sub_moves 是 lvl 到 k-1 级的窗口序列；找 end_index == source_index 的段
            // （即执行级候选段），计算 DivCand 四条件。
            let ctx: Vec<ContextMove> = knode.sub_moves.iter().map(|m| {
                ContextMove::from_rmove(&m.rmove, m.start_index, m.end_index)
            }).collect();
            match knode.sub_moves.iter().position(|m| m.end_index == source_index) {
                Some(tidx) => {
                    super::super::classifier::cand_predicate::div_cand(
                        &super::super::classifier::cand_predicate::DivCandInput {
                            context: &ctx,
                            target_idx: tidx,
                            hist,
                            delta,
                        }
                    )
                }
                None => false, // 执行级候选段不在 k 级次级别序列中 ⟹ 无 Cand
            }
        };
        rung_buf.push(NestRung { interval: interval_k, cand: cand_k });
    }
    // n_delta 期望 rungs[0]=最高级，rungs[last]=lvl+1 级——rung_buf 是低到高，需反转。
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

/// σ_higher：信号所在 level 的上级层（tower[level+1]）末走势端点价净差符号（666 号，见 SignalDecomp.sigma_higher）。
/// +1 净涨 / −1 净跌 / 0 持平；level+1 越界或上级层空 → 0。端点越界/非正 close → 0（诚实，不兜底）。
fn sigma_higher_at(
    tower: &[std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    bars: &[Bar],
    level: usize,
) -> i8 {
    let Some(upper) = tower.get(level + 1) else { return 0 };
    let Some(m) = upper.last() else { return 0 };
    let (s, e) = (bars.get(m.start_index), bars.get(m.end_index));
    match (s, e) {
        (Some(sb), Some(eb)) if sb.close > 0 && eb.close > 0 => {
            (eb.close - sb.close).signum() as i8
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            exit_decision: ExitDecision::CloseRoot,
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
                subs: sub_rmoves,
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
                subs: sub_rmoves,
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
                subs: sub_rmoves,
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
                subs: sub_rmoves,
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

        // per-class (level, δ, bsp_class) 分桶：adverse-only Σcaptured + 真实成交 Σactual_pnl 双口径（664-Q3）。
        // W4：key=(level, δ, bsp_class)，bsp_class 加入消除 buy1/buy2/buy3 混合池稀释（P4 codex 判决）。
        // value=(n, Σab, Σηin, Σηout, Σce, Σcaptured, n_cap_pos, Σactual_spread, Σactual_pnl, n_act_pos)。
        type Bucket = (usize, f64, f64, f64, f64, f64, usize, f64, f64, usize);
        let mut buckets: BTreeMap<(u32, i8, u8), Bucket> = BTreeMap::new();
        for d in &decomps {
            let e = buckets.entry((d.level, d.delta, d.bsp_class)).or_default();
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
        let _ = writeln!(rpt, "## per-class (level, δ, bsp_class) 三键分桶（W4 P4：消除 buy1/2/3 混合池稀释）");
        let _ = writeln!(rpt, "μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。");
        let _ = writeln!(rpt, "bsp_class=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3）。");
        let _ = writeln!(rpt, "| level | δ | bsp_class | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |");
        let _ = writeln!(rpt, "|---|---|---|---|---|---|---|---|---|---|---|");
        for ((lvl, dlt, cls), (n, sab, sin, sout, sce, scap, _ncap, _sact, sactpnl, nact)) in &buckets {
            let mu_hat = if *n > 0 { sactpnl / *n as f64 } else { 0.0 };
            let _ = writeln!(rpt, "| {} | {:+} | 0x{:02x} | {} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {:.4e} | {}/{} ({:.0}%) |",
                lvl, dlt, cls, n, mu_hat, sactpnl, sab, sin + sout, sce, scap,
                nact, n, if *n > 0 { 100.0 * *nact as f64 / *n as f64 } else { 0.0 });
        }
        let _ = writeln!(rpt);
        // ── 663 判据：全级别×方向×类型 μ̂>0 分类（正条件期望，出现就做不统计显著）。 ──
        let _ = writeln!(rpt, "## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）");
        let max_level = buckets.keys().map(|(l, _, _)| *l).max().unwrap_or(0);
        let _ = writeln!(rpt, "涌现最高级别 L={max_level}（全级别 0..{max_level} 均列；稀疏高级别照报不判硬墙，663）。");
        let _ = writeln!(rpt, "**有效域**：本窗 bars={n_bars}（{window_start}→{window_end}）。663 要求全历史长窗——");
        let _ = writeln!(rpt, "若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。");
        let _ = writeln!(rpt, "μ̂>0 类 = 该 (level,δ,bsp_class) 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：");
        let mut n_pos_class = 0usize;
        let mut n_total_class = 0usize;
        for ((lvl, dlt, cls), (n, _, _, _, _, _, _, _, sactpnl, nact)) in &buckets {
            n_total_class += 1;
            let mu_hat = if *n > 0 { sactpnl / *n as f64 } else { 0.0 };
            let mark = if mu_hat > 0.0 { n_pos_class += 1; "✓μ̂>0" } else { "✗μ̂≤0" };
            let _ = writeln!(rpt, "- (level={lvl}, δ={dlt:+}, cls=0x{cls:02x}) {mark}: μ̂={mu_hat:.4e}, Σ={sactpnl:.4e}, n_act+/n={nact}/{n}");
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

    /// P4 类型分桶键 L1 自检：同 (level,δ) 不同 bsp_class 必须分到不同桶（W4 codex 判决）。
    ///
    /// 三个合成信号：(0,+1,buy1=0x01), (0,+1,buy2=0x02), (0,+1,buy1=0x01)。
    /// 预期：桶键 (0,+1,0x01) n=2，桶键 (0,+1,0x02) n=1（混合池已消除）。
    #[test]
    fn bucket_key_includes_bsp_class_l1() {
        use std::collections::BTreeMap;

        let mk = |bsp_class: u8, actual_pnl: f64| SignalDecomp {
            entry_bar: 0, exit_bar: 1, level: 0, delta: 1, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0,
            actual_pnl, sigma_higher: 0, bsp_class, exit_decision: ExitDecision::Hold,
        };
        // buy1 × 2, buy2 × 1：全部 (level=0, δ=+1)，仅 bsp_class 不同。
        let decomps = vec![mk(0x01, 1.0), mk(0x02, 2.0), mk(0x01, 3.0)];

        type Bucket = (usize, f64, f64, f64, f64, f64, usize, f64, f64, usize);
        let mut buckets: BTreeMap<(u32, i8, u8), Bucket> = BTreeMap::new();
        for d in &decomps {
            let e = buckets.entry((d.level, d.delta, d.bsp_class)).or_default();
            e.0 += 1;
            e.8 += d.actual_pnl;
        }

        assert_eq!(buckets.len(), 2, "buy1/buy2 应分为 2 桶（不得混池）");
        let buy1 = buckets.get(&(0, 1, 0x01)).expect("桶 (0,+1,buy1) 应存在");
        assert_eq!(buy1.0, 2, "buy1 桶 n=2");
        assert!((buy1.8 - 4.0).abs() < 1e-9, "buy1 Σactual_pnl=1+3=4");
        let buy2 = buckets.get(&(0, 1, 0x02)).expect("桶 (0,+1,buy2) 应存在");
        assert_eq!(buy2.0, 1, "buy2 桶 n=1");
        assert!((buy2.8 - 2.0).abs() < 1e-9, "buy2 Σactual_pnl=2");
    }

    /// neff/LCB/bootstrap helper L1 自检（合成数据，验证算术，零信息增量但保非平凡逻辑不破）。
    #[test]
    fn walkforward_helpers_l1() {
        // train_winner：(0,-1) Σ=+5 最强，(1,1) Σ=−2 被滤。
        let synth = |level: u32, delta: i8, pnl: f64| SignalDecomp {
            entry_bar: 0, exit_bar: 1, level, delta, a_b: 0.0, x_in: 0.0, y_out: 0.0,
            eta_in: 0.0, eta_out: 0.0, actual_spread: 0.0, ce_unit: 0.0, captured: 0.0, actual_pnl: pnl,
            sigma_higher: 0, bsp_class: 0, exit_decision: ExitDecision::Hold,
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
    /// - 中间级 bsp_pre=0 ⟹ **H3**（架构：mod.rs:247 上级层只产第二类 B2/S2，第一/三类仅 L0；
    ///   第二类稀疏 ⟹ 中间级天然空。不是运行时 bug，是 bsp 提取的 level 语义）。
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
        let mut sig_post = [0usize; LMAX];     // (c) 通过 N^δ 门
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

        let mut classifier_incr = IncrementalClassifier::new(bars, &config);
        let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();

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
                for p in &ls.bsp {
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
                                moves: Vec::new(), centers: Vec::new(),
                                bsp: if l2 == lvl { vec![p.clone()] } else { Vec::new() },
                            })
                            .collect(),
                    };
                    for c in &assemble_gamma_with_tower(&single, &tower_i) {
                        if c.dir == VoiceSide::Flat { continue; }
                        if lvl < LMAX { gamma_nonflat[lvl] += 1; }
                        let delta_side = match c.dir {
                            VoiceSide::Long => Side::Long,
                            VoiceSide::Short => Side::Short,
                            VoiceSide::Flat => continue,
                        };
                        // P1 FullNest：用 build_nest_certificate（与生产门共用构造）拿证书，
                        // 门判定 = cert.n_delta()（bit-exact == build_multilevel_nest_cert）。
                        let Some(cert) = build_nest_certificate(&tower_i, lvl, p.source_index, delta_side, &p.bits, &macd_hist) else {
                            continue; // 执行级无候选段 ⟹ 门拒（同 build_multilevel_nest_cert None 分支）
                        };
                        if !cert.n_delta() {
                            continue;
                        }
                        if lvl < LMAX { sig_post[lvl] += 1; }
                        // 有效跨级深度（通过门 ⟹ 所有 rung cand=true ⟹ depth=rungs.len()）。
                        let depth = effective_nest_depth(&cert).min(LMAX);
                        nest_depth_hist_pass[depth] += 1;
                        if lvl < LMAX { nest_depth_by_level_pass[lvl][depth] += 1; }
                        n_gate_pass_total += 1;
                        if lvl >= 1 {
                            let delta: i8 = if c.dir == VoiceSide::Long { 1 } else { -1 };
                            highlevel_hits.push((lvl, p.source_index, delta, bsp_class, cert.rungs.len()));
                        }
                    }
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
            "**H3（架构性，非运行时 bug）**：中间级 bsp_pre 全 0。根因=mod.rs:247 上级层（level≥1）\
             bsp 提取只产第二类 B2/S2（extract_second_for_level），第一/三类仅 L0 层提取。\
             第二类识别需 upper_moves 的 sub_moves 出现「第一类离开+回拉不创新高/低」结构且背驰——\
             中间级此结构稀疏 ⟹ 中间级天然空洞。**不是 N^δ 门滤空，不是全历史路径 bug**。"
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

        // ── 配对后 decomps level 分布（关键：sig_post 是门后配对前，decomps 是配对后）──
        // 分水岭：若中间级 sig_post>0 但 decomps 该级=0 ⟹ 配对阶段丢失（非门滤空 H1，是右删失/跨级混合配对）。
        let (decomps_prod, agg_prod) = decompose_capturable_spread(&ds, &config);
        let mut decomp_by_level = [0usize; LMAX];
        for d in &decomps_prod {
            if (d.level as usize) < LMAX { decomp_by_level[d.level as usize] += 1; }
        }
        let _ = writeln!(rpt, "## 配对后 decomps level 分布（sig_post=门后配对前 vs decomps=配对后）");
        let _ = writeln!(rpt, "| level | sig_post(门后) | decomps(配对后) | 配对丢失 |");
        let _ = writeln!(rpt, "|---|---|---|---|");
        let mut any_pairing_loss_mid = false;
        for l in 0..LMAX {
            if sig_post[l] == 0 && decomp_by_level[l] == 0 { continue; }
            let lost = sig_post[l].saturating_sub(decomp_by_level[l]);
            if l >= 1 && decomp_by_level[l] == 0 && sig_post[l] > 0 { any_pairing_loss_mid = true; }
            let _ = writeln!(rpt, "| {l} | {} | {} | {} |", sig_post[l], decomp_by_level[l], lost);
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

        // 真封：分级别 sig_post 之和 >= 生产路径 n_signals（门后信号含未配对出场者）。
        let sig_post_sum: usize = sig_post.iter().sum();
        assert!(sig_post_sum >= agg_prod.n_signals,
            "sig_post_sum({sig_post_sum}) 应 >= n_signals({})：门后信号数含未配对出场者", agg_prod.n_signals);
        // 真封②：配对后 decomps 逐级和 = n_signals（聚合完整性）。
        let decomp_sum: usize = decomp_by_level.iter().sum();
        assert_eq!(decomp_sum, agg_prod.n_signals,
            "decomp 逐级和({decomp_sum}) 应 = n_signals({})", agg_prod.n_signals);
        eprintln!("真封：sig_post_sum={sig_post_sum} >= n_signals={} = decomp_sum={decomp_sum}",
            agg_prod.n_signals);
        // 真封③（P1 FullNest）：深度直方图总数 = 通过门总数 = sig_post_sum（build_nest_certificate
        // 与 build_multilevel_nest_cert 门判定 bit-exact 同源，通过门信号无遗漏）。
        let depth_hist_sum: usize = nest_depth_hist_pass.iter().sum();
        assert_eq!(depth_hist_sum, n_gate_pass_total,
            "深度直方图和({depth_hist_sum}) 应 = 通过门总数({n_gate_pass_total})");
        assert_eq!(n_gate_pass_total, sig_post_sum,
            "通过门总数({n_gate_pass_total}) 应 = sig_post_sum({sig_post_sum})（build_nest_certificate.n_delta bit-exact == build_multilevel_nest_cert）");
        eprintln!("真封③（P1）：depth_hist_sum={depth_hist_sum} = n_gate_pass_total={n_gate_pass_total} = sig_post_sum={sig_post_sum}");
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
    /// 共用构造），rung 分解的 `div_cand`/`ContextMove` 与生产同源；cond4(Weak) 用「cond1∧2∧3 通过但
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
        use super::super::super::classifier::cand_predicate::{div_cand, DivCandInput, ContextMove};
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
        let mut leg_gap_hist = [0usize; 16]; // cond3 到达域的 target_idx−j（s_prev 与 s 间距，=2 ⟹ 单条反向腿=codex 常见结构 s_prev==m1）
        let mut base_conf_false = 0usize;   // confirm_side(δ) 假（δ 与 bits 侧不符 ⟹ base 拒，非互斥）

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
                for p in &ls.bsp {
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
                                moves: Vec::new(), centers: Vec::new(),
                                bsp: if l2 == lvl { vec![p.clone()] } else { Vec::new() },
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
                                let ctx: Vec<ContextMove> = knode.sub_moves.iter()
                                    .map(|m| ContextMove::from_rmove(&m.rmove, m.start_index, m.end_index))
                                    .collect();
                                if let Some(tidx) = knode.sub_moves.iter().position(|m| m.end_index == src) {
                                    r_target = true;
                                    let s = ctx[tidx];
                                    let expected = match delta {
                                        Side::Long => Direction::Down,
                                        Side::Short => Direction::Up,
                                    };
                                    r_cond1 = s.direction == expected;
                                    if r_cond1 {
                                        if let Some((j, sp)) = ctx[..tidx].iter().enumerate().rev()
                                            .find(|(_, m)| m.direction == s.direction)
                                        {
                                            r_cond2 = true;
                                            r_leggap = tidx - j;
                                            r_extreme = match delta {
                                                Side::Long => s.lo < sp.lo,
                                                Side::Short => s.hi > sp.hi,
                                            };
                                            s_ext = match delta { Side::Long => s.lo, Side::Short => s.hi };
                                            sp_ext = match delta { Side::Long => sp.lo, Side::Short => sp.hi };
                                        }
                                    }
                                    r_divcand = div_cand(&DivCandInput {
                                        context: &ctx, target_idx: tidx, hist: &macd_hist, delta,
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
                        } else if cert_opt.is_none() {
                            st_base_none += 1;
                            stage = "base_none";
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
        assert_eq!(extreme_false + extreme_true_pass + extreme_true_reject, cond3_reached,
            "cond3 到达域守恒：Extreme必假({extreme_false})+Extreme真通过({extreme_true_pass})+Extreme真拒({extreme_true_reject}) 应 = cond3_reached({cond3_reached})");
        eprintln!("真封：Σ阶段={stage_sum}=n_total={n_total}；cond3_reached={cond3_reached}=必假{extreme_false}+真通过{extreme_true_pass}+真拒{extreme_true_reject}");
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
