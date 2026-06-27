//! Θ_risk 风险投影 + 结构止损 + sizing（reference-theta-v0.md:44-47，
//! 契约锚 `Origin.RiskProj`）。
//!
//! ## 范围
//!
//! - 结构止损（spec:46）：1/2买止损=对应买点 pivot low；3买止损=ZG；卖/空镜像 pivot high
//!   或 ZD。
//! - sizing（spec:47）：`qty = min(floor(ρ*NAV/(|entry-stop|+κ*cost_per_unit)),
//!   floor(w_depth*γ*NAV/entry), parent_cap)`；`qty<=0` 不交易；默认 lot=config.risk.default_lot。
//! - 风险投影唯一总仓位（Origin.RiskProj `riskproj_exists_unique`）：sizing 的三路 min 是
//!   有限网格上确定选择器的具体实例——三个上界的最小值是网格 𝒦 中 cost 字典序最小的可行
//!   格点（这里 cost = 仓位绝对值，约束 = 三个上界，min 即字典序最小可行 qty）。
//!
//! ## bit-exact：浮点约简顺序固定
//!
//! sizing 含浮点（NAV/cost/权重是 f64），整数 tick 价（entry/stop 是 Tick=i64）。bit-exact
//! 要求每个 min 项的浮点运算按**固定顺序**约简——本模块每个公式拆为确定步骤，floor 到 i64，
//! 不依赖编译器重排（Cargo.toml 已禁 fast-math）。
//!
//! ## 认识论等级
//!
//! L0（定义内蕴）：止损价选取规则 / sizing 三路 min 是给定 Θ_risk 参数后的确定函数。
//! ★诚实：ρ/β/γ/κ/止损规则全是 **Θ_risk 参数**（非缠论可导，Origin.RiskProj 核心命题：
//! 缠论结构不能推出仓位大小）。本模块证「给定这些 Θ_risk 参数后 qty 唯一确定」，不证盈利。

use super::super::config::RiskConfig;
use super::super::types::{BspBits, Center, Tick};
use super::voice::{root_sel, RootCandidates, VoiceSide};

/// 结构止损价的方向（多头止损在下方，空头止损在上方）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopSide {
    /// 多头（买点）：止损在入场价**下方**（pivot low / ZG）。
    Long,
    /// 空头（卖点）：止损在入场价**上方**（pivot high / ZD）。
    Short,
}

/// 结构止损输入（buy/sell 点的 pivot 极值 + 所在级别的最后中枢边界）。
///
/// 字段语义（reference-theta-v0.md:46）：
/// - `pivot_low`/`pivot_high`：买卖点对应的 pivot 极值价（1/2 类止损源）。
/// - `center`：该买卖点所在级别的最后中枢（3 类止损取 `zg`(买)/`zd`(卖)）。
///
/// ★诚实：pivot 价 + 中枢边界来自上游分类（classifier 的 BSP/Center），本结构作为给定值
/// 承载——止损价**选取规则**（1/2 类用 pivot，3 类用中枢边界）是 Θ_risk 设计选择（spec:46）。
#[derive(Debug, Clone, Copy)]
pub struct StopInput {
    pub pivot_low: Tick,
    pub pivot_high: Tick,
    pub center: Center,
}

/// 计算结构止损价（bit-exact，reference-theta-v0.md:46）。
///
/// 买点（`StopSide::Long`）：
/// - 1 买 / 2 买止损 = `pivot_low`（对应买点 pivot low）；
/// - 3 买止损 = `center.zg`（ZG）。
///
/// 卖点（`StopSide::Short`，镜像）：
/// - 1 卖 / 2 卖止损 = `pivot_high`（pivot high）；
/// - 3 卖止损 = `center.zd`（ZD）。
///
/// ★重合情形裁定（**spec:46 未定义，Θ_risk 设计选择**，非 spec 推论）：spec:46 只给单类
/// 止损规则（1/2 买=pivot low；3 买=ZG），**未定义**一点同时持有多类买点（BSP 非互斥，
/// BspBits 可多位为真）时的止损。本实现的设计选择 [设计选择,Θ_risk]：取**更宽**者——
/// 多头取 `min`（止损更低 = 容忍更大回撤至结构彻底失效），空头取 `max`。语义依据：止损是
/// 「结构失效价」，一点持多类买点时，任一类未失效则结构未彻底失效，故取最宽松的失效线
/// （所有持有类别中最宽止损）。**此裁定不由 spec 推出，是 strategy 层的 Θ_risk 设计选择**
/// （若 spec 后续明确重合规则与此冲突，须 change request 而非保留此选择）。
///
/// 边界条件：`bits` 无对应方向的任何买/卖点位（如 Long 但 b1/b2/b3 全 false）⟹ 返回 `None`
/// （无止损可定 = 非该方向交易点，调用方不应在此开仓）。
pub fn structural_stop(side: StopSide, bits: &BspBits, stop_in: &StopInput) -> Option<Tick> {
    match side {
        StopSide::Long => {
            // 收集所有持有的买点类别的止损价。
            let mut stop: Option<Tick> = None;
            if bits.buy1 || bits.buy2 {
                stop = Some(stop_in.pivot_low);
            }
            if bits.buy3 {
                // 3 类止损 = ZG；与已有 1/2 类止损取更宽（更低）者。
                stop = Some(match stop {
                    Some(s) => s.min(stop_in.center.zg),
                    None => stop_in.center.zg,
                });
            }
            stop
        }
        StopSide::Short => {
            let mut stop: Option<Tick> = None;
            if bits.sell1 || bits.sell2 {
                stop = Some(stop_in.pivot_high);
            }
            if bits.sell3 {
                stop = Some(match stop {
                    Some(s) => s.max(stop_in.center.zd),
                    None => stop_in.center.zd,
                });
            }
            stop
        }
    }
}

/// sizing 输入（reference-theta-v0.md:47）。
///
/// 字段语义：
/// - `nav`：账户净值 NAV（账户层状态，Θ 之外的运行时输入）。单位：**美元**。
/// - `entry`：入场价（整数 tick；>0 必要——分母）。
/// - `stop`：结构止损价（`structural_stop` 产出，整数 tick）。
/// - `tick_size`：价格最小变动单位（`config.tick.tick_size`）。用于把整数 tick 转换为
///   美元价格（`entry_tick * tick_size` = 美元价格），与 NAV 量纲对齐。
///   sizing 公式 `floor(w·γ·NAV/entry)` 中的 `entry` 是**美元价格**而非 tick 整数，
///   公式 `floor(ρ·NAV/|entry-stop|)` 中的 `|entry-stop|` 是**美元亏损**（tick差×tick_size）。
/// - `cost_per_unit`：每单位成本（commission+slippage+tax 折算，账户/Θ_exec 参数）。
/// - `w_depth`：声部深度资金权重（`voice::depth_weight`，spec:42）。
/// - `parent_cap`：父声部仓位上限（父子比 β 投影，spec:45；β 在调用方按 `parent_qty*β` 算好）。
#[derive(Debug, Clone, Copy)]
pub struct SizingInput {
    pub nav: f64,
    pub entry: Tick,
    pub stop: Tick,
    /// 价格最小变动单位（美元/tick）。用于把 tick 整数转换为美元价格与 NAV 量纲对齐。
    pub tick_size: f64,
    pub cost_per_unit: f64,
    pub w_depth: f64,
    pub parent_cap: i64,
}

/// sizing：唯一目标手数 qty（bit-exact 对齐 reference-theta-v0.md:47 + Origin.RiskProj）。
///
/// `qty = min(floor(ρ*NAV/(|entry-stop|*tick_size+κ*cost_per_unit)),
///            floor(w_depth*γ*NAV/(entry*tick_size)),
///            parent_cap)`
///
/// ★量纲修正（工程层 sizing 门控根因，L2 OKLO 诊断 2026-06-27）：
/// spec:47 的公式 `ρ·NAV/|entry-stop|` 和 `w·γ·NAV/entry` 中的分母是**美元**量纲——
/// `|entry-stop|` 是每手最大亏损（美元），`entry` 是入场价（美元）。Rust 实装用整数
/// tick，必须乘以 `tick_size`（美元/tick）才能与 NAV（美元）量纲对齐。
/// 量纲不对齐时（如 tick_size=1e-8，OKLO 价格~$10 → entry_tick~1e9 >> NAV=1e6），
/// 项2分母 >> 分子，floor → 0，令 sizing 永不开仓（工程层门控，非 Θ 经验否证）。
///
/// 三路上界（风险投影的三个约束，Origin.RiskProj 网格 𝒦 的可行格点上界）：
/// 1. **风险预算**：单声部风险 ρ·NAV 除以每手最大亏损（`|entry-stop|*tick_size + κ·cost`）——
///    保证单声部亏损 ≤ ρ·NAV（spec:45 单声部风险）。
/// 2. **名义上限**：`w_depth·γ·NAV / (entry*tick_size)`——深度资金帽 × 总名义上限 γ 除以
///    入场美元价（spec:42 资金帽 + spec:45 总名义上限 γ）。
/// 3. **父子约束**：`parent_cap`（父子仓位比 β 投影的上界，spec:45）。
///
/// `qty <= 0 ⟹ 不交易`（返回 0，spec:47）。三路 min 是 Origin.RiskProj `riskproj_exists_unique`
/// 的具体实例：三约束下的可行 qty 集合是 {0,1,…,min三上界} 的有限网格，min 是字典序最小
/// 可行格点上界 = 唯一总仓位（确定选择，无平局）。
///
/// ## bit-exact 浮点约简顺序（spec:47 未钉死括号，本实装固定为下列顺序 [设计选择]）
///
/// 本实装把约简顺序**固定**为（bit-exact 的设计选择，Cargo.toml 已禁浮点重排）：
/// - 项 1 分母：先 `|entry-stop| as f64 * tick_size`，再 `+ κ·cost`；分子 `ρ·NAV`；除后 `floor`。
/// - 项 2：`((w_depth·γ)·NAV) / (entry as f64 * tick_size)`（左结合），后 `floor`。
///
/// 边界条件：
/// - `entry <= 0` ⟹ 项 2 分母非正，返回 0（不交易；入场价须为正 tick）。
/// - `tick_size <= 0` ⟹ 量纲无效，返回 0（保护边界）。
/// - 项 1 分母 `|entry-stop|*tick_size + κ·cost = 0`（entry=stop 且 cost=0）⟹ 风险预算无界，
///   项 1 不约束（取 i64::MAX），由项 2/3 约束。
/// - `default_lot`（spec:47）：qty>0 时按 lot 取整——qty 向下取整到 lot 的整数倍
///   （`default_lot=1` 时无影响）。
pub fn size_position(input: &SizingInput, config: &RiskConfig) -> i64 {
    if input.entry <= 0 {
        return 0;
    }
    if input.tick_size <= 0.0 {
        return 0; // 量纲无效（tick_size 必须正数）
    }

    // ── 项 1：风险预算上界 floor(ρ*NAV / (|entry-stop|*tick_size + κ*cost_per_unit)) ──
    // |entry-stop|*tick_size = 每手最大亏损（美元），与 NAV（美元）量纲一致。
    let risk_dist_ticks = (input.entry - input.stop).abs() as f64; // tick 差，整数 → f64
    let risk_dist_usd = risk_dist_ticks * input.tick_size; // 美元亏损（量纲对齐）
    let denom1 = risk_dist_usd + config.kappa * input.cost_per_unit; // 先乘 κ·cost 后加
    let bound1: i64 = if denom1 <= 0.0 {
        // 分母非正（entry=stop 且 cost=0）：风险预算不约束。
        i64::MAX
    } else {
        let numer1 = config.rho * input.nav; // ρ·NAV（美元）
        floor_nonneg(numer1 / denom1)
    };

    // ── 项 2：名义上限 floor(w_depth*γ*NAV / (entry*tick_size)) ──
    // entry*tick_size = 入场美元价，与 NAV（美元）量纲一致。
    let entry_usd = input.entry as f64 * input.tick_size; // 美元价格（量纲对齐）
    let numer2 = input.w_depth * config.gamma * input.nav; // (w_depth·γ)·NAV，左结合
    let bound2: i64 = floor_nonneg(numer2 / entry_usd);

    // ── 项 3：父子约束 parent_cap ──
    let bound3 = input.parent_cap;

    // ── 三路 min ──
    let raw = bound1.min(bound2).min(bound3);
    if raw <= 0 {
        return 0; // qty <= 0 不交易（spec:47）
    }

    // default_lot 取整（spec:47）：向下取整到 lot 的整数倍。
    let lot = config.default_lot.max(1) as i64;
    (raw / lot) * lot
}

/// 把非负 f64 向下取整为 i64（bit-exact 辅助）。
///
/// 负值/NaN/无穷返回 0（不交易兜底）；正值 `floor` 后转 i64。超出 i64 范围的极大值
/// 钳到 `i64::MAX`（不溢出）。
///
/// ★精度细节（安全性见证）：`i64::MAX as f64 == 2^63`（= 9223372036854775808.0），**严格大于**
/// `i64::MAX`（= 2^63-1）——f64 只有 53 位尾数，无法精确表示 2^63-1。因此 `f >= i64::MAX as f64`
/// 的比较阈值实为 2^63，覆盖了所有会令 `f as i64` 越界/UB 的 f 值（Rust 中 `f as i64` 对超出
/// i64 范围的 f 在 release 下是饱和转换，但显式钳制更明确）。所有 `f ∈ [2^63-1, 2^63)` 被安全
/// 截断到 `i64::MAX`——非 off-by-one，是 f64 表示精度的必然结果。
fn floor_nonneg(x: f64) -> i64 {
    if !x.is_finite() || x < 0.0 {
        return 0;
    }
    let f = x.floor();
    if f >= i64::MAX as f64 {
        i64::MAX
    } else {
        f as i64
    }
}

/// 父子仓位上限投影（reference-theta-v0.md:45：父子仓位比 β）。
///
/// 子声部上限 = `floor(parent_qty · β)`——子声部仓位不超过父声部仓位的 β 倍（spec:45
/// β=0.5）。根声部无父，上限 = 全局名义上限（由 sizing 项 2 的 γ 约束，此处返回 i64::MAX
/// 表示父子约束不生效，留项 2 约束）。
///
/// 边界条件：`parent_qty <= 0`（父空仓）⟹ 子上限 = 0（父无仓则子不开，Origin.VoiceTree `Permit`
/// 要求 `q_p > 0`）。
pub fn parent_cap(parent_qty: i64, config: &RiskConfig) -> i64 {
    if parent_qty <= 0 {
        return 0;
    }
    floor_nonneg(parent_qty as f64 * config.beta)
}

/// 根声部的父子上限（无父）：父子约束不生效。
///
/// 根声部没有父，故 `parent_cap` 项不约束——返回 `i64::MAX`（三路 min 中此项让位给
/// 风险预算/名义上限项）。
pub fn root_parent_cap() -> i64 {
    i64::MAX
}

// ──────────────────────────────────────────────────────────────────────────
//  §11 风险模式 μ_t + GlobalRiskClose 全局风控平仓
//  （strict §11「杠杆、保证金和风险模式」line 361-375 / FULL 十三 line 1012-1057
//   + strict §9 根方向递归 line 280 GlobalRiskClose_t / FULL 十 line 643）
// ──────────────────────────────────────────────────────────────────────────

/// 五态风险模式 μ_t（bit-exact 对齐 strict §11 line 364 / FULL 十三 line 1016-1022，
/// 契约锚 Lean `risk_mode_complete_unique`：Σ𝟙=1）。
///
/// 按优先级穷尽互斥（strict §11 line 369-374 / FULL 十三 line 1026-1046）：
/// M0 > M1 > M2 > M3 > M4，每个 M_i 含 `¬M0 ∧ … ∧ ¬M_{i-1}` 前缀，故恰一态成立。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskMode {
    /// M0：`E_t ≤ 0`——权益耗尽（破产）。
    Insolvent,
    /// M1：`¬M0 ∧ (LiqFlag_t ∨ E_t < MM_t)`——触发强平（场所强平标志或权益跌破维持保证金）。
    Liquidation,
    /// M2：`¬M0 ∧ ¬M1 ∧ E_t < MM_t + B1`——主动去杠杆区（权益在维持线 + 一档缓冲内）。
    Deleverage,
    /// M3：`¬M0 ∧ ¬M1 ∧ ¬M2 ∧ E_t < MM_t + B2`——只许平仓区（二档缓冲内）。
    CloseOnly,
    /// M4：以上皆否——正常交易区。
    Normal,
}

/// 风险模式输入（strict §11 / FULL 十三：账户层运行时量，非缠论可导）。
///
/// 字段语义（strict §11 line 370-373）：
/// - `equity` = E_t：账户权益（美元）。M0 判 `E_t ≤ 0`。
/// - `maint_margin` = MM_t(q_t)：当前持仓的维持保证金（美元）。M1 判 `E_t < MM_t`。
/// - `buffer1` = B1_t、`buffer2` = B2_t：去杠杆/只平仓的两档缓冲（美元，`0 < B1 < B2`，
///   strict §11 / FULL 十三 line 1010）。
/// - `liq_flag` = LiqFlag_t：交易场所强平标志（外部事件 ν_t 的一部分，strict §1 line 47）。
///
/// ★诚实（formalization-validity-domain）：E_t/MM_t/B1/B2/LiqFlag 全是**账户/场所层运行时输入**
/// （Θ 之外的外部事件 e_{t+1}/ν_t，strict §1:47 + §9:264）——**不由缠论或 Θ_risk 推导**。本结构
/// 作为给定 Z 承载；风险模式划分在给定它们后是 L0 逻辑必然（穷尽互斥）。
#[derive(Debug, Clone, Copy)]
pub struct RiskModeInput {
    pub equity: f64,
    pub maint_margin: f64,
    pub buffer1: f64,
    pub buffer2: f64,
    pub liq_flag: bool,
}

/// 计算风险模式 μ_t（bit-exact 对齐 strict §11 line 369-374 / FULL 十三 line 1026-1057）。
///
/// 按优先级 M0 > M1 > M2 > M3 > M4 短路判定——`if/else if` 链天然实现 `¬M0 ∧ … ∧ ¬M_{i-1}`
/// 前缀（先判 M0，否则才判 M1，…）。穷尽（最后 else = M4 = Normal）+ 互斥（短路保证恰一态），
/// 对齐 Lean `risk_mode_complete_unique`（Σ𝟙=1）。
///
/// 边界条件：
/// - `E_t ≤ 0` ⟹ Insolvent（不论 LiqFlag/MM——M0 最高优先级吸收）。
/// - 缓冲 `B1 < B2` 是 strict 前提；若配置 `B1 ≥ B2` 则 M2 吸收 M3 的部分区间（CloseOnly
///   退化），属配置违规而非本函数 bug——本函数忠实按给定 B1/B2 划分。
pub fn risk_mode(input: &RiskModeInput) -> RiskMode {
    if input.equity <= 0.0 {
        RiskMode::Insolvent // M0：E_t ≤ 0
    } else if input.liq_flag || input.equity < input.maint_margin {
        RiskMode::Liquidation // M1：LiqFlag ∨ E_t < MM_t
    } else if input.equity < input.maint_margin + input.buffer1 {
        RiskMode::Deleverage // M2：E_t < MM_t + B1
    } else if input.equity < input.maint_margin + input.buffer2 {
        RiskMode::CloseOnly // M3：E_t < MM_t + B2
    } else {
        RiskMode::Normal // M4
    }
}

/// **GlobalRiskClose_t：全局风控平仓触发判定**（strict §9 line 280 / FULL 十 line 643，
/// 契约锚 §16 优先级 P1「破产或强平」line 535）。
///
/// 触发条件 = 风险模式落入**破产或强平**两态：`μ_t ∈ {Insolvent, Liquidation}`。这是 strict
/// §16 动作意图优先级的**最高级**（P1，line 535-536「破产或强平 ≻ 主动去杠杆 ≻ …」），也是
/// strict §9 / FULL 十根方向递归的**首触发**（`σ̃_{r,t+1}=0` 的第一个 case，line 280/643）。
///
/// **平仓语义**（strict §9 line 287 / FULL 十 line 656「先平根仓，下一决策周期才允许反向重新
/// 建立」）：GlobalRiskClose 成立 ⟹ 根方向强制归零（σ̃_{r,t+1}=0）⟹ 全局平根仓。由于级联关闭
/// （Origin.VoiceTree `cascade_close`：父关 ⟹ 后代全关），根仓平 ⟹ 所有子声部仓位级联平 ⟹
/// **全局平仓**。这关死「权益耗尽/被强平时仍持仓」。
///
/// ★区分（Deleverage/CloseOnly 不触发全局平仓）：μ_t=Deleverage(M2)/CloseOnly(M3) 是 §12
/// 可行集的 `G(q') ≤ G(q_t)`（不增总名义）约束（strict §12 line 403），**不是全局平仓**——
/// 它们限制开新仓/增仓，但不强制平掉现有仓。只有 Insolvent/Liquidation 触发 GlobalRiskClose。
///
/// ★认识论 L0（定义内蕴）：给定 μ_t 后，GlobalRiskClose 是「μ_t ∈ {Insolvent, Liquidation}」的
/// 布尔判定，对齐 strict §16 P1 谓词。μ_t 本身依赖账户层运行时输入（[`RiskModeInput`]，非缠论
/// 可导），但「哪些 μ_t 触发全局平仓」是 strict §9/§16 的定义层规则。
///
/// 边界条件：若 Θ 后续把 GlobalRiskClose 扩到「执行异常」（strict §16 P3「未完成订单和执行
/// 异常处理」）等更低优先级触发，则须 change request——Θ v0 内 GlobalRiskClose 仅 = {Insolvent,
/// Liquidation}（P1），不含 P2（主动去杠杆）及以下。
pub fn global_risk_close(mode: RiskMode) -> bool {
    matches!(mode, RiskMode::Insolvent | RiskMode::Liquidation)
}

/// **根方向递归 σ̃_{r,t+1}**（strict §9 line 278-284 / FULL 十 line 638-654，bit-exact）。
///
/// 全定义根方向状态转移——按 strict §9 的 4 路 case（优先级从上到下短路）：
/// ```text
/// σ̃_{r,t+1} =
///   0,                              GlobalRiskClose_t                    （首触发：全局平根仓）
///   0,                              σ_{r,t} ≠ 0 ∧ χ^{-σ_r}_{r,t} = 1     （反向信号：先平后建）
///   RootSel(χ⁺,χ⁻),                 σ_{r,t} = 0                          （空仓：按候选选向）
///   σ_{r,t},                        其他                                 （持仓延续）
/// ```
///
/// 参数：
/// - `mode`：当前风险模式 μ_t（[`risk_mode`] 产出）——决定 GlobalRiskClose_t。
/// - `current`：当前根方向 σ_{r,t}（[`VoiceSide`]：Long/Short/Flat）。
/// - `cands`：根触发候选 (χ⁺,χ⁻)（[`voice::RootCandidates`]）——供 RootSel 与反向信号判定。
///
/// **第二 case「反向信号先平后建」**（strict §9 line 281/287 / FULL 十 line 644-647/656）：持有
/// 根仓（σ_{r,t}≠0）且出现**反向**触发（做多根遇 χ⁻=1，或做空根遇 χ⁺=1）⟹ 先平（σ̃=0），不在
/// 同一时刻假设已反手（避免未成交就反向）。下一决策周期 σ_{r,t}=0 时才经第三 case 重新建反向仓。
///
/// ★认识论 L0：4 路 case 是 strict §9 根方向递归的逐 case 镜像，给定 (μ_t, σ_{r,t}, χ±) 后唯一
/// 确定 σ̃_{r,t+1}。返回的 Flat = σ̃=0（平根仓 → 级联全平）。
///
/// 边界条件：case 优先级不可交换——GlobalRiskClose 必须最高（强平先于一切）；反向先平先于持仓
/// 延续（避免反手假设）。若交换则破坏 strict §9 的 case 顺序语义。
pub fn root_dir_next(mode: RiskMode, current: VoiceSide, cands: RootCandidates) -> VoiceSide {
    // case 1：GlobalRiskClose_t ⟹ σ̃ = 0（全局平根仓，最高优先级）。
    if global_risk_close(mode) {
        return VoiceSide::Flat;
    }
    // case 2：持仓 + 反向信号 ⟹ 先平（σ̃ = 0），不假设已反手。
    let reverse_signal = match current {
        VoiceSide::Long => cands.short_trigger,  // 做多根遇卖侧触发 χ⁻=1
        VoiceSide::Short => cands.long_trigger,   // 做空根遇买侧触发 χ⁺=1
        VoiceSide::Flat => false,                 // 空仓无「反向」可言
    };
    if current != VoiceSide::Flat && reverse_signal {
        return VoiceSide::Flat;
    }
    // case 3：空仓 ⟹ 按 RootSel 选向（含 (1,1) 镜像反对称消歧）。
    if current == VoiceSide::Flat {
        return root_sel(cands);
    }
    // case 4：其他（持仓且无反向信号）⟹ 延续当前方向。
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_center(zd: Tick, zg: Tick) -> Center {
        Center { zd, zg, dd: zd - 10, gg: zg + 10, start_index: 0, end_index: 9 }
    }

    /// 结构止损（spec:46）：1/2 买 = pivot low；3 买 = ZG；卖镜像。
    #[test]
    fn structural_stop_class_rules() {
        let c = mk_center(100, 200);
        let si = StopInput { pivot_low: 90, pivot_high: 210, center: c };
        // 1 买：pivot low。
        let b1 = BspBits { buy1: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Long, &b1, &si), Some(90));
        // 2 买：pivot low。
        let b2 = BspBits { buy2: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Long, &b2, &si), Some(90));
        // 3 买：ZG。
        let b3 = BspBits { buy3: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Long, &b3, &si), Some(200));
        // 1 卖：pivot high。
        let s1 = BspBits { sell1: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Short, &s1, &si), Some(210));
        // 3 卖：ZD。
        let s3 = BspBits { sell3: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Short, &s3, &si), Some(100));
    }

    /// 2B/3B 重合（BSP 非互斥）：止损取更宽（多头更低）者。pivot_low=90 vs ZG=200 → 90。
    #[test]
    fn structural_stop_2b3b_coincide_takes_wider() {
        let c = mk_center(100, 200);
        let si = StopInput { pivot_low: 90, pivot_high: 210, center: c };
        let b23 = BspBits { buy2: true, buy3: true, ..Default::default() };
        // 多头取更低（更宽）：min(90, 200) = 90。
        assert_eq!(structural_stop(StopSide::Long, &b23, &si), Some(90));
        // 空头 2S/3S 重合：取更高（更宽）：max(pivot_high=210, zd=100) = 210。
        let s23 = BspBits { sell2: true, sell3: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Short, &s23, &si), Some(210));
    }

    /// 无对应方向买卖点 ⟹ None（非该方向交易点）。
    #[test]
    fn structural_stop_no_bsp_yields_none() {
        let c = mk_center(100, 200);
        let si = StopInput { pivot_low: 90, pivot_high: 210, center: c };
        let empty = BspBits::default();
        assert_eq!(structural_stop(StopSide::Long, &empty, &si), None);
        // Long 方向但只有卖点位 ⟹ None。
        let only_sell = BspBits { sell1: true, ..Default::default() };
        assert_eq!(structural_stop(StopSide::Long, &only_sell, &si), None);
    }

    /// sizing 三路 min（spec:47）：风险预算项约束最紧的情形。
    #[test]
    fn sizing_risk_budget_binds() {
        let cfg = RiskConfig::default(); // ρ=0.005, β=0.5, γ=1.0, κ=2.0, lot=1
        // NAV=1_000_000, entry=100 tick（tick_size=1.0 ⟹ entry_usd=100），stop=90（|d_usd|=10），cost=0。
        // 项1 = floor(0.005*1e6 / (10*1.0 + 2*0)) = floor(5000/10) = 500。
        // 项2 = floor(0.6*1.0*1e6 / (100*1.0)) = floor(6000) = 6000。
        // 项3 = MAX。 min = 500。
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
            tick_size: 1.0, // 测试用 tick_size=1 ⟹ tick=美元，期望值不变
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: i64::MAX,
        };
        assert_eq!(size_position(&inp, &cfg), 500);
    }

    /// sizing：名义上限项约束最紧。
    #[test]
    fn sizing_notional_cap_binds() {
        let cfg = RiskConfig::default();
        // entry=100, stop=99 (|d_usd|=1*1.0=1) ⟹ 项1 = floor(5000/1)=5000；
        // w_depth=0.1: 项2 = floor(0.1*1e6/(100*1.0))=1000；项1=5000 ⟹ min=1000。
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 99,
            tick_size: 1.0,
            cost_per_unit: 0.0,
            w_depth: 0.1,
            parent_cap: i64::MAX,
        };
        assert_eq!(size_position(&inp, &cfg), 1000);
    }

    /// sizing：parent_cap 约束最紧。
    #[test]
    fn sizing_parent_cap_binds() {
        let cfg = RiskConfig::default();
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
            tick_size: 1.0,
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: 50, // 比项1=500、项2=6000 都小
        };
        assert_eq!(size_position(&inp, &cfg), 50);
    }

    /// sizing：qty<=0 不交易（parent_cap=0 = 父空仓）。
    #[test]
    fn sizing_zero_qty_no_trade() {
        let cfg = RiskConfig::default();
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
            tick_size: 1.0,
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: 0,
        };
        assert_eq!(size_position(&inp, &cfg), 0);
    }

    /// sizing：entry<=0 不交易（无效入场价）。
    #[test]
    fn sizing_nonpositive_entry_no_trade() {
        let cfg = RiskConfig::default();
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 0,
            stop: -10,
            tick_size: 1.0,
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: i64::MAX,
        };
        assert_eq!(size_position(&inp, &cfg), 0);
    }

    /// 父子上限投影（spec:45 β=0.5）：floor(parent_qty·β)。
    #[test]
    fn parent_cap_beta_projection() {
        let cfg = RiskConfig::default(); // β=0.5
        assert_eq!(parent_cap(100, &cfg), 50);
        assert_eq!(parent_cap(7, &cfg), 3); // floor(7*0.5)=floor(3.5)=3
        assert_eq!(parent_cap(0, &cfg), 0); // 父空仓 ⟹ 子上限 0
        assert_eq!(parent_cap(-5, &cfg), 0);
    }

    fn mk_risk_input(equity: f64, mm: f64, liq: bool) -> RiskModeInput {
        // 缓冲 B1=100, B2=300（0<B1<B2，strict §11 前提）。
        RiskModeInput { equity, maint_margin: mm, buffer1: 100.0, buffer2: 300.0, liq_flag: liq }
    }

    /// 风险模式五态穷尽互斥（strict §11 / FULL 十三，Lean `risk_mode_complete_unique`）：
    /// 逐区间验证 M0>M1>M2>M3>M4 优先级。
    #[test]
    fn risk_mode_five_states_priority() {
        // M0：E_t ≤ 0 ⟹ Insolvent（不论 LiqFlag/MM）。
        assert_eq!(risk_mode(&mk_risk_input(0.0, 500.0, false)), RiskMode::Insolvent);
        assert_eq!(risk_mode(&mk_risk_input(-10.0, 500.0, true)), RiskMode::Insolvent);
        // M1：E_t>0 ∧ (LiqFlag ∨ E_t<MM) ⟹ Liquidation。MM=500，E=400<500。
        assert_eq!(risk_mode(&mk_risk_input(400.0, 500.0, false)), RiskMode::Liquidation);
        // M1 经 LiqFlag：E=1000>MM+B2，但 LiqFlag=true ⟹ 仍 Liquidation（强平标志优先）。
        assert_eq!(risk_mode(&mk_risk_input(1000.0, 500.0, true)), RiskMode::Liquidation);
        // M2：MM ≤ E < MM+B1 ⟹ Deleverage。MM=500，B1=100，E=550 ∈ [500,600)。
        assert_eq!(risk_mode(&mk_risk_input(550.0, 500.0, false)), RiskMode::Deleverage);
        // M3：MM+B1 ≤ E < MM+B2 ⟹ CloseOnly。E=700 ∈ [600,800)。
        assert_eq!(risk_mode(&mk_risk_input(700.0, 500.0, false)), RiskMode::CloseOnly);
        // M4：E ≥ MM+B2 ⟹ Normal。E=900 ≥ 800。
        assert_eq!(risk_mode(&mk_risk_input(900.0, 500.0, false)), RiskMode::Normal);
    }

    /// 风险模式恰一态（穷尽互斥见证）：扫一系列权益，每个恰好返回一个 RiskMode（match 静态保证）。
    #[test]
    fn risk_mode_exhaustive_exclusive() {
        for e in [-100.0, 0.0, 1.0, 400.0, 500.0, 550.0, 600.0, 700.0, 800.0, 900.0] {
            let m = risk_mode(&mk_risk_input(e, 500.0, false));
            // 恰一态：返回值是 5 态之一（Rust enum 穷尽 + 函数全定义）。
            assert!(matches!(
                m,
                RiskMode::Insolvent
                    | RiskMode::Liquidation
                    | RiskMode::Deleverage
                    | RiskMode::CloseOnly
                    | RiskMode::Normal
            ));
        }
    }

    /// ★GlobalRiskClose 触发判定（strict §9/§16 P1）：仅 Insolvent/Liquidation 触发全局平仓，
    /// Deleverage/CloseOnly/Normal **不**触发（它们限增仓不强制平仓）。
    #[test]
    fn global_risk_close_only_insolvent_or_liquidation() {
        assert!(global_risk_close(RiskMode::Insolvent)); // M0 → P1 触发
        assert!(global_risk_close(RiskMode::Liquidation)); // M1 → P1 触发
        assert!(!global_risk_close(RiskMode::Deleverage)); // M2 不触发全局平仓
        assert!(!global_risk_close(RiskMode::CloseOnly)); // M3 不触发
        assert!(!global_risk_close(RiskMode::Normal)); // M4 不触发
    }

    /// ★根方向递归 σ̃_{r,t+1}（strict §9 line 278-284 / FULL 十）：4 路 case 逐一验证。
    #[test]
    fn root_dir_next_four_cases() {
        let long_c = RootCandidates { long_trigger: true, short_trigger: false };
        let short_c = RootCandidates { long_trigger: false, short_trigger: true };
        let none_c = RootCandidates { long_trigger: false, short_trigger: false };
        let both_c = RootCandidates { long_trigger: true, short_trigger: true };

        // case 1：GlobalRiskClose（Liquidation）⟹ σ̃=0，覆盖一切（即便持多仓 + 同向信号）。
        assert_eq!(root_dir_next(RiskMode::Liquidation, VoiceSide::Long, long_c), VoiceSide::Flat);
        assert_eq!(root_dir_next(RiskMode::Insolvent, VoiceSide::Short, short_c), VoiceSide::Flat);

        // case 2：持多仓（Long）+ 反向触发（χ⁻=1）⟹ 先平（σ̃=0），不假设反手。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Long, short_c), VoiceSide::Flat);
        // 持空仓（Short）+ 反向触发（χ⁺=1）⟹ 先平。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Short, long_c), VoiceSide::Flat);

        // case 3：空仓（Flat）⟹ 按 RootSel 选向。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Flat, long_c), VoiceSide::Long);
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Flat, short_c), VoiceSide::Short);
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Flat, none_c), VoiceSide::Flat);
        // 空仓遇双触发 (1,1) ⟹ RootSel 镜像反对称消歧为 Flat（不开根仓）。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Flat, both_c), VoiceSide::Flat);

        // case 4：持多仓 + 同向/无反向信号 ⟹ 延续 Long。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Long, long_c), VoiceSide::Long);
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Long, none_c), VoiceSide::Long);
        // 持空仓 + 同向（χ⁻=1）⟹ 延续 Short。
        assert_eq!(root_dir_next(RiskMode::Normal, VoiceSide::Short, short_c), VoiceSide::Short);
    }

    /// 根方向递归 case 优先级：GlobalRiskClose 高于反向先平高于持仓延续。
    #[test]
    fn root_dir_next_case_priority() {
        let short_c = RootCandidates { long_trigger: false, short_trigger: true };
        // 持多仓 + 反向触发 + 同时 Liquidation ⟹ case 1（GlobalRiskClose）先于 case 2，均得 Flat
        // （此例两 case 都给 Flat，但 case 1 必须优先——Insolvent 时即便无反向也平）。
        assert_eq!(root_dir_next(RiskMode::Liquidation, VoiceSide::Long, short_c), VoiceSide::Flat);
        // Deleverage（非 GlobalRiskClose）+ 持多仓 + 无反向 ⟹ 延续（去杠杆不强制平根仓）。
        let none_c = RootCandidates { long_trigger: false, short_trigger: false };
        assert_eq!(root_dir_next(RiskMode::Deleverage, VoiceSide::Long, none_c), VoiceSide::Long);
    }

    /// default_lot 取整（spec:47）：qty 向下取整到 lot 倍数。
    #[test]
    fn sizing_lot_rounding() {
        let mut cfg = RiskConfig::default();
        cfg.default_lot = 100; // lot=100
        // 项1=500 ⟹ floor(500/100)*100 = 500。
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
            tick_size: 1.0,
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: i64::MAX,
        };
        assert_eq!(size_position(&inp, &cfg), 500);
        // parent_cap=250, lot=100 ⟹ min(500,...,250)=250 ⟹ floor(250/100)*100=200。
        let inp2 = SizingInput { parent_cap: 250, ..inp };
        assert_eq!(size_position(&inp2, &cfg), 200);
    }
}
