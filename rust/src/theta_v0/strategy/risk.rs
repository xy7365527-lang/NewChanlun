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
/// - `nav`：账户净值 NAV（账户层状态，Θ 之外的运行时输入）。
/// - `entry`：入场价（整数 tick；>0 必要——分母）。
/// - `stop`：结构止损价（`structural_stop` 产出，整数 tick）。
/// - `cost_per_unit`：每单位成本（commission+slippage+tax 折算，账户/Θ_exec 参数）。
/// - `w_depth`：声部深度资金权重（`voice::depth_weight`，spec:42）。
/// - `parent_cap`：父声部仓位上限（父子比 β 投影，spec:45；β 在调用方按 `parent_qty*β` 算好）。
#[derive(Debug, Clone, Copy)]
pub struct SizingInput {
    pub nav: f64,
    pub entry: Tick,
    pub stop: Tick,
    pub cost_per_unit: f64,
    pub w_depth: f64,
    pub parent_cap: i64,
}

/// sizing：唯一目标手数 qty（bit-exact 对齐 reference-theta-v0.md:47 + Origin.RiskProj）。
///
/// `qty = min(floor(ρ*NAV/(|entry-stop|+κ*cost_per_unit)),
///            floor(w_depth*γ*NAV/entry),
///            parent_cap)`
///
/// 三路上界（风险投影的三个约束，Origin.RiskProj 网格 𝒦 的可行格点上界）：
/// 1. **风险预算**：单声部风险 ρ·NAV 除以每手最大亏损（`|entry-stop| + κ·cost`）——
///    保证单声部亏损 ≤ ρ·NAV（spec:45 单声部风险）。
/// 2. **名义上限**：`w_depth·γ·NAV / entry`——深度资金帽 × 总名义上限 γ 除以入场价
///    （spec:42 资金帽 + spec:45 总名义上限 γ）。
/// 3. **父子约束**：`parent_cap`（父子仓位比 β 投影的上界，spec:45）。
///
/// `qty <= 0 ⟹ 不交易`（返回 0，spec:47）。三路 min 是 Origin.RiskProj `riskproj_exists_unique`
/// 的具体实例：三约束下的可行 qty 集合是 {0,1,…,min三上界} 的有限网格，min 是字典序最小
/// 可行格点上界 = 唯一总仓位（确定选择，无平局）。
///
/// ## bit-exact 浮点约简顺序（spec:47 未钉死括号，本实装固定为下列顺序 [设计选择]）
///
/// spec:47 的公式 `floor(ρ*NAV/(...))` / `floor(w_depth*γ*NAV/entry)` **未明确括号结合序**。
/// 本实装把约简顺序**固定**为（bit-exact 的设计选择，Cargo.toml 已禁浮点重排）：
/// - 项 1 分母：先 `|entry-stop|`（整数 tick → f64），再 `+ κ·cost`（先乘 `κ·cost` 后加）；
///   分子 `ρ·NAV`（先乘）；除后 `floor`。
/// - 项 2：`((w_depth·γ)·NAV) / entry`（**左结合**，先 `w_depth·γ`，再 `·NAV`，再 `/entry`），
///   后 `floor`。
///
/// 这个左结合顺序是**确定选择**（非 spec 推论）——同一 Θ + 输入恒产出同一 qty（bit-exact）。
/// 若需与其他实装（如 Lean fixture 或 Python 参考）对齐到 bit，须共享此括号约定。
///
/// 边界条件：
/// - `entry <= 0` ⟹ 项 2 分母非正，返回 0（不交易；入场价须为正 tick）。
/// - 项 1 分母 `|entry-stop| + κ·cost = 0`（entry=stop 且 cost=0）⟹ 风险预算无界，
///   项 1 不约束（取 i64::MAX），由项 2/3 约束。
/// - `default_lot`（spec:47）：qty>0 时按 lot 取整——qty 向下取整到 lot 的整数倍
///   （`default_lot=1` 时无影响）。
pub fn size_position(input: &SizingInput, config: &RiskConfig) -> i64 {
    if input.entry <= 0 {
        return 0;
    }

    // ── 项 1：风险预算上界 floor(ρ*NAV / (|entry-stop| + κ*cost_per_unit)) ──
    let risk_dist = (input.entry - input.stop).abs() as f64; // |entry-stop|，整数 tick → f64
    let denom1 = risk_dist + config.kappa * input.cost_per_unit; // 先乘 κ·cost 后加
    let bound1: i64 = if denom1 <= 0.0 {
        // 分母非正（entry=stop 且 cost=0）：风险预算不约束。
        i64::MAX
    } else {
        let numer1 = config.rho * input.nav; // ρ·NAV
        floor_nonneg(numer1 / denom1)
    };

    // ── 项 2：名义上限 floor(w_depth*γ*NAV / entry) ──
    let numer2 = input.w_depth * config.gamma * input.nav; // (w_depth·γ)·NAV，左结合
    let bound2: i64 = floor_nonneg(numer2 / (input.entry as f64));

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
        // NAV=1_000_000, entry=100, stop=90 (|d|=10), cost=0。
        // 项1 = floor(0.005*1e6 / (10 + 2*0)) = floor(5000/10) = 500。
        // 项2 = floor(0.6*1.0*1e6 / 100) = floor(6000) = 6000。
        // 项3 = MAX。 min = 500。
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 90,
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
        // entry=100, stop=99 (|d|=1) ⟹ 项1 = floor(5000/1)=5000；
        // 项2 = floor(0.6*1e6/100)=6000... 让项2更紧：用小 w_depth。
        // w_depth=0.1: 项2 = floor(0.1*1e6/100)=1000；项1=5000 ⟹ min=1000。
        let inp = SizingInput {
            nav: 1_000_000.0,
            entry: 100,
            stop: 99,
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
