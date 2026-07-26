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
use super::super::types::{BspBits, Center, Tick, Timestamp};
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
///   **方案A协变**：`nav` 在 runner 层即是协变资本单位 `U_ℓ`（按级别缩放注入）；
///   sizing 公式 `w·γ·NAV/entry` 中 `γ` 是无量纲 Θ 参数，`NAV/entry` 给出名义手数上限
///   （d_j=1 名义上限，与 coverage.rs `feasible_net_cap` γ̄ 对齐）。
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
    /// 单笔风险预算 ρ_{ℓ,δ,r}（PDF《全互斥定义策略》§3：ρ 是**状态函数**，按 (level ℓ, delta δ,
    /// root r) 取值，多空不强行镜像）。调用方（`build_open_order`）按当前决策的 (level, side) 从
    /// [`super::super::config::SizingProfile`] 解析；无 override ⟹ 退化为 `RiskConfig.rho`（标量），
    /// 此时 bit-exact 等于改动前。
    pub rho: f64,
    /// 名义上限 Γ_{ℓ,δ,r}（同上，PDF §3）。无 override ⟹ 退化为 `RiskConfig.gamma`。
    pub gamma: f64,
    /// 缺口缓冲 GapBuffer(z,a)（PDF §2：`D = M·|P−S| + Cost + GapBuffer`，美元）。隔夜跳空/
    /// 滑点尾部的额外止损距离，加进项 1 分母 D ⟹ 风险归一化更保守。无 override ⟹ 0（bit-exact）。
    pub gap_buffer: f64,
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
///    **方案A d_j=1**：`γ` 是无量纲 Θ 参数，`NAV/entry_usd` 给出名义手数；约束是
///    d_j=1 名义约束（对齐 CovariantCapital.lean `dNominal=1` + coverage.rs `feasible_net_cap`）。
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
    // D = M·|P−S| + Cost + GapBuffer（PDF §2）：M·|P−S| = risk_dist_usd，κ·cost = Cost 项，
    // gap_buffer = GapBuffer 项。约简顺序固定：先 risk_dist_usd，加 κ·cost，再加 gap_buffer。
    let denom1 = risk_dist_usd + config.kappa * input.cost_per_unit + input.gap_buffer;
    let bound1: i64 = if denom1 <= 0.0 {
        // 分母非正（entry=stop 且 cost=0 且 gap=0）：风险预算不约束。
        i64::MAX
    } else {
        let numer1 = input.rho * input.nav; // ρ_{ℓ,δ,r}·NAV（美元）
        floor_nonneg(numer1 / denom1)
    };

    // ── 项 2：名义上限 floor(w_depth*Γ*NAV / (entry*tick_size)) ──
    // entry*tick_size = 入场美元价，与 NAV（美元）量纲一致。
    let entry_usd = input.entry as f64 * input.tick_size; // 美元价格（量纲对齐）
    let numer2 = input.w_depth * input.gamma * input.nav; // (w_depth·Γ_{ℓ,δ,r})·NAV，左结合
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
///
/// `Hash/Ord`（G3 #138）：本枚举作为 [`MuClass`](crate::theta_v0::backtest::mu_estimator::MuClass)
/// 第 13 维 `risk_mode` 的分量（§6 RiskMode+MarginState），需与 MuClass 的 derive 全家桶同级。
/// derive `Ord` 取声明序 Insolvent<…<Normal（= 严重度降序），仅供 BTreeMap 有序报告，无业务比较语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
///
/// **方案A协变兼容**（2026-06-28 CovariantCapital.lean §5 ReadyReturn/StageOne/StageThree 对齐）：
/// `risk_mode` 的 `E_t < MM_t` 等比较等价于归一化形式 `Ē_t < MM̄_t`（两侧同除 `U_ℓ > 0` 对消）——
/// 比较保序，函数体不变；runner 注入绝对值（美元），策略层语义对应无量纲比例（d_j=0 杠杆/d_j=1 名义）。
/// 杠杆约束 `leverage_ok`（`G_t/E_t ≤ L̄^G`）是比值（d_j=0，天然无量纲，完全方案A兼容）。
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

// ──────────────────────────────────────────────────────────────────────────
//  §13 杠杆系统：有符号名义头寸 n_v + 毛/净敞口 G_t/N_t + 毛/净杠杆 L^G/L^N
//  （strict §11「杠杆、保证金和风险模式」line 341-359 / FULL 十三 line 975-1006，
//   契约锚 Lean `Origin.LeverageCapital`：net_le_gross / gross_cap_implies_net_cap）
// ──────────────────────────────────────────────────────────────────────────

/// 单声部名义头寸（strict §11 line 343 / FULL 十三 line 977-981：n_v = σ_v·M_v·P_v·q_v）。
///
/// 字段语义：
/// - `side`：声部方向 σ_v（[`VoiceSide`]：Long/Short/Flat；Flat ⟹ 名义 0）。
/// - `notional_mag`：名义大小 |n_v| = M_v·P_v·q_v（≥0，调用方按合约乘数×价格×手数算好，
///   单位美元；M_v 是 Θ_leverage 合约乘数参数，P_v 是价格，q_v 是手数）。
///
/// ★诚实（formalization-validity-domain）：M_v（合约乘数）是 Θ_leverage 参数（非缠论可导）。
/// 本结构承载折算后的名义大小，证聚合关系，不重算乘积（对齐 Lean `VoicePosition`）。
#[derive(Debug, Clone, Copy)]
pub struct VoiceNotional {
    pub side: VoiceSide,
    pub notional_mag: i64,
}

impl VoiceNotional {
    /// 有符号名义头寸 n_v = σ_v·|n_v|（strict §11 line 343）：Long → +|n_v|，Short → -|n_v|，
    /// Flat → 0（空仓声部无名义贡献，对齐 [`VoiceSide::Flat`] 的 q_v=0 语义）。
    ///
    /// ★对齐 Lean `Origin.LeverageCapital.VoicePosition.signedNotional`（long→+, short→-）；
    /// Flat 是 Rust 域显式空仓态（Lean Side 只 long/short，Flat 在 Rust 对应 notional_mag=0 的
    /// 退化，本函数直接返回 0 使空仓声部不进毛/净聚合）。
    pub fn signed_notional(&self) -> i64 {
        match self.side {
            VoiceSide::Long => self.notional_mag,
            VoiceSide::Short => -self.notional_mag,
            VoiceSide::Flat => 0,
        }
    }

    /// 名义大小 |n_v|（毛敞口贡献；Flat ⟹ 0）。
    fn gross_contribution(&self) -> i64 {
        match self.side {
            VoiceSide::Flat => 0,
            _ => self.notional_mag,
        }
    }
}

/// 毛名义头寸 G_t = Σ_v |n_v|（strict §11 line 347 / FULL 十三 line 985-987）。
///
/// 各声部名义大小之和（绝对值之和——双开的两腿都计入，故毛敞口高）。对齐 Lean
/// `Origin.LeverageCapital.grossNotional`。
pub fn gross_notional(voices: &[VoiceNotional]) -> i64 {
    voices.iter().map(VoiceNotional::gross_contribution).sum()
}

/// 净名义头寸 N_t = |Σ_v n_v|（strict §11 line 348 / FULL 十三 line 991-996）。
///
/// 各声部有符号名义的代数和的绝对值（多空抵消——双开两腿符号相反，故净敞口可低）。对齐 Lean
/// `Origin.LeverageCapital.netNotional`。
pub fn net_notional(voices: &[VoiceNotional]) -> i64 {
    voices.iter().map(VoiceNotional::signed_notional).sum::<i64>().abs()
}

/// 杠杆度量结果（毛杠杆 L^G、净杠杆 L^N，strict §11 line 355-357 / FULL 十三 line 1000-1004）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeverageMetrics {
    /// 毛敞口 G_t（美元）。
    pub gross: i64,
    /// 净敞口 N_t（美元）。
    pub net: i64,
    /// 毛杠杆 L^G_t = G_t / E_t。
    pub gross_lev: f64,
    /// 净杠杆 L^N_t = N_t / E_t。
    pub net_lev: f64,
}

/// 计算杠杆度量 L^G/L^N（strict §11 line 355-357 / FULL 十三 line 1000-1004，bit-exact）。
///
/// `L^G = G_t / E_t`，`L^N = N_t / E_t`（名义除以权益）。E_t ≤ 0 时杠杆无定义（破产态由
/// §11 风险模式 M0 处理），返回 `f64::INFINITY`（杠杆爆表 ⟹ 任何上限约束都违反，安全侧）。
///
/// ★净 ≤ 毛（对齐 Lean `net_le_gross`）：N_t = |Σ n_v| ≤ Σ|n_v| = G_t（三角不等式），故
/// L^N ≤ L^G（E_t>0 时除以正权益保序）。这坐实 strict §11 line 359「同单位数双开可能让净杠杆
/// 很低，但总杠杆仍高，所以必须同时约束」——单约束净不够（见 [`leverage_ok`] 同时查两者）。
///
/// 边界条件：`equity <= 0` ⟹ 两杠杆 = ∞（约束必违反，让位 §11 M0 Insolvent 处理）。
pub fn leverage_metrics(voices: &[VoiceNotional], equity: f64) -> LeverageMetrics {
    let gross = gross_notional(voices);
    let net = net_notional(voices);
    let (gross_lev, net_lev) = if equity <= 0.0 {
        (f64::INFINITY, f64::INFINITY)
    } else {
        (gross as f64 / equity, net as f64 / equity)
    };
    LeverageMetrics { gross, net, gross_lev, net_lev }
}

/// 杠杆上限（Θ_leverage 参数：毛杠杆上限 L̄^G、净杠杆上限 L̄^N，strict §12 line 396-397 /
/// FULL 十四 line 1086-1087：C6/C7 约束）。
///
/// ★诚实（formalization-validity-domain）：L̄^G/L̄^N 是 Θ_leverage 参数（**非缠论可导**）——
/// 缠论结构不能推出杠杆上限。本结构承载给定的上限值，still-MISSING（L2）：上限的经验校准
/// （多大上限在真实账户避免强平）属 EmpiricalDomain，不由本模块声称。
#[derive(Debug, Clone, Copy)]
pub struct LeverageCaps {
    /// 毛杠杆上限 L̄^G。
    pub gross_cap: f64,
    /// 净杠杆上限 L̄^N。
    pub net_cap: f64,
}

/// 杠杆约束检查（strict §12 line 396-397 / FULL 十四 C6/C7，契约锚 Lean
/// `Origin.ConstraintSystem.c6_grossLev / c7_netLev`）。
///
/// 返回 `true` ⟺ **同时**满足 `L^G ≤ L̄^G`（毛上限）**且** `L^N ≤ L̄^N`（净上限）。
///
/// ★必须同时约束（strict §11 line 359 / Lean `net_ok_not_imply_gross_ok`）：净约束**不能**替代
/// 毛约束——双开 long k + short k 使 N=0（净约束平凡满足）但 G=2k（毛约束可违反）。故本函数
/// 查两者合取，不是只查净（只查净会漏掉双开抬高的毛敞口风险）。
///
/// 边界条件：E_t ≤ 0 ⟹ [`leverage_metrics`] 返回 ∞ ⟹ 两约束都违反 ⟹ 返回 `false`（安全侧：
/// 破产态拒绝任何杠杆，让位 §11 M0 Insolvent 全局平仓）。
pub fn leverage_ok(metrics: LeverageMetrics, caps: LeverageCaps) -> bool {
    metrics.gross_lev <= caps.gross_cap && metrics.net_lev <= caps.net_cap
}

// ──────────────────────────────────────────────────────────────────────────
//  §13' 毛头寸约束的 units 空间形式（G7：codex #122 终裁 + codex decide 5b46）
//
//  strict §11 的毛杠杆约束 `L^G = G_t/E_t ≤ L̄^G` 在单标的 units 空间的精确等价形式：
//  `G_t = gross_units·px`、`E_t = base_units·px`（runner `base_units = NAV/px`）⟹
//  `L^G ≤ γ̄ ⟺ gross_units ≤ γ̄·base_units`（px>0 两侧对消，f64 精确，无 i64 量化）。
//  生产 K_Θ 的毛约束判定走本节（coverage.rs `apply_gross_cap` 调用，不私写同义比较）；
//  [`gross_notional`]/[`leverage_metrics`]/[`leverage_ok`] 保留为美元空间 Lean 镜像
//  （`Origin.LeverageCapital`），两空间等价由测试 `gross_units_ok_iff_leverage_ok` 锁死。
// ──────────────────────────────────────────────────────────────────────────

/// 毛敞口 units 上限 `Ḡ = γ̄·U_ℓ`（无量纲毛杠杆上限 × 协变资本单位，d_j=1 名义协变，
/// 与 coverage.rs `feasible_net_cap` 净 cap 同构——毛/净共用同一 `γ` 参数，#122 裁定
/// 暂不拆 gross_gamma/net_gamma）。
pub fn gross_units_cap(base_units: f64, gamma: f64) -> f64 {
    gamma.abs() * base_units.abs()
}

/// 毛头寸约束判定（units 空间，= strict §11 `L^G ≤ L̄^G` 单标的精确等价形式，见 §13' 节头）。
///
/// 返回 `true` ⟺ `gross_units ≤ γ̄·base_units`。与 [`leverage_ok`] 的毛分量在
/// `E = base_units·px` 下逐点等价（等价性测试锁死）；净分量由 coverage.rs
/// `pi_theta_position` 的净 cap（同一 `γ`）承担——毛+净同时约束（strict §11 line 359）。
pub fn gross_units_ok(gross_units: f64, base_units: f64, gamma: f64) -> bool {
    gross_units <= gross_units_cap(base_units, gamma)
}

// ──────────────────────────────────────────────────────────────────────────
//  真保证金模型（D2，task #113；设计锚 `.chanlun/review-results/margin-model-design-20260703.md` v2）
//
//  存在论：MM/liq_flag 来自交易所公开规则表的**版本化 datum**（非 Θ 参数，避免 codex-p2 §D2 否定的
//  「虚构 config 保证金率」）；buffer1/2 是 Θ_risk 协变缓冲（§2.3，交易所不公布策略减仓垫）。
//  认识论 L1（规则转录一致性，golden 对照交易所公布公式），不声称 L2 盈利。
// ──────────────────────────────────────────────────────────────────────────

/// Binance 分级维保档（交易所公布规则的一档，datum）。§2.2。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarginTier {
    /// 该档名义下界（美元，含）。
    pub notional_floor: f64,
    /// 维持保证金率 MMR ∈ (0,1]。
    pub mmr: f64,
    /// 维持速算额（美元，≥0；分档计算的固定扣减项）。
    pub maint_amount: f64,
}

/// 交易所保证金规则表（版本化 datum）。§2.2/§1.2。
#[derive(Debug, Clone, PartialEq)]
pub enum MarginSchedule {
    /// Binance 分级：`MM = N·mmr(tier) − maint_amount(tier)`，tier = 最高 floor ≤ N 的档。
    BinanceTiered { tiers: Vec<MarginTier> },
    /// CME 简化口径（**CME-simple，非 SPAN 全场景**，§1.2）：`MM = pct_maint·N·retail_mult`。
    CmeSimple { pct_maint: f64, retail_mult: f64 },
}

impl MarginSchedule {
    /// fail-loud 构造 Binance 分级（§2.9）：非空、floor 严格升序且首档覆盖 0、mmr∈(0,1]、maint≥0、
    /// 全有限，否则 `Err`（禁静默）。
    pub fn binance_tiered(tiers: Vec<MarginTier>) -> Result<Self, String> {
        if tiers.is_empty() {
            return Err("margin: empty tiers".into());
        }
        if tiers[0].notional_floor > 0.0 {
            return Err("margin: first tier floor must cover 0 (≤0)".into());
        }
        let mut prev_floor = f64::NEG_INFINITY;
        for (i, t) in tiers.iter().enumerate() {
            if !(t.notional_floor.is_finite() && t.mmr.is_finite() && t.maint_amount.is_finite()) {
                return Err(format!("margin: non-finite in tier {i}"));
            }
            if t.notional_floor <= prev_floor {
                return Err(format!("margin: tiers not strictly ascending at {i}"));
            }
            if !(t.mmr > 0.0 && t.mmr <= 1.0) {
                return Err(format!("margin: mmr∉(0,1] at {i}"));
            }
            if t.maint_amount < 0.0 {
                return Err(format!("margin: maint_amount<0 at {i}"));
            }
            prev_floor = t.notional_floor;
        }
        Ok(MarginSchedule::BinanceTiered { tiers })
    }

    /// fail-loud 构造 CME-simple（§2.9）：pct∈(0,1]、retail≥1、有限，否则 `Err`。
    pub fn cme_simple(pct_maint: f64, retail_mult: f64) -> Result<Self, String> {
        if !(pct_maint.is_finite() && pct_maint > 0.0 && pct_maint <= 1.0) {
            return Err("margin: cme pct_maint∉(0,1]".into());
        }
        if !(retail_mult.is_finite() && retail_mult >= 1.0) {
            return Err("margin: cme retail_mult<1".into());
        }
        Ok(MarginSchedule::CmeSimple { pct_maint, retail_mult })
    }

    /// 维持保证金 `MM(net_notional_usd)`。net_notional **已是美元**（§2.2 单位修正：勿再乘价）。
    /// `MM≥0`（速算额不使 MM 转负）。
    pub fn maint_margin(&self, net_notional_usd: f64) -> f64 {
        let n = net_notional_usd.abs();
        match self {
            MarginSchedule::BinanceTiered { tiers } => {
                // tier = 最高 floor ≤ n（tiers 升序 ⟹ partition_point 二分；首档 floor≤0 保非空）。
                let idx = tiers.partition_point(|t| t.notional_floor <= n).saturating_sub(1);
                let t = &tiers[idx];
                (n * t.mmr - t.maint_amount).max(0.0)
            }
            MarginSchedule::CmeSimple { pct_maint, retail_mult } => n * pct_maint * retail_mult,
        }
    }
}

/// 分段快照簿（§2.7）：回测 bar 时间 → 生效快照，**禁未来快照泄漏**（零前视）。
#[derive(Debug, Clone, PartialEq)]
pub struct MarginScheduleBook {
    /// 各段 `(effective_from 含, effective_to 不含, schedule)`，按 from 升序不重叠。
    snapshots: Vec<(Timestamp, Timestamp, MarginSchedule)>,
}

impl MarginScheduleBook {
    /// fail-loud 构造：非空、各段 `from<to`、按 from 升序不重叠，否则 `Err`。
    pub fn new(snapshots: Vec<(Timestamp, Timestamp, MarginSchedule)>) -> Result<Self, String> {
        if snapshots.is_empty() {
            return Err("margin book: empty".into());
        }
        let mut prev_to: Option<Timestamp> = None;
        for (from, to, _) in &snapshots {
            if from >= to {
                return Err("margin book: from>=to".into());
            }
            if let Some(pt) = prev_to {
                if *from < pt {
                    return Err("margin book: overlapping or unsorted".into());
                }
            }
            prev_to = Some(*to);
        }
        Ok(MarginScheduleBook { snapshots })
    }

    /// as_of：`bar_ts` 落在哪段 `[from,to)`。无覆盖段 ⟹ `None`（有效域外，不借用未来快照）。
    pub fn as_of(&self, bar_ts: Timestamp) -> Option<&MarginSchedule> {
        self.snapshots
            .iter()
            .find(|(f, t, _)| bar_ts >= *f && bar_ts < *t)
            .map(|(_, _, s)| s)
    }
}

/// **分段 funding 费率快照簿**（A10 C2 裁定 additive 接口预冻结，与 [`MarginScheduleBook`]
/// 同构模式）：各段 `(effective_from 含, effective_to 不含, signed_rate)`——**signed_rate 带符号
/// datum**（venue+symbol 维度、结算周期对齐、版本化快照；费率标定是 L2 缺口，datum 到位即插，
/// 签名不变）。
///
/// ★venue 适用性（#303 裁定 2026-07-26）：资金费是**永续**venue 的科目；本仓 BTC 数据窗 =
/// **Binance 现货**（无 funding），故本簿在当前生产路径**不注入**（`m6_cost_model()` 走无向
/// 保底通道，现货语义 = 资金占用机会成本，见 [`CostModel`] 节头）。本类型是真永续接入的
/// 预冻结承接位——真 funding 数据接入是另票（#62 数据源 + datum 版本管理），本票不做。
///
/// - `as_of` 零前视：`bar_ts` 落在哪段 `[from,to)`（from 含/to 不含）；无覆盖段 ⟹ `None`
///   （有效域外，不借用未来快照，不外推跨段）。
/// - fail-loud 构造：空 / `from>=to` / 重叠或乱序 / 非有限费率 ⟹ `Err`（禁静默退化）。
/// - 消费配对：[`CostModel::funding_accrual_signed`]——`as_of(bar_ts)` 取 signed_rate 后
///   按持仓符号定成本方向（long×rate>0 付费、short×rate>0 收费；符号表锚 venue 文档 golden）。
///
/// ★口径标签（A10 附则B 裁决2）：datum 未注入期间，一切带 funding 成本的 R 数值报告强制
/// 标注 [`RATE_UNCALIBRATED_LABEL`]；datum 注入后升 `[L2费率标定: datum 版本哈希]`，不得跳级。
#[derive(Debug, Clone, PartialEq)]
pub struct FundingScheduleBook {
    /// 各段 `(effective_from 含, effective_to 不含, signed_rate)`，按 from 升序不重叠。
    snapshots: Vec<(Timestamp, Timestamp, f64)>,
}

impl FundingScheduleBook {
    /// fail-loud 构造：非空、各段 `from<to`、按 from 升序不重叠、费率有限，否则 `Err`。
    pub fn new(snapshots: Vec<(Timestamp, Timestamp, f64)>) -> Result<Self, String> {
        if snapshots.is_empty() {
            return Err("funding book: empty".into());
        }
        let mut prev_to: Option<Timestamp> = None;
        for (from, to, rate) in &snapshots {
            if from >= to {
                return Err("funding book: from>=to".into());
            }
            if let Some(pt) = prev_to {
                if *from < pt {
                    return Err("funding book: overlapping or unsorted".into());
                }
            }
            if !rate.is_finite() {
                return Err("funding book: non-finite signed_rate".into());
            }
            prev_to = Some(*to);
        }
        Ok(FundingScheduleBook { snapshots })
    }

    /// as_of：`bar_ts` 落在哪段 `[from,to)` 的 signed_rate。无覆盖段 ⟹ `None`
    /// （有效域外，零前视——未来快照不可见）。
    pub fn as_of(&self, bar_ts: Timestamp) -> Option<f64> {
        self.snapshots
            .iter()
            .find(|(f, t, _)| bar_ts >= *f && bar_ts < *t)
            .map(|(_, _, r)| *r)
    }
}

/// **费率未标定口径标签**（A10 C2/附则B 裁定强制）：v0 三常费率（`funding_rate_per_period`/
/// `borrow_rate_per_bar`/`liq_penalty_rate`）是机制闭合用保底参数，非 venue datum 标定——
/// **一切带成本 R 数值报告强制带本标签**；常费率数值**禁作 alpha 论据、禁作策略择优输入**
/// （v3：历史数据只验证代码正确性）。datum 注入后升 `[L2费率标定: datum 版本哈希]`（不得跳级）。
pub const RATE_UNCALIBRATED_LABEL: &str = "[L1机制/费率未标定]";

/// 去杠杆/只平仓缓冲 B1/B2（Θ_risk 协变参数，§2.3；**非**交易所数据）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskCushions {
    pub buffer1: f64,
    pub buffer2: f64,
}

impl RiskCushions {
    /// fail-loud（§2.9 + strict §11 前提 `0<B1<B2`），否则 `Err`。
    pub fn new(buffer1: f64, buffer2: f64) -> Result<Self, String> {
        if !(buffer1.is_finite() && buffer2.is_finite()) {
            return Err("cushions: non-finite".into());
        }
        if !(0.0 < buffer1 && buffer1 < buffer2) {
            return Err("cushions: require 0<B1<B2".into());
        }
        Ok(RiskCushions { buffer1, buffer2 })
    }
}

/// 完整保证金模型注入（`ThetaConfig.margin`）。`None` ⟹ 保持 MM=0 退化口径（bit-exact 现状）。
#[derive(Debug, Clone, PartialEq)]
pub struct MarginModel {
    pub book: MarginScheduleBook,
    pub cushions: RiskCushions,
}

/// 从持仓净名义 + 权益 + 规则表派生 [`RiskModeInput`]（§2.1/§2.4：统一算 MM+liq_flag，单一数据源，
/// 避免 v1 `liquidation_flag` 签名缺 equity 的不自洽）。
///
/// - `net_notional_usd`：当前净名义敞口（**美元，已折算**，勿再乘价，§2.2）。
/// - `equity`：盯市账户权益 E_t（§2.5）。
/// - `schedule`：`as_of` 取到的当段规则表；`cushions`：Θ_risk 缓冲。
/// - `liq_flag = equity ≤ MM`（§2.4，v0 逐仓价格触发；ADL/funding 为缺口，诚实标注）。
pub fn margin_inputs(
    net_notional_usd: f64,
    equity: f64,
    schedule: &MarginSchedule,
    cushions: &RiskCushions,
) -> RiskModeInput {
    let mm = schedule.maint_margin(net_notional_usd);
    RiskModeInput {
        equity,
        maint_margin: mm,
        buffer1: cushions.buffer1,
        buffer2: cushions.buffer2,
        liq_flag: equity <= mm,
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  M6 持仓期/强平成本模型：周期性持有成本（科目名 Funding）+ Borrow（杠杆借贷）
//  + LiquidationLoss（强平罚金）
//  （TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关：
//   R = Σ N_t ΔP_t − Commission − Slippage − Funding − Borrow − LiquidationLoss）
//
//  ★venue 口径（#303 编排者裁定 2026-07-26，切 **spot**）：本仓价格数据 = **Binance 现货**
//  （`btc_1m_full.json` ← data.binance.vision/data/**spot**/monthly/klines，
//  `scripts/download_btc_binance.py:38-40`）。现货**无资金费（funding）**——三通道在现货口径下
//  分别是：周期通道 = **资金占用机会成本**（按 |净名义| 每周期一次），Borrow = **现货杠杆借贷
//  利息**（按借入名义 max(0,|N|−E) 逐 bar），LiquidationLoss = **现货杠杆强平罚金**。
//  永续资金费（正负互付、8h 结算）机制在此**保留但不适用于本数据窗**：`FundingScheduleBook`
//  + `funding_accrual_signed` 是 perp datum 的预冻结承接位（A10 C2），真 funding 数据接入是
//  **另票**（#62 数据源 + datum 版本管理），本票不做。
//  命名落差照实登记（090，不掩盖）：字段/科目仍叫 `funding_*` / `Funding`，是 A10 C2/F2 冻结的
//  历史命名（签名逐字不动），**不代表本数据窗收取资金费**；其现货语义以本节与各字段 doc 为准。
//
//  ★两通道基数重叠 ⟹ 持有成本是**上界**（照实登记，#303 影子评审 C1）：周期通道按 |N| **全额**
//  计，borrow 按借入名义 max(0,|N|−E) 计——借入的那部分被两个通道同时计入（机会成本 + 借币利息）。
//  永续语义下二者是互不重叠的科目（资金费按全名义、借贷按借入额），改判现货语义后重叠显形。
//  当前**不改计算**（票 #303 裁定「按现参数」，且重叠方向是**高估成本**＝保守，不美化回测结果）；
//  严格分解（自有 min(|N|,E) → 机会成本、借入 max(0,|N|−E) → 借币利息）是**费率标定（L2）时**
//  与 datum 一并落的待办，不在本票。故三项之和读作持有成本上界，非精确分科。
//
//  存在论：Commission/Slippage 已由 ExecConfig fee_rate 进 apply_fill；本模型补上验收公式剩余
//  三项。**有效域声明（231号 / formalization-validity-domain）**：v0 用**参数化常费率**——真实
//  现货借贷利率曲线 / 机会成本基准（以及将来真永续的资金费历史）是**外部数据源缺口**（L2），
//  A10 waiver 豁免的正是这类外部数据源，**不豁免机制实装**。机制在此真实装（逐 bar 计提、进
//  PnL、进 R 分解、守恒断言），费率标定待外部数据（结果包声明）。认识论 L1（机制正确性，非 L2
//  盈利）。
// ──────────────────────────────────────────────────────────────────────────

/// M6 成本模型（`ThetaConfig.cost_model`）。`None` ⟹ 三项成本恒 0（bit-exact 现状，M6 前口径）。
///
/// 字段语义（venue 口径 = **spot**，#303 裁定；命名落差见节头）：
/// - `funding_rate_per_period`：**周期性持有成本率**，作用于 |净名义|，每 `funding_period_bars`
///   收一次。现货口径 ⟹ **资金占用机会成本**（现货无资金费）；永续口径 ⟹ 资金费率（本数据窗
///   不适用）。v0 取 |N| 绝对持有成本（**不分多空方向**）——机会成本对多空同向发生，取绝对值即
///   其严格形态；永续资金费 long/short 互付的方向性由 [`CostModel::funding_accrual_signed`] + datum 承载
///   （另票），常费率保底不表达方向（诚实缺口，见节头 231号声明）。
/// - `funding_period_bars`：周期通道的结算 bar 数（≥1；调用方按数据 bar 间隔换算结算周期）。
/// - `borrow_rate_per_bar`：每 bar 借贷成本率（作用于**借入名义** = max(0, |N|−E)——杠杆超出自有
///   权益的部分是借入的；现货口径 = 现货杠杆借币利息）。
/// - `liq_penalty_rate`：强平罚金率（作用于强平时刻 |净名义|——交易所强平清算费/滑点罚金；现货
///   口径 = 现货杠杆强平）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostModel {
    pub funding_rate_per_period: f64,
    pub funding_period_bars: u32,
    pub borrow_rate_per_bar: f64,
    pub liq_penalty_rate: f64,
}

impl CostModel {
    /// fail-loud 构造（禁静默退化，对齐 MarginSchedule/RiskCushions 校验风格）：费率有限且 ≥0、
    /// funding_period_bars ≥1，否则 `Err`。
    pub fn new(
        funding_rate_per_period: f64,
        funding_period_bars: u32,
        borrow_rate_per_bar: f64,
        liq_penalty_rate: f64,
    ) -> Result<Self, String> {
        if !(funding_rate_per_period.is_finite() && funding_rate_per_period >= 0.0) {
            return Err("cost: funding_rate_per_period 须有限且≥0".into());
        }
        if funding_period_bars == 0 {
            return Err("cost: funding_period_bars 须≥1".into());
        }
        if !(borrow_rate_per_bar.is_finite() && borrow_rate_per_bar >= 0.0) {
            return Err("cost: borrow_rate_per_bar 须有限且≥0".into());
        }
        if !(liq_penalty_rate.is_finite() && liq_penalty_rate >= 0.0) {
            return Err("cost: liq_penalty_rate 须有限且≥0".into());
        }
        Ok(CostModel {
            funding_rate_per_period,
            funding_period_bars,
            borrow_rate_per_bar,
            liq_penalty_rate,
        })
    }

    /// 单 bar 周期性持有成本计提（美元，≥0；现货口径 = 资金占用机会成本，#303）。仅在周期边界
    /// bar 收取（`bar_index>0 ∧ bar_index % funding_period_bars == 0`）——每周期一次，非逐 bar；
    /// `|net_notional_usd|` × 费率。空仓（N=0）⟹ 0。
    pub fn funding_accrual(&self, bar_index: usize, net_notional_usd: f64) -> f64 {
        let period = self.funding_period_bars as usize;
        if bar_index == 0 || bar_index % period != 0 {
            return 0.0;
        }
        net_notional_usd.abs() * self.funding_rate_per_period
    }

    /// **有向 funding 计提**（A10 C2 裁定 additive 接口预冻结；美元，**带符号**：正=付费成本，
    /// 负=收费收入）。★venue 适用性（#303）：有向互付是**永续**资金费的形态——本仓 BTC 数据窗是
    /// **现货**（无 funding），故本方法在当前生产路径**不被调用**（`FundingScheduleBook` 亦不注入）；
    /// 它是真永续接入（另票：#62 datum + datum 版本管理）的预冻结承接位，签名/语义在此保持不动。
    /// 周期边界口径与 [`funding_accrual`](Self::funding_accrual) 逐字一致
    /// （`bar_index>0 ∧ bar_index % funding_period_bars == 0`），只换费率来源与方向语义：
    ///
    /// - `signed_rate`：带符号费率 datum（由 [`FundingScheduleBook::as_of`] 零前视取得——
    ///   费率标定 L2 缺口，datum 到位即插；**禁**徒手喂未标定常数进生产报告）。
    /// - `net_notional_usd`：**带符号**净名义（正=净多，负=净空——与无向保底的 `.abs()` 口径
    ///   不同，调用方传符号值）。
    /// - 符号表（锚 venue 文档 golden，T-N2 钉死）：long（N>0）×rate>0 ⟹ **正**（付费=成本）；
    ///   short（N<0）×rate>0 ⟹ **负**（收费=负成本=收入）；rate<0 镜像翻转；空仓（N=0）恒 0。
    ///
    /// ★F2 锁死：本方法是 additive 新增——[`funding_accrual`](Self::funding_accrual) 无向保底
    /// 路径签名/字段/语义逐字不动（既有 5+3 测试锁死）；datum 未注入期间一切带 funding 成本的
    /// R 数值报告强制标注 [`RATE_UNCALIBRATED_LABEL`]（附则B）。
    pub fn funding_accrual_signed(&self, bar_index: usize, net_notional_usd: f64, signed_rate: f64) -> f64 {
        let period = self.funding_period_bars as usize;
        if bar_index == 0 || bar_index % period != 0 {
            return 0.0;
        }
        net_notional_usd * signed_rate
    }

    /// 单 bar 借贷成本（美元，≥0）：借入名义 = `max(0, |N|−E)`（杠杆超出权益部分）× 每 bar 率。
    /// 未用杠杆（|N|≤E）或空仓 ⟹ 0。`equity≤0`（破产态）⟹ 借入 = 全部 |N|（无自有权益覆盖）。
    pub fn borrow_accrual(&self, net_notional_usd: f64, equity: f64) -> f64 {
        let borrowed = (net_notional_usd.abs() - equity.max(0.0)).max(0.0);
        borrowed * self.borrow_rate_per_bar
    }

    /// 强平罚金（美元，≥0）：强平时刻 `|净名义|` × 罚金率。空仓 ⟹ 0。
    pub fn liquidation_penalty(&self, net_notional_usd: f64) -> f64 {
        net_notional_usd.abs() * self.liq_penalty_rate
    }
}

/// **R 分解表**（路线.pdf p16 第十一关：`R = Σ N_t ΔP_t − Commission − Slippage − Funding −
/// Borrow − LiquidationLoss`）。各项**独立累计**（非从 net_r 反推），守恒断言校验和 = 账本净变动。
///
/// - `price_pnl_gross` = `Σ_t N_t·ΔP_t`：mark-to-market 逐 bar 价格贡献（含浮盈），**不含**任何费用。
/// - `commission_slippage`：ExecConfig fee_rate（commission+slippage+tax）扣的成交费（含税项）。
/// - `funding`/`borrow`/`liquidation_loss`：本模型三项（None ⟹ 全 0）。★口径（#303，spot）：
///   `funding` 行记的是**周期性持有成本**——现货口径 = 资金占用机会成本（本仓 BTC 数据窗），
///   非永续资金费；科目名沿用 p16 验收公式的 `Funding`（命名落差见 [`CostModel`] 节头）。
/// - `net_r` = `price_pnl_gross − commission_slippage − funding − borrow − liquidation_loss`。
/// - `ledger_delta`：账本**独立**测得的净变动 = `final_equity_abs − nav0`（含浮盈强平）。
/// - `conservation_residual` = `net_r − ledger_delta`：守恒残差，应 ≈0（浮点容差）。非零 = 资金泄漏 bug。
/// - `tw_holding_cost_bridge`：**TW 桥对账行**（A10 C5 裁定 (b)，G1）——TW 账本（i64，#124 裁定4
///   单一生产真值源）不经任何构造子见到持盾成本（GAP3 A' 冻结，零账本侵入），故 η=tw() **高估**
///   真实在险权益恰 = 本字段（= ⌊funding+borrow+liquidation_loss⌋ 累计量化，f64→定点口径与
///   treasury `Realize(⌊realized_cum⌋)` 一致）。消费侧 η_corrected = tw() − tw_holding_cost_bridge
///   （enter_ready 判据与 η_bucket 同源修正，F4）。cost_model=None ⟹ 恒 0（bit-exact）。
///   ★守恒范围声明：R 守恒覆盖 f64 cash 域、TW 守恒覆盖 i64 TW 域，两账本不同构（#90/674）——
///   本行是两域间**对账不变量**，不声称跨账本单一 Realize/统一账本（ledger.rs 裁定清单⑨）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RDecomposition {
    pub price_pnl_gross: f64,
    pub commission_slippage: f64,
    pub funding: f64,
    pub borrow: f64,
    pub liquidation_loss: f64,
    pub net_r: f64,
    pub ledger_delta: f64,
    pub conservation_residual: f64,
    pub tw_holding_cost_bridge: i64,
}

impl RDecomposition {
    /// 从各独立累计项 + 账本净变动 + TW 桥对账行组装，算 net_r + 守恒残差。
    pub fn assemble(
        price_pnl_gross: f64,
        commission_slippage: f64,
        funding: f64,
        borrow: f64,
        liquidation_loss: f64,
        ledger_delta: f64,
        tw_holding_cost_bridge: i64,
    ) -> Self {
        let net_r = price_pnl_gross - commission_slippage - funding - borrow - liquidation_loss;
        RDecomposition {
            price_pnl_gross,
            commission_slippage,
            funding,
            borrow,
            liquidation_loss,
            net_r,
            ledger_delta,
            conservation_residual: net_r - ledger_delta,
            tw_holding_cost_bridge,
        }
    }
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

    /// bit-exact 标量默认 SizingInput（rho/gamma 取 RiskConfig::default 标量、gap=0）——
    /// 改动前的 sizing 行为（无 (level,side) override），所有现有 sizing 测试的期望值据此。
    fn base_sizing() -> SizingInput {
        SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
            tick_size: 1.0, // 测试用 tick_size=1 ⟹ tick=美元，期望值不变
            cost_per_unit: 0.0,
            w_depth: 0.6,
            parent_cap: i64::MAX,
            rho: 0.005,      // = RiskConfig::default().rho（标量退化）
            gamma: 1.0,      // = RiskConfig::default().gamma
            gap_buffer: 0.0, // 无 GapBuffer（bit-exact）
        }
    }

    /// sizing 三路 min（spec:47）：风险预算项约束最紧的情形。
    #[test]
    fn sizing_risk_budget_binds() {
        let cfg = RiskConfig::default(); // ρ=0.005, β=0.5, γ=1.0, κ=2.0, lot=1
        // NAV=1_000_000, entry=100 tick（tick_size=1.0 ⟹ entry_usd=100），stop=90（|d_usd|=10），cost=0。
        // 项1 = floor(0.005*1e6 / (10*1.0 + 2*0)) = floor(5000/10) = 500。
        // 项2 = floor(0.6*1.0*1e6 / (100*1.0)) = floor(6000) = 6000。
        // 项3 = MAX。 min = 500。
        let inp = base_sizing();
        assert_eq!(size_position(&inp, &cfg), 500);
    }

    /// sizing：名义上限项约束最紧。
    #[test]
    fn sizing_notional_cap_binds() {
        let cfg = RiskConfig::default();
        // entry=100, stop=99 (|d_usd|=1*1.0=1) ⟹ 项1 = floor(5000/1)=5000；
        // w_depth=0.1: 项2 = floor(0.1*1e6/(100*1.0))=1000；项1=5000 ⟹ min=1000。
        let inp = SizingInput { stop: 99, w_depth: 0.1, ..base_sizing() };
        assert_eq!(size_position(&inp, &cfg), 1000);
    }

    /// sizing：parent_cap 约束最紧。
    #[test]
    fn sizing_parent_cap_binds() {
        let cfg = RiskConfig::default();
        let inp = SizingInput { parent_cap: 50, ..base_sizing() }; // 比项1=500、项2=6000 都小
        assert_eq!(size_position(&inp, &cfg), 50);
    }

    /// sizing：qty<=0 不交易（parent_cap=0 = 父空仓）。
    #[test]
    fn sizing_zero_qty_no_trade() {
        let cfg = RiskConfig::default();
        let inp = SizingInput { parent_cap: 0, ..base_sizing() };
        assert_eq!(size_position(&inp, &cfg), 0);
    }

    /// sizing：entry<=0 不交易（无效入场价）。
    #[test]
    fn sizing_nonpositive_entry_no_trade() {
        let cfg = RiskConfig::default();
        let inp = SizingInput { entry: 0, stop: -10, ..base_sizing() };
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
        let inp = base_sizing();
        assert_eq!(size_position(&inp, &cfg), 500);
        // parent_cap=250, lot=100 ⟹ min(500,...,250)=250 ⟹ floor(250/100)*100=200。
        let inp2 = SizingInput { parent_cap: 250, ..inp };
        assert_eq!(size_position(&inp2, &cfg), 200);
    }

    /// ★PDF §2 风险归一化：q_Θ 随 D 增大（止损更远 / GapBuffer 更大）而单调减小。
    /// D = M·|P−S| + Cost + GapBuffer；项1 = ρW/D，D↑ ⟹ q↓。
    #[test]
    fn sizing_q_decreases_with_distance() {
        let cfg = RiskConfig::default(); // ρ=0.005
        // 近止损（|P−S|=10）：项1 = floor(5000/10)=500。
        let near = SizingInput { stop: 90, ..base_sizing() };
        // 远止损（|P−S|=50）：项1 = floor(5000/50)=100。
        let far = SizingInput { stop: 50, ..base_sizing() };
        let q_near = size_position(&near, &cfg);
        let q_far = size_position(&far, &cfg);
        assert!(q_far < q_near, "止损更远 ⟹ D 更大 ⟹ q 更小（风险归一化）");
        assert_eq!(q_near, 500);
        assert_eq!(q_far, 100);
        // GapBuffer 增大 D：gap=40 让近止损的 D 从 10 → 50 ⟹ q 退到 100。
        let near_gap = SizingInput { stop: 90, gap_buffer: 40.0, ..base_sizing() };
        assert_eq!(size_position(&near_gap, &cfg), 100, "GapBuffer 加进 D ⟹ q↓");
    }

    /// ★PDF §3：不同 ρ（按 level 解析）→ 不同 q（风险预算项缩放）。
    #[test]
    fn sizing_different_rho_different_q() {
        let cfg = RiskConfig::default();
        // ρ=0.005 ⟹ 项1=floor(0.005*1e6/10)=500。
        let lo = SizingInput { rho: 0.005, ..base_sizing() };
        // ρ=0.010（某 level override）⟹ 项1=floor(0.010*1e6/10)=1000。
        let hi = SizingInput { rho: 0.010, ..base_sizing() };
        assert_eq!(size_position(&lo, &cfg), 500);
        assert_eq!(size_position(&hi, &cfg), 1000);
        assert_ne!(size_position(&lo, &cfg), size_position(&hi, &cfg));
    }

    /// ★PDF §3：多空不强行镜像——ρ_{ℓ,+} ≠ ρ_{ℓ,−} ⟹ 同结构 q 不同。
    /// 空头有借券费/保证金/尾部风险不同，故 ρ_{ℓ,−} 可独立于 ρ_{ℓ,+}。
    #[test]
    fn sizing_long_short_not_mirrored() {
        let cfg = RiskConfig::default();
        // 做多用 ρ_{ℓ,+}=0.006 ⟹ 项1=floor(6000/10)=600。
        let long = SizingInput { rho: 0.006, ..base_sizing() };
        // 做空用更保守的 ρ_{ℓ,−}=0.003 ⟹ 项1=floor(3000/10)=300（不镜像 = 不等于 long）。
        let short = SizingInput { rho: 0.003, ..base_sizing() };
        assert_eq!(size_position(&long, &cfg), 600);
        assert_eq!(size_position(&short, &cfg), 300);
        assert_ne!(
            size_position(&long, &cfg),
            size_position(&short, &cfg),
            "ρ_{{ℓ,+}}≠ρ_{{ℓ,−}} ⟹ 多空 q 不镜像"
        );
    }

    // ── §13 杠杆系统测试（strict §11 / FULL 十三，Lean Origin.LeverageCapital）──

    /// 有符号名义 n_v = σ_v·|n_v|（Long→+，Short→-，Flat→0）。
    #[test]
    fn signed_notional_by_side() {
        let long = VoiceNotional { side: VoiceSide::Long, notional_mag: 100 };
        let short = VoiceNotional { side: VoiceSide::Short, notional_mag: 100 };
        let flat = VoiceNotional { side: VoiceSide::Flat, notional_mag: 100 };
        assert_eq!(long.signed_notional(), 100);
        assert_eq!(short.signed_notional(), -100);
        assert_eq!(flat.signed_notional(), 0); // 空仓声部无名义
    }

    /// 毛/净敞口：单边持仓 G=N（无抵消）。
    #[test]
    fn gross_net_single_side() {
        let voices = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: 300 },
            VoiceNotional { side: VoiceSide::Long, notional_mag: 200 },
        ];
        assert_eq!(gross_notional(&voices), 500); // |300|+|200|
        assert_eq!(net_notional(&voices), 500); // |+300+200|=500（同向无抵消）
    }

    /// ★精确同单位数双开：净敞口低（0），毛敞口高（2k）——strict §11 line 359 的核心情形。
    /// 对齐 Lean `hedged_gross_high_net_zero`。
    #[test]
    fn hedged_gross_high_net_zero() {
        let k = 400;
        let voices = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: k },
            VoiceNotional { side: VoiceSide::Short, notional_mag: k },
        ];
        assert_eq!(net_notional(&voices), 0); // |+k-k|=0（净敞口被双开抵消为 0）
        assert_eq!(gross_notional(&voices), 2 * k); // k+k=2k（毛敞口仍满额）
    }

    /// ★净 ≤ 毛（三角不等式，Lean `net_le_gross`）：混合多空，净敞口 ≤ 毛敞口。
    #[test]
    fn net_le_gross_property() {
        let voices = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: 500 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: 200 },
            VoiceNotional { side: VoiceSide::Long, notional_mag: 100 },
        ];
        let gross = gross_notional(&voices); // 500+200+100=800
        let net = net_notional(&voices); // |+500-200+100|=400
        assert_eq!(gross, 800);
        assert_eq!(net, 400);
        assert!(net <= gross); // 净 ≤ 毛（三角不等式）
    }

    /// 杠杆度量 L^G/L^N = G/E, N/E（E>0）+ 净杠杆 ≤ 毛杠杆。
    #[test]
    fn leverage_metrics_computation() {
        let voices = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: 800 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: 300 },
        ];
        // G=1100, N=|800-300|=500, E=1000 ⟹ L^G=1.1, L^N=0.5。
        let m = leverage_metrics(&voices, 1000.0);
        assert_eq!(m.gross, 1100);
        assert_eq!(m.net, 500);
        assert!((m.gross_lev - 1.1).abs() < 1e-9);
        assert!((m.net_lev - 0.5).abs() < 1e-9);
        assert!(m.net_lev <= m.gross_lev); // 净杠杆 ≤ 毛杠杆
    }

    /// 杠杆度量 E≤0 ⟹ 杠杆 = ∞（破产态，约束必违反）。
    #[test]
    fn leverage_metrics_zero_equity_infinite() {
        let voices = [VoiceNotional { side: VoiceSide::Long, notional_mag: 100 }];
        let m = leverage_metrics(&voices, 0.0);
        assert!(m.gross_lev.is_infinite());
        assert!(m.net_lev.is_infinite());
    }

    /// ★杠杆约束必须同时查毛净（Lean `net_ok_not_imply_gross_ok`）：双开使净约束满足但毛违反。
    #[test]
    fn leverage_ok_must_check_both() {
        // 双开 long 600 + short 600：G=1200, N=0, E=1000 ⟹ L^G=1.2, L^N=0。
        let voices = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: 600 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: 600 },
        ];
        let m = leverage_metrics(&voices, 1000.0);
        // 净上限 1.0：L^N=0 ≤ 1.0 满足；毛上限 1.0：L^G=1.2 > 1.0 违反。
        let caps = LeverageCaps { gross_cap: 1.0, net_cap: 1.0 };
        // 只查净会误判 OK；同时查 ⟹ false（毛违反）。
        assert!(!leverage_ok(m, caps));
        // 放宽毛上限到 1.5 ⟹ 两者都满足 ⟹ OK。
        let caps2 = LeverageCaps { gross_cap: 1.5, net_cap: 1.0 };
        assert!(leverage_ok(m, caps2));
    }

    /// 杠杆约束 E≤0 ⟹ false（破产态拒绝杠杆）。
    #[test]
    fn leverage_ok_zero_equity_false() {
        let voices = [VoiceNotional { side: VoiceSide::Long, notional_mag: 100 }];
        let m = leverage_metrics(&voices, 0.0);
        let caps = LeverageCaps { gross_cap: 100.0, net_cap: 100.0 };
        assert!(!leverage_ok(m, caps)); // ∞ > 任何有限上限
    }

    /// ★G7 等价性锁（codex decide 5b46）：units 空间毛判定 [`gross_units_ok`] ⟺ 美元空间
    /// [`leverage_ok`] 毛分量（`G=gross_units·px`、`E=base_units·px`，px 两侧对消）。
    /// 整数精确取值（无浮点近似），跨激活/违反两侧验证 iff。
    #[test]
    fn gross_units_ok_iff_leverage_ok() {
        let px = 50.0; // 美元/unit（整数精确乘法）
        let gamma = 1.0;
        let base_units = 20.0; // E = 20×50 = $1000
        let equity = base_units * px;
        // 情形1：双开 12+12 units ⟹ gross_units=24 > γ̄·base=20（违反）。
        // 美元侧：G=24×50=1200，L^G=1.2 > 1.0（同判违反）。
        let voices_bad = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: (12.0 * px) as i64 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: (12.0 * px) as i64 },
        ];
        let m_bad = leverage_metrics(&voices_bad, equity);
        let caps = LeverageCaps { gross_cap: gamma, net_cap: f64::INFINITY }; // 只比毛分量
        assert!(!gross_units_ok(24.0, base_units, gamma));
        assert!(!leverage_ok(m_bad, caps));
        // 情形2：双开 8+8 units ⟹ gross_units=16 ≤ 20（满足）。美元侧 L^G=0.8 ≤ 1.0（同判满足）。
        let voices_ok = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: (8.0 * px) as i64 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: (8.0 * px) as i64 },
        ];
        let m_ok = leverage_metrics(&voices_ok, equity);
        assert!(gross_units_ok(16.0, base_units, gamma));
        assert!(leverage_ok(m_ok, caps));
        // 边界：gross_units 恰 = cap（20）⟹ 两侧同判满足（≤ 含界）。
        let voices_eq = [
            VoiceNotional { side: VoiceSide::Long, notional_mag: (10.0 * px) as i64 },
            VoiceNotional { side: VoiceSide::Short, notional_mag: (10.0 * px) as i64 },
        ];
        assert!(gross_units_ok(20.0, base_units, gamma));
        assert!(leverage_ok(leverage_metrics(&voices_eq, equity), caps));
    }

    /// G7 毛 cap：`Ḡ = γ̄·U_ℓ`（绝对值，与净 cap `feasible_net_cap·base_units` 同构）。
    #[test]
    fn gross_units_cap_covariant() {
        assert!((gross_units_cap(1000.0, 1.0) - 1000.0).abs() < 1e-12);
        assert!((gross_units_cap(1000.0, 0.5) - 500.0).abs() < 1e-12);
        // 协变：base_units 缩放 a 倍 ⟹ cap 缩放 a 倍（d_j=1）。
        assert!((gross_units_cap(2000.0, 0.5) - 2.0 * gross_units_cap(1000.0, 0.5)).abs() < 1e-12);
        // 符号鲁棒（abs）：负 gamma/base 不产生负 cap。
        assert!((gross_units_cap(-1000.0, -1.0) - 1000.0).abs() < 1e-12);
    }

    // ── 真保证金模型（#113，margin-model-design v2）──

    /// L1 golden-vector：Binance 分级 MM = N·mmr − maint_amount（对照公布公式，§3.1）。
    #[test]
    fn margin_binance_tier_golden() {
        // 三档（floor 升序，首档覆盖0）：[0,50k) 0.4%/0；[50k,250k) 0.5%/50；[250k,∞) 1%/1300。
        let s = MarginSchedule::binance_tiered(vec![
            MarginTier { notional_floor: 0.0, mmr: 0.004, maint_amount: 0.0 },
            MarginTier { notional_floor: 50_000.0, mmr: 0.005, maint_amount: 50.0 },
            MarginTier { notional_floor: 250_000.0, mmr: 0.01, maint_amount: 1300.0 },
        ])
        .unwrap();
        assert!((s.maint_margin(10_000.0) - (10_000.0 * 0.004)).abs() < 1e-9); // 档0
        assert!((s.maint_margin(100_000.0) - (100_000.0 * 0.005 - 50.0)).abs() < 1e-9); // 档1
        assert!((s.maint_margin(300_000.0) - (300_000.0 * 0.01 - 1300.0)).abs() < 1e-9); // 档2
        // 边界 N=50k 精确落档1（floor 含）。
        assert!((s.maint_margin(50_000.0) - (50_000.0 * 0.005 - 50.0)).abs() < 1e-9);
    }

    /// L1 golden：CME-simple = pct·N·retail（§3.1）。
    #[test]
    fn margin_cme_simple_golden() {
        let s = MarginSchedule::cme_simple(0.37, 1.10).unwrap();
        let notional = 5.0 * 60_000.0; // 5 BTC × $60k
        assert!((s.maint_margin(notional) - notional * 0.37 * 1.10).abs() < 1e-6);
    }

    /// as_of 分段 + 零前视：未来快照不泄漏（§2.7/§3）。
    #[test]
    fn margin_book_as_of_no_lookahead() {
        let s1 = MarginSchedule::cme_simple(0.30, 1.0).unwrap();
        let s2 = MarginSchedule::cme_simple(0.40, 1.0).unwrap();
        let book = MarginScheduleBook::new(vec![(100, 200, s1), (200, 300, s2)]).unwrap();
        assert!(matches!(book.as_of(150), Some(MarginSchedule::CmeSimple { pct_maint, .. }) if (*pct_maint - 0.30).abs() < 1e-12));
        assert!(matches!(book.as_of(250), Some(MarginSchedule::CmeSimple { pct_maint, .. }) if (*pct_maint - 0.40).abs() < 1e-12));
        assert!(book.as_of(50).is_none()); // 段前：无覆盖（不借未来快照）
        assert!(book.as_of(300).is_none()); // 段后（to 不含）
        assert!(book.as_of(200).is_some()); // from 含 → 落段2
    }

    /// fail-loud：非法输入构造期 reject（§2.9）——禁静默退化。
    #[test]
    fn margin_fail_loud_rejects() {
        // 未排序 tiers
        assert!(MarginSchedule::binance_tiered(vec![
            MarginTier { notional_floor: 0.0, mmr: 0.01, maint_amount: 0.0 },
            MarginTier { notional_floor: 0.0, mmr: 0.01, maint_amount: 0.0 },
        ])
        .is_err());
        // mmr 越界
        assert!(MarginSchedule::binance_tiered(vec![MarginTier { notional_floor: 0.0, mmr: 1.5, maint_amount: 0.0 }]).is_err());
        // NaN
        assert!(MarginSchedule::binance_tiered(vec![MarginTier { notional_floor: 0.0, mmr: f64::NAN, maint_amount: 0.0 }]).is_err());
        // 首档不覆盖 0
        assert!(MarginSchedule::binance_tiered(vec![MarginTier { notional_floor: 10.0, mmr: 0.01, maint_amount: 0.0 }]).is_err());
        // cushions 违反 0<B1<B2
        assert!(RiskCushions::new(0.0, 100.0).is_err());
        assert!(RiskCushions::new(200.0, 100.0).is_err());
        assert!(RiskCushions::new(f64::NAN, 100.0).is_err());
        assert!(RiskCushions::new(50.0, 100.0).is_ok());
        // book 段重叠
        let s = MarginSchedule::cme_simple(0.3, 1.0).unwrap();
        assert!(MarginScheduleBook::new(vec![(100, 250, s.clone()), (200, 300, s)]).is_err());
    }

    /// margin_inputs：liq_flag=E≤MM，且填 RiskModeInput 后五态在真实 MM 下可达（§2.4/§3.4）。
    #[test]
    fn margin_inputs_liq_and_reachability() {
        let s = MarginSchedule::cme_simple(0.1, 1.0).unwrap(); // MM=0.1·N
        let cushions = RiskCushions::new(100.0, 300.0).unwrap();
        let net = 10_000.0; // MM=1000
        // E=900<MM=1000 ⟹ liq_flag + Liquidation。
        let ri = margin_inputs(net, 900.0, &s, &cushions);
        assert!((ri.maint_margin - 1000.0).abs() < 1e-9);
        assert!(ri.liq_flag);
        assert_eq!(risk_mode(&ri), RiskMode::Liquidation);
        // E=1050 ∈ [MM,MM+B1)=[1000,1100) ⟹ Deleverage（M2 首次可达）。
        assert_eq!(risk_mode(&margin_inputs(net, 1050.0, &s, &cushions)), RiskMode::Deleverage);
        // E=1200 ∈ [MM+B1,MM+B2)=[1100,1300) ⟹ CloseOnly（M3）。
        assert_eq!(risk_mode(&margin_inputs(net, 1200.0, &s, &cushions)), RiskMode::CloseOnly);
        // E=1400 ≥ MM+B2 ⟹ Normal。
        assert_eq!(risk_mode(&margin_inputs(net, 1400.0, &s, &cushions)), RiskMode::Normal);
    }

    // ── M6 成本模型（CostModel + RDecomposition）──

    /// CostModel fail-loud：负费率 / funding_period=0 / NaN reject（禁静默）。
    #[test]
    fn cost_model_fail_loud() {
        assert!(CostModel::new(-0.01, 8, 0.0, 0.0).is_err()); // 负 funding
        assert!(CostModel::new(0.01, 0, 0.0, 0.0).is_err()); // period=0
        assert!(CostModel::new(0.01, 8, -0.001, 0.0).is_err()); // 负 borrow
        assert!(CostModel::new(0.01, 8, 0.0, f64::NAN).is_err()); // NaN penalty
        assert!(CostModel::new(0.0001, 8, 0.00001, 0.005).is_ok());
    }

    /// funding：仅周期边界 bar 收取，作用于 |净名义|；bar0/非周期 bar ⟹ 0。
    #[test]
    fn cost_funding_periodic() {
        let c = CostModel::new(0.0001, 8, 0.0, 0.0).unwrap();
        // net_notional=$10000，费率 1bp ⟹ 每周期收 $1。
        assert_eq!(c.funding_accrual(0, 10_000.0), 0.0); // bar0 不收
        assert_eq!(c.funding_accrual(4, 10_000.0), 0.0); // 非周期 bar
        assert!((c.funding_accrual(8, 10_000.0) - 1.0).abs() < 1e-12); // 周期边界
        assert!((c.funding_accrual(16, -10_000.0) - 1.0).abs() < 1e-12); // 空头取绝对值
        assert_eq!(c.funding_accrual(8, 0.0), 0.0); // 空仓无 funding
    }

    /// borrow：借入名义 = max(0,|N|−E)；无杠杆/空仓 ⟹ 0；破产态 E≤0 ⟹ 借全额。
    #[test]
    fn cost_borrow_leverage_only() {
        let c = CostModel::new(0.0, 8, 0.00001, 0.0).unwrap();
        // |N|=$15000, E=$10000 ⟹ 借 $5000 × 0.001% = $0.05。
        assert!((c.borrow_accrual(15_000.0, 10_000.0) - 5_000.0 * 0.00001).abs() < 1e-12);
        assert_eq!(c.borrow_accrual(8_000.0, 10_000.0), 0.0); // |N|≤E 无借入
        assert_eq!(c.borrow_accrual(0.0, 10_000.0), 0.0); // 空仓
        // 破产 E≤0 ⟹ 借全额 |N|。
        assert!((c.borrow_accrual(10_000.0, -100.0) - 10_000.0 * 0.00001).abs() < 1e-12);
    }

    /// liquidation penalty：|净名义| × 罚金率；空仓 ⟹ 0。
    #[test]
    fn cost_liquidation_penalty() {
        let c = CostModel::new(0.0, 8, 0.0, 0.005).unwrap();
        assert!((c.liquidation_penalty(20_000.0) - 100.0).abs() < 1e-12); // $20k × 0.5%
        assert_eq!(c.liquidation_penalty(0.0), 0.0);
    }

    /// RDecomposition：net_r = 价格 PnL − 五项成本；守恒残差 = net_r − 账本净变动。
    #[test]
    fn r_decomposition_conservation() {
        // 价格贡献 $1000，费 $30 + funding $5 + borrow $2 + 强平 $10 ⟹ net_r=953；
        // TW 桥对账行 = ⌊5+2+10⌋=17（A10 C5：tw() 高估在险权益的量）。
        let r = RDecomposition::assemble(1000.0, 30.0, 5.0, 2.0, 10.0, 953.0, 17);
        assert!((r.net_r - 953.0).abs() < 1e-9);
        assert!(r.conservation_residual.abs() < 1e-9, "账本一致 ⟹ 残差≈0");
        assert_eq!(r.tw_holding_cost_bridge, 17, "TW 桥对账行 = ⌊funding+borrow+liq⌋ 累计量化");
        // 账本漂移（泄漏）⟹ 残差非零可观测。
        let leak = RDecomposition::assemble(1000.0, 30.0, 5.0, 2.0, 10.0, 900.0, 17);
        assert!((leak.conservation_residual - 53.0).abs() < 1e-9);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  A10 C2：FundingScheduleBook（additive 接口预冻结）+ funding_accrual_signed 有向口径
    // ──────────────────────────────────────────────────────────────────────

    /// ★T-N1（裁定 C2 验收线）：`FundingScheduleBook` 复刻 margin book 测试族——`as_of` 边界
    /// （from 含/to 不含）、未来快照不可见（零前视）、无覆盖段 None、fail-loud 构造拒错族。
    #[test]
    fn a10_funding_book_as_of_zero_lookahead() {
        let book = FundingScheduleBook::new(vec![
            (0, 100, 0.0001),   // 段1：[0,100) rate=+1bp
            (100, 250, -0.0002), // 段2：[100,250) rate=−2bp（带符号 datum）
        ])
        .unwrap();
        // from 含：ts=0 落段1；to 不含：ts=100 不落段1 落段2。
        assert_eq!(book.as_of(0), Some(0.0001), "from 含 ⟹ ts=0 落段1");
        assert_eq!(book.as_of(99), Some(0.0001));
        assert_eq!(book.as_of(100), Some(-0.0002), "to 不含 ⟹ ts=100 落段2（非段1）");
        assert_eq!(book.as_of(249), Some(-0.0002));
        // 未来快照不可见 / 有效域外 None：ts≥250 无覆盖段（不外推未来）；ts<0 无覆盖。
        assert_eq!(book.as_of(250), None, "无覆盖段 ⟹ None（零前视，不借用未来快照）");
        assert_eq!(book.as_of(10_000), None, "远未来 ⟹ None（不外推）");
        assert_eq!(book.as_of(-1), None, "簿起点前 ⟹ None");
        // 带符号费率如实返回（负费率段可读回负值）。
        assert_eq!(book.as_of(150), Some(-0.0002), "signed datum：负费率段读回负值");
    }

    /// ★T-N1 fail-loud 构造拒错族（裁定 C2：空/重叠/乱序/非有限 ⟹ Err，与 margin book 同构）。
    #[test]
    fn a10_funding_book_fail_loud_rejects() {
        // 空簿。
        assert!(FundingScheduleBook::new(vec![]).is_err(), "空簿 ⟹ Err");
        // from>=to。
        assert!(FundingScheduleBook::new(vec![(100, 100, 0.0001)]).is_err(), "from==to ⟹ Err");
        assert!(FundingScheduleBook::new(vec![(200, 100, 0.0001)]).is_err(), "from>to ⟹ Err");
        // 重叠（段2 from < 段1 to）。
        assert!(
            FundingScheduleBook::new(vec![(0, 200, 0.0001), (150, 300, 0.0002)]).is_err(),
            "段重叠 ⟹ Err"
        );
        // 乱序（段2 from < 段1 from——首段违规即被重叠检查捕获的同一机制：from < prev_to）。
        assert!(
            FundingScheduleBook::new(vec![(100, 200, 0.0001), (50, 80, 0.0002)]).is_err(),
            "乱序 ⟹ Err"
        );
        // 非有限费率（NaN / ±∞）。
        assert!(FundingScheduleBook::new(vec![(0, 100, f64::NAN)]).is_err(), "NaN 费率 ⟹ Err");
        assert!(FundingScheduleBook::new(vec![(0, 100, f64::INFINITY)]).is_err(), "∞ 费率 ⟹ Err");
        // 相邻不重叠（from==prev_to）合法 + 带符号费率合法。
        assert!(
            FundingScheduleBook::new(vec![(0, 100, 0.0001), (100, 200, -0.0001)]).is_ok(),
            "相邻不重叠 + 带符号费率 ⟹ Ok"
        );
    }

    /// ★T-N2（裁定 C2 验收线，符号表锚 venue 文档 golden 钉死）：有向 funding——
    /// long×rate>0 **付费**（正成本）、short×rate>0 **收费**（负成本=收入）、空仓恒 0；
    /// 周期边界口径与无向保底一致；无向保底 `funding_accrual` 旧签名/语义回归不变（F2）。
    #[test]
    fn a10_funding_accrual_signed_direction() {
        let c = CostModel::new(0.0001, 8, 0.0, 0.0).unwrap();
        let rate = 0.0001; // venue golden：rate>0 时 long 付 short 收
        // 周期边界 bar8：long（N=+10000）×rate>0 ⟹ +1.0（付费=正成本）。
        assert!((c.funding_accrual_signed(8, 10_000.0, rate) - 1.0).abs() < 1e-12, "long×rate>0 ⟹ 付费（正成本）");
        // short（N=−10000）×rate>0 ⟹ −1.0（收费=负成本=收入）。
        assert!((c.funding_accrual_signed(8, -10_000.0, rate) + 1.0).abs() < 1e-12, "short×rate>0 ⟹ 收费（负成本）");
        // rate<0 镜像翻转：long 收、short 付。
        assert!((c.funding_accrual_signed(8, 10_000.0, -rate) + 1.0).abs() < 1e-12, "long×rate<0 ⟹ 收费");
        assert!((c.funding_accrual_signed(8, -10_000.0, -rate) - 1.0).abs() < 1e-12, "short×rate<0 ⟹ 付费");
        // 空仓恒 0（任意费率/边界）。
        assert_eq!(c.funding_accrual_signed(8, 0.0, rate), 0.0, "空仓恒 0");
        // 周期边界口径：bar0 不收、非周期 bar 不收（与无向保底一致）。
        assert_eq!(c.funding_accrual_signed(0, 10_000.0, rate), 0.0, "bar0 不收");
        assert_eq!(c.funding_accrual_signed(4, 10_000.0, rate), 0.0, "非周期 bar 不收");
        // F2 回归：无向保底旧语义逐字不变——|N|×rate 恒≥0，与有向版在 long+正 rate 下同值。
        assert_eq!(c.funding_accrual(8, -10_000.0), 1.0, "无向保底：空头仍取 |N| 恒正（保守口径不动）");
        assert_eq!(c.funding_accrual(8, 10_000.0), c.funding_accrual_signed(8, 10_000.0, rate),
            "long+正 rate 下有向==无向（保底是有向口径的保守超集）");
        // FundingScheduleBook 配对：as_of 取 rate 喂计提（datum 到位即插的接缝演示）。
        let book = FundingScheduleBook::new(vec![(0, 100, rate), (100, 200, -rate)]).unwrap();
        let r1 = book.as_of(8).unwrap();
        let r2 = book.as_of(120).unwrap();
        assert!((c.funding_accrual_signed(8, 10_000.0, r1) - 1.0).abs() < 1e-12, "book 段1 rate 喂入");
        assert!((c.funding_accrual_signed(8, 10_000.0, r2) + 1.0).abs() < 1e-12, "book 段2 负 rate 喂入");
    }

    /// ★附则B 标签（裁定验收线 T-N9 口径）：费率未标定标签常量钉死逐字值——
    /// datum 注入前一切带成本 R 数值报告强制带它；禁弱化/移除（不回滚条款）。
    #[test]
    fn a10_rate_uncalibrated_label_frozen() {
        assert_eq!(RATE_UNCALIBRATED_LABEL, "[L1机制/费率未标定]", "标签逐字值冻结（090 措辞纪律）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  A10 C4：T-N5 守卫（OQ-5(d) 定理代码锁）——cost_basis<0 禁进强平判据
    // ──────────────────────────────────────────────────────────────────────

    /// ★T-N5（裁定 C4 升强制，OQ-5(d) 已结算定理的代码锁）：**禁把 `cost_basis<0` 编码进
    /// 强平判据/豁免**——强平判据永远读 basis/权益域（>0）；负成本「无风险」是现货命题，
    /// 期货越界（three_stages §4.4:319-331；教义域外 [L0域外]）。
    ///
    /// 构造 stage=EarningShares + 负成本域账本态（cum_net_cash 强负 = 成本基<0 的「成本为0后」
    /// 延伸态），断言 `margin_inputs`/`risk_mode` 判据输出只由 (net_notional, equity, schedule,
    /// cushions) 决定——负成本域账本态在场与否**逐位相同**；且权益跌破 MM 照常触发
    /// Liquidation（**负成本不免强平**）。任何未来把 cost_basis 喂进强平链的改动（margin_inputs
    /// 签名加参 / risk_mode 读账本态）被本测试的期望值击杀。
    #[test]
    fn a10_tn5_negative_cost_basis_never_enters_liquidation_criteria() {
        use crate::theta_v0::strategy::ledger::{TStage, TwState};
        // 负成本域账本态证人：EarningShares + cum_net_cash=−1e6（净现金口径成本基<0，
        // 「现货无风险」命题域）——它**不在** margin_inputs/risk_mode 的输入里（本测试锁死这一点）。
        let neg_cost_witness = TwState {
            stage: TStage::EarningShares,
            cum_net_cash: -1_000_000,
            ..TwState::initial()
        };
        assert!(neg_cost_witness.cum_net_cash < 0, "证人态确在负成本域（构造前置）");
        assert_eq!(neg_cost_witness.stage, TStage::EarningShares, "证人态在 EarningShares（裁定构造要求）");

        let s = MarginSchedule::cme_simple(0.1, 1.0).unwrap(); // MM=0.1·N
        let cushions = RiskCushions::new(100.0, 300.0).unwrap();
        let net = 10_000.0; // MM=1000
        // 同一 (N,E) 网格扫五态全相：判据输出与负成本域无关（margin_inputs 不读任何账本态——
        // 期望值手工推导自 equity/MM/B1/B2，若强平链接入 cost_basis 这些期望值必变 = 击杀）。
        let cases: [(f64, RiskMode); 5] = [
            (1400.0, RiskMode::Normal),       // E ≥ MM+B2
            (1200.0, RiskMode::CloseOnly),    // E ∈ [MM+B1, MM+B2)
            (1050.0, RiskMode::Deleverage),   // E ∈ [MM, MM+B1)
            (900.0, RiskMode::Liquidation),   // E < MM（负成本不免强平的核心域）
            (0.0, RiskMode::Insolvent),       // E ≤ 0
        ];
        for (equity, expected) in cases {
            let ri = margin_inputs(net, equity, &s, &cushions);
            assert_eq!(ri.liq_flag, equity <= ri.maint_margin, "liq_flag 只读 equity≤MM（E={equity}）");
            assert_eq!(
                risk_mode(&ri),
                expected,
                "E={equity}：risk_mode 只读 equity/MM/B1/B2——负成本域账本态在场判据不变（OQ-5(d)）"
            );
        }
        // 显式锚（裁定裁决3）：负成本域 + equity<MM ⟹ 照常 Liquidation——**负成本不免强平**。
        let ri = margin_inputs(net, 900.0, &s, &cushions);
        assert!(ri.liq_flag, "负成本域下 equity≤MM ⟹ liq_flag 照常触发");
        assert_eq!(risk_mode(&ri), RiskMode::Liquidation, "负成本不免强平（OQ-5(d) 贯穿）");
        // 对称锚：负成本域 + equity≤0 ⟹ 照常 Insolvent（破产也不豁免）。
        assert_eq!(risk_mode(&margin_inputs(net, 0.0, &s, &cushions)), RiskMode::Insolvent);
    }

    /// buffer 敏感性网格（§3.5）：buffer 扰动改变 M2/M3 触发边界（暴露 Θ 自由度）。
    #[test]
    fn margin_buffer_sensitivity_grid() {
        let s = MarginSchedule::cme_simple(0.1, 1.0).unwrap();
        let net = 10_000.0; // MM=1000
        let equity = 1150.0; // 固定权益，看 buffer 网格如何改变态
        let modes: Vec<RiskMode> = [(50.0, 100.0), (100.0, 300.0), (200.0, 400.0)]
            .iter()
            .map(|&(b1, b2)| {
                let c = RiskCushions::new(b1, b2).unwrap();
                risk_mode(&margin_inputs(net, equity, &s, &c))
            })
            .collect();
        // E=1150 固定：(50,100)→>MM+100=Normal;(100,300)→∈[1100,1300)=CloseOnly;(200,400)→∈[1000,1200)=Deleverage。
        // buffer 扰动使同一权益落不同态 ⟹ 自由度可观测（存在两格触发态不同）。
        assert!(modes.iter().any(|m| *m != modes[0]), "buffer 网格应暴露不同触发态，实得 {modes:?}");
    }
}
