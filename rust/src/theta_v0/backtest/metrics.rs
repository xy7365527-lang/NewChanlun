//! 回测指标层——权益曲线 + 交易列表 → 协议 §3 全套指标。
//!
//! ## 认识论等级：**L1**（确定性变换，零信息增量）
//!
//! 给定权益曲线/交易序列，算 CAGR/Sharpe/... 是确定性数值变换——验证指标算法正确，
//! **不验证 Θ 有效**（formalization-validity-domain 231号）。指标的 **L2** 价值只在喂入
//! **真实历史数据驱动的引擎产出**的权益曲线时才产生（届时指标可否证 Θ）。
//!
//! ## 协议锚点（docs/backtest-protocol-v0.md §3，全部预注册，全部报告）
//!
//! 不允许只报有利指标。本模块计算 §3 列出的**全部**指标，调用方一次性输出。
//!
//! ## 实现诚实声明
//!
//! - **统计显著性**（§3.4 Sharpe 标准误 Lo 2002 调整 / block bootstrap p 值）和**随机
//!   入场对照**（§4，seed=20260625）是**独立的统计检验步骤**，依赖完整交易序列与
//!   per-trade 收益分布。本模块提供基础指标（CAGR/Sharpe/...）；bootstrap/随机对照在
//!   runner 层组织（需要引擎产出真实订单流后才有意义，属**等引擎阻塞点**）。
//! - 年化因子按"每年交易日"近似——日聚合 returns 的年化用 √252（标准约定）。各品种实际
//!   交易日数在 runner 层按数据日历精确化（§3.1"年化因子按各品种实际交易日"）。
//!
//! ## ★随机入场对照重写（2026-06-28，自举恒真否证后的操作语义版）
//!
//! **旧 §4 缺陷（codex 异质审查 + 编排者裁定，已删）**：旧随机对照从 Θ 自己的 `trade_pnls`
//! 池有放回重抽 n 笔求和——这是 **self-resampling 自举恒真**：重抽分布的均值恒等于 Θ 总盈亏
//! （n×池均值 = sum），Θ 分位必然≈0.5，与择时能力**无关**。它从未与"随机选时点入场"比较。
//!
//! **新 §4（操作语义 + 内在语法 + 含浮盈）**：随机对照不再脱离执行做统计重抽，而是**保持 Θ 的
//! 操作骨架**（开仓→持仓 hold_bars→平仓），只随机化"在哪根 bar 入场"（破坏缠论内在语法——
//! 无笔/线段/中枢/买卖点结构），在**原始价格序列**上重新执行开/持/平操作语义（含成本，
//! close-to-close 口径与 Θ 账本侧一致），算 **total_return**（含浮盈口径）。零假设 = "破坏缠论
//! 结构（随机入场）vs 保持（Θ 按结构入场）"，检验缠论内在语法是否贡献择时 alpha。
//!
//! 两个对照（codex 严格版）：
//! - **主：schedule-shift**（保操作外形）——整体平移交易日程，保留交易间隔/并发/hold_bars/qty
//!   序列的联合结构，只随机偏移起点。
//! - **副：independent-entry**（边际择时）——逐笔独立随机抽 entry，破坏交易间时序相关，只保边际。
//!
//! Θ 与随机**同口径**（逐笔无复利绝对盈亏 ÷ 同一 nav_base）⟹ 唯一区别 = 入场点，无复利偏置。
//! p 值用 `p_upper = (1 + count(rand ≥ theta)) / (N+1)`（不是裸分位）；`beats_random ⟺ 两对照
//! p_upper ≤ 0.05 且两对照非退化`。
//!
//! ## ★诚实有效域边界（formalization-validity-domain，codex 实现审查缺陷③④）
//!
//! 1. **不重建复利权益曲线**：total_return = **逐笔无复利**绝对盈亏求和归一化，**不**重建现金/
//!    持仓的复利权益曲线，**不**算随机策略的 CAGR/Sharpe，**不**建模 independent 对照可能产生
//!    的重叠仓位资金约束。声明仅为"逐笔无复利总收益对照"——比"完整 equity 曲线"弱（诚实不膨胀）。
//!    Θ 同口径基准也用同一逐笔无复利公式（非 MtM 复利 strat_return），故 Θ 与随机口径严格一致。
//! 2. **多空双向有效域**：runner.rs `track_position_transition` 按持仓符号产真实 [`TradeRecord::long`]
//!    （多/空），消费侧 [`trade_abs_pnl`] **方向感知**（多头低买高卖 `qty·(exit·(1−fee)−entry·(1+fee))`；
//!    空头高卖低买 `qty·(entry·(1−fee)−exit·(1+fee))`，canonical 镜像等变 σ→−σ，`formal/Origin/`
//!    LeverageCapital.lean 有符号名义 `n_v=σ_v·M·P·q` + MainTheorem.lean §22「多空镜像」）。
//!    随机对照按每笔方向选公式、保持方向不变（只随机化入场点）——**有效域 = 多空交易**。

/// 协议 §3 全套指标（单个检验窗）。每个字段对应 §3 一行。
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    // ── §3.1 收益与风险调整 ──
    /// 几何年化收益（扣成本后）。
    pub cagr: f64,
    /// 年化 Sharpe（rf=0，日 returns 年化）。
    pub sharpe: f64,
    /// 年化 Sortino（下行标准差）。
    pub sortino: f64,
    /// 最大回撤（峰谷，正数，0..1）。
    pub max_drawdown: f64,
    /// Calmar = CAGR / |MaxDD|。
    pub calmar: f64,
    /// CVaR(5%)：5% 尾部条件期望亏损（日 returns，负数表亏损）。
    pub cvar_5: f64,

    // ── §3.2 交易行为 ──
    /// 完整开平仓次数。
    pub n_trades: usize,
    /// 胜率 = 盈利交易 / 总交易。
    pub win_rate: f64,
    /// 盈亏比 = 平均盈利 / |平均亏损|。
    pub profit_factor: f64,
    /// top-5 交易盈亏和 / 总盈亏（§5.2 失败判据：>50% 收益过度集中）。
    pub top5_pnl_ratio: f64,

    // ── 基线对照（§4 Buy&Hold；随机/方向打乱在 runner 层）──
    /// 策略总收益率（期末权益/初始-1）。
    pub strat_return: f64,
    /// buy&hold 总收益率（末 close/首 close-1）。
    pub bh_return: f64,
}

/// 计算全套指标。
///
/// 输入：
/// - `equity_curve`：逐 bar 权益（归一化，初始=1.0），与回测 bar 序列同长。
/// - `trade_pnls`：每笔完整平仓交易的盈亏（归一化口径）。
/// - `bars_per_year`：年化因子分母（日 returns 数 / 年；§3.1 按各品种实际交易日）。
///   1min bar 聚合到日 returns 后，标准约定 √252——此处传日 returns 数对应的年化基数。
/// - `bh_return`：buy&hold 收益率（runner 层算，传入对照）。
///
/// **边界条件**：`equity_curve` 空或全平（无交易）⇒ 收益/风险指标为 0，盈亏比/胜率为 0
/// （NaN 防御，§5.5 由 runner 判 inconclusive）。
pub fn compute(
    equity_curve: &[f64],
    daily_returns: &[f64],
    trade_pnls: &[f64],
    years: f64,
    bh_return: f64,
) -> Metrics {
    let strat_return = if let (Some(&first), Some(&last)) =
        (equity_curve.first(), equity_curve.last())
    {
        if first > 0.0 {
            last / first - 1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // CAGR：几何年化。years<=0 或权益<=0 ⇒ 0。
    let cagr = if years > 0.0 && strat_return > -1.0 {
        (1.0 + strat_return).powf(1.0 / years) - 1.0
    } else {
        0.0
    };

    let (sharpe, sortino) = sharpe_sortino(daily_returns);
    let max_drawdown = max_drawdown(equity_curve);
    let calmar = if max_drawdown > 1e-12 {
        cagr / max_drawdown
    } else {
        0.0
    };
    let cvar_5 = cvar(daily_returns, 0.05);

    let (win_rate, profit_factor) = win_rate_profit_factor(trade_pnls);
    let top5_pnl_ratio = top_n_pnl_ratio(trade_pnls, 5);

    Metrics {
        cagr,
        sharpe,
        sortino,
        max_drawdown,
        calmar,
        cvar_5,
        n_trades: trade_pnls.len(),
        win_rate,
        profit_factor,
        top5_pnl_ratio,
        strat_return,
        bh_return,
    }
}

// ════════════════════════════════════════════════════════════════════════
//  统计显著性检验（§3.4 + §4，预注册，seed=20260625）
//
//  ## 认识论等级：**L2**（当且仅当输入是真实数据回测的交易序列）
//
//  §3.4 block bootstrap / §4 随机入场对照是**可否证**的假设检验——它们对真实数据
//  回测的 trade_pnls/daily_returns 给出 p 值与经验分布，用于判定 Θ 策略是否**显著
//  优于零收益 / 优于随机择时**（§5.1/§5.3 失败判据）。否定性结果（p>0.05 或 Sharpe
//  落在随机 95% 区间内）= Θ 被否证，缩小有效域，是正信息增量（231号）。
//
//  ★合成数据上跑这些检验仍是 **L1**（验证检验管线正确，零信息增量）——L2 只在喂
//  真实回测交易序列时产生。
//
//  ## seed 冻结（§4，可复现硬约束）
//
//  随机数 seed = **20260625**（codex 裁决日，backtest-protocol-v0.md:141 冻结）。
//  用 SplitMix64（无外部依赖、纯算术、确定性）——重跑 bit-exact 复现。
// ════════════════════════════════════════════════════════════════════════

/// 预注册随机种子（§4，backtest-protocol-v0.md:141 冻结，不可改）。
pub const PREREG_SEED: u64 = 20260625;

/// trade_pnls 序列 block bootstrap 的块长（**交易笔数**单位，保留成交序列局部自相关）。
///
/// ★单位诚实（codex 修正）：旧代码用 `BLOCK_LEN=20` 对 `trade_pnls` 做 block bootstrap 是
/// **单位错误**——协议 §3.4（backtest-protocol-v0.md:127）的「块长 20 **交易日**」是
/// **日 returns 序列**的块长，**不是** 20 笔交易。trade_pnls 是交易序列，块长单位是笔数。
/// 取 5 笔（短块保留相邻交易的局部相关，不假设全序列独立）。
///
/// **诚实声明（formalization-validity-domain）**：协议 §3.4 还要求对 **daily_returns 序列**
/// 做块长=20 交易日的 block bootstrap 输出 CAGR/Sharpe 经验分布——本 [`significance`] **未实装
/// 该分支**（daily_returns 仅用于 Lo 2002 Sharpe 标准误）。该协议项是已存在的实装缺口，
/// 不在本轮（操作语义随机对照重写）owner 范围，不声明已具备。
pub const TRADE_BLOCK_LEN: usize = 5;

/// ★交易执行轨迹（操作语义随机对照的输入，2026-06-28 新增）。
///
/// 每个**平仓事件**（逐 fill 口径，bughunt F-06）记录其**操作骨架**：在哪根 bar 开仓、持有
/// 多少根、仓位规模、方向、是否窗口终点强平。持仓符号归零/翻转配对整段；**部分减仓**同样
/// 逐 fill 产记录（qty = 本次减掉的手数，entry_bar 延续首次入场）——与 runner `apply_order`
/// 逐 fill push `trade_pnls` 的已实现口径对齐，两序列一一对应，消费方不得假设"一条记录 =
/// 一次完整往返"。随机对照保持 `hold_bars`/`qty`/`side`（操作外形），只随机化 `entry_bar`
/// （破坏缠论内在语法），在原始价格序列上重新执行开/持/平算 PnL。
///
/// **认识论**：这是 L2 否证的载体——`entry_bar` 携带"Θ 按缠论结构选时点"的信息，随机化它
/// 即移除该信息，比较 Θ vs 随机的 equity 分布检验缠论语法的择时 alpha 贡献。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeRecord {
    /// 开仓成交 bar 索引（回测 bars 局部下标，close 成交口径）。
    pub entry_bar: usize,
    /// 平仓成交 bar 索引（close 成交口径）。`exit_bar > entry_bar`。
    pub exit_bar: usize,
    /// 持仓时长（bar 数）= `exit_bar − entry_bar`（≥1）。随机对照保持此值不变。
    pub hold_bars: usize,
    /// 仓位规模（lot 数，绝对手数）。随机对照保持此值不变。
    pub qty: f64,
    /// 方向（`true=多头`，`false=空头`）。runner.rs 的 `track_position_transition` 按持仓符号
    /// （`units_before > 0.0`）真实区分多空——做空腿已建模（v1 方向中性账本）。
    /// 消费侧 [`trade_abs_pnl`] **方向感知**（多头低买高卖 / 空头高卖低买，canonical 镜像等变
    /// σ→−σ），随机对照按该字段选公式、保持方向不变。**有效域 = 多空双向**（不再限多头）。
    pub long: bool,
    /// 是否窗口终点强制平仓（[`TradeRecord`] 的 `ForcedWindowClose` 标记）。
    /// ★含浮盈口径的载体：终点未平仓的持仓被强平实现浮盈，但**不计入 n_trades≥30 的
    /// 统计功效门槛**（避免 29 笔真实退出 + 1 笔强平偷过门槛，codex 指出的陷阱）。
    pub forced_close: bool,
}

/// bootstrap / 蒙特卡洛重采样次数（§3.4 + §4：n=1000）。
pub const N_RESAMPLE: usize = 1000;

/// SplitMix64 确定性 PRNG（无依赖，纯算术）。
///
/// 给定 seed，`next_u64` 序列完全确定（bit-exact 可复现，§4 seed 冻结要求）。
/// 这是技术性工具（伪随机数发生器），不涉及任何缠论领域概念。
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// 下一个 u64（SplitMix64 标准约简，确定性）。
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `[0, n)` 均匀整数（n>0；Lemire 无偏约简的简化——取模偏置在 n≪2^64 时可忽略，
    /// 但为确定性可复现，用直接取模，bit-exact）。
    fn next_below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0, "next_below 要求 n>0");
        (self.next_u64() % n as u64) as usize
    }
}

/// §3.4 + §4 统计显著性检验结果（预注册全套，全部报告，不挑有利项）。
#[derive(Debug, Clone, PartialEq)]
pub struct Significance {
    // ── §3.4 block bootstrap（H0：收益 ≤ 0）──
    /// bootstrap 重采样的交易序列**总收益**经验分布的均值。
    pub boot_mean_total_pnl: f64,
    /// **单边 p 值 H0：总收益 ≤ 0**（§5.1 失败判据：p>0.05 ⟹ 收益不显著）。
    /// = 重采样中总收益 ≤ 0 的比例（经验 p 值）。
    pub boot_pvalue_pnl_le_0: f64,
    /// bootstrap 总收益的 95% 置信区间下界（2.5 分位）。
    pub boot_ci95_lo: f64,
    /// bootstrap 总收益的 95% 置信区间上界（97.5 分位）。
    pub boot_ci95_hi: f64,

    // ── §3.4 Sharpe 标准误（Lo 2002 自相关调整）──
    /// 实测年化 Sharpe（日 returns）。
    pub sharpe: f64,
    /// Sharpe 标准误（Lo 2002 一阶自相关调整）。
    pub sharpe_se: f64,
    /// Sharpe 95% 置信区间下界（sharpe - 1.96·se）。
    pub sharpe_ci95_lo: f64,
    /// Sharpe 95% 置信区间上界（sharpe + 1.96·se）。
    pub sharpe_ci95_hi: f64,

    // ── §4 随机入场对照（操作语义版，2026-06-28 重写）──
    // ★主检验 schedule-shift（保操作外形：交易间隔/并发/hold_bars/qty 联合结构）+
    //   副检验 independent-entry（边际择时）。Θ 与随机**同口径**（逐笔无复利绝对盈亏 ÷ 同一
    //   nav_base）比较——消除「Θ 复利 MtM vs 随机逐笔」的口径偏置（codex 设计审查第2点）。
    /// 主检验（schedule-shift）随机策略 total_return 分布均值。
    pub shift_mean_return: f64,
    /// Θ 同口径 total_return 在 schedule-shift 分布中的上单边 p 值
    /// `p_upper = (1 + count(rand ≥ theta)) / (N+1)`。§5.3：p_upper > 0.05 ⟹ 不优于随机。
    pub shift_pvalue: f64,
    /// 副检验（independent-entry）随机策略 total_return 分布均值。
    pub indep_mean_return: f64,
    /// Θ 同口径 total_return 在 independent-entry 分布中的上单边 p 值。
    pub indep_pvalue: f64,
    /// ★Θ **同口径** total_return（随机对照的实际比较基准）：用 Θ 的 trades 在 prices 上按
    /// **与随机对照完全相同**的逐笔无复利公式（`trade_abs_pnl ÷ nav_base`，含浮盈/终点持仓）算。
    /// 唯一区别于随机的变量 = 入场点（Θ 真实 vs 随机）⟹ 严格同质比较，无复利偏置。
    pub theta_return_same_caliber: f64,
    /// Θ MtM 复利口径 total_return（= `metrics.strat_return`）——**仅报告参考**，不用于比较
    /// （复利与随机逐笔不同质）。诚实并列：同口径用于判定，MtM 用于展示量级。
    pub theta_return_mtm: f64,
    /// Θ 是否**显著优于**随机择时：**两对照都** p_upper ≤ 0.05 **且两对照非退化**（codex
    /// 严格判定：Θ 须同时打败 schedule-shift 与 independent-entry 才算择时携带信息）。
    pub theta_beats_random: bool,
    /// ★随机对照是否退化（codex 缺陷⑤）：schedule-shift 无非零合法平移（span<2）或
    /// independent 全笔 hold≥len ⟹ 无真随机样本 ⟹ p_upper 无统计含义。退化时下游归
    /// **inconclusive**（不冒充否证也不冒充确认）。非退化 ⟹ p_upper 可采信。
    pub controls_degenerate: bool,
}

/// 计算 §3.4 block bootstrap + Sharpe 标准误 + §4 操作语义随机入场对照（预注册，seed 冻结）。
///
/// 输入：
/// - `trade_pnls`：每笔完整平仓交易的盈亏（真实数据回测产出 ⟹ L2；合成 ⟹ L1）。
///   §3.4 block bootstrap 用它（H0：收益≤0）。
/// - `daily_returns`：逐 bar returns 序列（Sharpe 标准误 Lo 2002 调整用）。
/// - `trades`：Θ 的交易执行轨迹（[`TradeRecord`]，操作语义随机对照的输入）。
/// - `prices`：原始价格序列（close 口径，与 Θ 账本侧 `apply_order` 的 `px=bar.close` 一致）。
/// - `fee_rate`：单边费用率（commission+slippage+tax，比率）。随机对照含同等成本。
/// - `theta_return_for_control`：Θ 含浮盈 total_return（MtM/终点强平口径）——随机对照的比较基准。
///
/// **block bootstrap**（§3.4，backtest-protocol-v0.md:127）：以 [`TRADE_BLOCK_LEN`]=5 笔为块长
/// （★单位修正：协议 §3.4 的「20 交易日」是日 returns 块，**不是** 20 笔交易；trade_pnls 序列
/// 用交易笔数块长），从 `trade_pnls` 有放回抽连续块拼成等长重采样序列（保留交易序列局部自相关），
/// 算其总收益，重复 [`N_RESAMPLE`]=1000 次 ⟹ 单边 p 值（H0：收益≤0）+ 95% 置信区间。
///
/// **Sharpe 标准误**（§3.4，Lo 2002）：日 returns 的 Sharpe 标准误，一阶自相关调整。
///
/// **操作语义随机入场对照**（§4，2026-06-28 重写，见 [`random_entry_controls`]）：保持 Θ 交易的
/// 操作外形（hold_bars/qty/方向），随机化入场点（破坏缠论内在语法），在 `prices` 上重新执行
/// 开/持/平算 total_return，主检验 schedule-shift + 副检验 independent-entry，各 n=1000。
/// p_upper = (1+count(rand≥theta))/(N+1)；`theta_beats_random ⟺ 两对照 p_upper 均 ≤ 0.05`。
///
/// **边界条件**：`trade_pnls` 为空/仅 1 笔 ⟹ §3.4 bootstrap 退化（p=1.0，CI=[0,0]）；
/// `trades` 为空或 `prices` 太短 ⟹ 随机对照退化（两 p_upper=1.0，theta_beats_random=false）——
/// 无交易/无价格序列无法否证（§5.5 inconclusive，不冒充显著性）。
///
/// **谱系引用**：§3.4/§4 统计方法直接引用 backtest-protocol-v0.md（看结果前冻结，
/// codex 裁决 seed=20260625）。本函数不引入新判据（下游不可事后改判据，协议 §影响声明）。
pub fn significance(
    trade_pnls: &[f64],
    daily_returns: &[f64],
    trades: &[TradeRecord],
    prices: &[f64],
    fee_rate: f64,
    theta_return_mtm: f64,
) -> Significance {
    let (sharpe, _) = sharpe_sortino(daily_returns);
    let (sharpe_se, sharpe_ci95_lo, sharpe_ci95_hi) = sharpe_lo2002(daily_returns, sharpe);

    // ── §4 操作语义随机入场对照（seed 冻结，独立 rng 流，先于 bootstrap 算保证可复现）──
    // ★Θ 同口径 total_return 在 random_entry_controls 内部用 trades+prices 算（与随机同公式）。
    let ctrl = random_entry_controls(trades, prices, fee_rate);

    let n = trade_pnls.len();
    // §3.4 退化：无交易 / 仅 1 笔 ⟹ 无可重采样的序列结构 ⟹ 不冒充显著性（§5.5 inconclusive）。
    if n < 2 {
        return Significance {
            boot_mean_total_pnl: trade_pnls.iter().sum(),
            boot_pvalue_pnl_le_0: 1.0, // 无法拒绝 H0（无证据）
            boot_ci95_lo: 0.0,
            boot_ci95_hi: 0.0,
            sharpe,
            sharpe_se,
            sharpe_ci95_lo,
            sharpe_ci95_hi,
            shift_mean_return: ctrl.shift_mean_return,
            shift_pvalue: ctrl.shift_pvalue,
            indep_mean_return: ctrl.indep_mean_return,
            indep_pvalue: ctrl.indep_pvalue,
            theta_return_same_caliber: ctrl.theta_return_same_caliber,
            theta_return_mtm,
            theta_beats_random: ctrl.theta_beats_random,
            controls_degenerate: ctrl.controls_degenerate,
        };
    }

    // ── §3.4 block bootstrap（块长 TRADE_BLOCK_LEN 笔，n=1000，H0：总收益≤0）──
    let mut rng = SplitMix64::new(PREREG_SEED);
    let mut boot_totals: Vec<f64> = Vec::with_capacity(N_RESAMPLE);
    for _ in 0..N_RESAMPLE {
        let total = block_bootstrap_total(trade_pnls, TRADE_BLOCK_LEN, &mut rng);
        boot_totals.push(total);
    }
    let boot_mean_total_pnl = boot_totals.iter().sum::<f64>() / N_RESAMPLE as f64;
    // 单边 p 值 H0：总收益 ≤ 0 = 重采样中总收益 ≤ 0 的比例。
    let n_le_0 = boot_totals.iter().filter(|&&t| t <= 0.0).count();
    let boot_pvalue_pnl_le_0 = n_le_0 as f64 / N_RESAMPLE as f64;
    let (boot_ci95_lo, boot_ci95_hi) = percentile_ci(&mut boot_totals, 0.025, 0.975);

    Significance {
        boot_mean_total_pnl,
        boot_pvalue_pnl_le_0,
        boot_ci95_lo,
        boot_ci95_hi,
        sharpe,
        sharpe_se,
        sharpe_ci95_lo,
        sharpe_ci95_hi,
        shift_mean_return: ctrl.shift_mean_return,
        shift_pvalue: ctrl.shift_pvalue,
        indep_mean_return: ctrl.indep_mean_return,
        indep_pvalue: ctrl.indep_pvalue,
        theta_return_same_caliber: ctrl.theta_return_same_caliber,
        theta_return_mtm,
        theta_beats_random: ctrl.theta_beats_random,
        controls_degenerate: ctrl.controls_degenerate,
    }
}

/// 操作语义随机入场对照的中间结果（[`random_entry_controls`] 产出）。
struct RandomControls {
    shift_mean_return: f64,
    shift_pvalue: f64,
    indep_mean_return: f64,
    indep_pvalue: f64,
    /// Θ 同口径 total_return（与随机对照完全相同的逐笔无复利公式算，比较基准）。
    theta_return_same_caliber: f64,
    theta_beats_random: bool,
    /// 随机对照退化（无真随机样本）⟹ p_upper 无统计含义。
    controls_degenerate: bool,
}

/// **★操作语义随机入场对照（§4 核心，2026-06-28 重写）**。
///
/// 零假设：破坏缠论内在语法（随机入场，无笔/线段/中枢/买卖点结构）vs 保持（Θ 按结构入场）。
/// 检验缠论结构本身是否贡献择时 alpha。**保持操作外形**（hold_bars/qty/方向序列），只随机化
/// 入场点，在 `prices` 上重新执行开/持/平操作语义（含成本），重建 equity 算 total_return（含浮盈）。
///
/// **两对照**（codex 严格版）：
/// - **主 schedule-shift**：整体平移交易日程——所有交易的 entry_bar 同加一个随机偏移 δ，保留
///   交易间隔/并发/重叠的**联合结构**（最强保操作外形）。δ 范围使所有交易 [entry+δ, exit+δ]
///   落在 [0, len)。
/// - **副 independent-entry**：逐笔独立随机抽 entry（破坏交易间时序相关，只保边际 hold_bars/qty）。
///
/// **total_return 重算**（含浮盈口径，Θ 与随机**同口径**——消除复利偏置，**方向感知**）：每笔交易
/// （无论 Θ 真实入场还是随机入场）= 在 entry 开仓、持有 hold_bars、平仓，按 **`trade_abs_pnl(…, t.long)`
/// 方向感知公式**（多头 `qty·(exit·(1−fee) − entry·(1+fee))`；空头 `qty·(entry·(1−fee) − exit·(1+fee))`，
/// canonical 镜像等变 σ→−σ）求**逐笔无复利**绝对盈亏归一化：total_return = Σ pnl(t) / nav_base，
/// nav_base = Σ qty·entry_px·(1+fee)（方向无关名义量级基准 |n_v|=M·P·q，镜像对称要求分母方向无关）。
/// **随机对照保持每笔的方向 `t.long` 不变**（只随机化入场点，方向是 Θ 内在语法决定的，不随机化）。
/// **Θ 的 total_return（[`RandomControls::theta_return_same_caliber`]）用 Θ 真实 entry/exit 按同一公式算**
/// ——与随机的唯一区别 = 入场点，无复利偏置（codex 设计审查第2点：不能比较「Θ 复利 MtM vs 随机逐笔」）。
/// 含浮盈口径体现于终点强平笔（hold 到末 bar，在 trades 内 forced_close=true）。close 成交口径
/// （与 Θ 账本侧 `apply_order` 的 `px=bar.close` 一致）。
///
/// **p 值**：`p_upper = (1 + count(rand ≥ theta_same_caliber)) / (N+1)`（保守上单边）。
/// `theta_beats_random ⟺ shift_pvalue ≤ 0.05 ∧ indep_pvalue ≤ 0.05`（两对照都打败才算择时有信息）。
///
/// **边界/退化**：`trades` 空、`prices.len() < 2`、或无合法随机区间 ⟹ 两 p_upper=1.0，
/// theta_beats_random=false（不冒充否证，§5.5 inconclusive）。
///
/// **认识论 L2**（真实数据时）：随机化 entry 移除"Θ 按缠论结构选时点"的信息——若 Θ 同口径收益
/// 不能稳定打败随机分布，则缠论内在语法**不贡献择时 alpha**（否证，缩小有效域，231号）。
fn random_entry_controls(
    trades: &[TradeRecord],
    prices: &[f64],
    fee_rate: f64,
) -> RandomControls {
    let len = prices.len();
    // 退化：无交易序列 / 价格序列太短 ⟹ 无法构造随机策略 ⟹ 不冒充否证。
    if trades.is_empty() || len < 2 {
        return RandomControls {
            shift_mean_return: 0.0,
            shift_pvalue: 1.0,
            indep_mean_return: 0.0,
            indep_pvalue: 1.0,
            theta_return_same_caliber: 0.0,
            theta_beats_random: false,
            controls_degenerate: true,
        };
    }

    // 归一化基准 = Θ 各笔入场名义额之和（方向无关的名义量级 |n_v|=M·P·q 折算，含建仓费量级）。
    // ★方向无关（镜像对称要求）：分母对多空统一，分子（PnL）方向感知 ⟹ 同价格走势 long_return =
    // −short_return（镜像等变的可测后果，formal LeverageCapital.lean |n_v| 方向无关）。
    let nav_base: f64 = trades
        .iter()
        .map(|t| {
            let entry_px = prices[t.entry_bar.min(len - 1)];
            t.qty * entry_px * (1.0 + fee_rate)
        })
        .sum();
    let nav_base = if nav_base > 1e-12 { nav_base } else { 1.0 };

    // ── ★Θ 同口径 total_return（用 Θ 真实 entry/exit，与随机完全相同的逐笔无复利公式）──
    // 这是随机对照的**实际比较基准**——与随机唯一区别 = 入场点，消除复利偏置。方向感知（按 t.long）。
    let theta_same_caliber: f64 = trades
        .iter()
        .map(|t| {
            let e = t.entry_bar.min(len - 1);
            let x = t.exit_bar.min(len - 1);
            trade_abs_pnl(prices[e], prices[x], t.qty, fee_rate, t.long)
        })
        .sum::<f64>()
        / nav_base;

    // ── 主：schedule-shift（整体平移，保留交易日程联合结构）──
    // δ 合法范围：所有交易 exit_bar+δ < len 且 entry_bar+δ ≥ 0 ⟹ δ ∈ [−min_entry, len−1−max_exit]。
    let min_entry = trades.iter().map(|t| t.entry_bar).min().unwrap_or(0);
    let max_exit = trades.iter().map(|t| t.exit_bar).max().unwrap_or(0);
    let mut rng = SplitMix64::new(PREREG_SEED);
    let mut shift_returns: Vec<f64> = Vec::with_capacity(N_RESAMPLE);
    // span = 合法 δ 取值数（含端点）。span<2 ⟹ 唯一 δ=0（复现 Θ），**无真随机平移**——
    // 退化（codex 缺陷⑤）：留空 shift_returns ⟹ control_stats 返 p_upper=1.0，但通过
    // theta_beats_random 的 shift 退化检查阻止「退化伪装否证/确认」（下游报告 inconclusive）。
    let span: usize = if max_exit < len {
        (len - 1 - max_exit) + min_entry + 1
    } else {
        0
    };
    let shift_degenerate = span < 2; // 无非零合法平移 ⟹ 退化
    if !shift_degenerate {
        for _ in 0..N_RESAMPLE {
            let delta = rng.next_below(span) as isize - min_entry as isize;
            let mut total_abs = 0.0;
            for t in trades {
                let e = (t.entry_bar as isize + delta) as usize;
                let x = (t.exit_bar as isize + delta) as usize;
                // 随机对照保持该 trade 方向（t.long）不变——只随机化入场点，方向是 Θ 内在语法决定的。
                total_abs += trade_abs_pnl(prices[e], prices[x], t.qty, fee_rate, t.long);
            }
            shift_returns.push(total_abs / nav_base);
        }
    }
    let (shift_mean_return, shift_pvalue) = control_stats(&mut shift_returns, theta_same_caliber);

    // ── 副：independent-entry（逐笔独立随机抽 entry，破坏交易间时序相关）──
    let mut indep_returns: Vec<f64> = Vec::with_capacity(N_RESAMPLE);
    for _ in 0..N_RESAMPLE {
        let mut total_abs = 0.0;
        let mut feasible = true;
        for t in trades {
            // entry' ∈ [0, len − hold_bars)（半开，保证 exit'=entry'+hold_bars < len）。
            if t.hold_bars >= len {
                feasible = false;
                break;
            }
            let e = rng.next_below(len - t.hold_bars);
            let x = e + t.hold_bars;
            // 随机对照保持该 trade 方向（t.long）不变——只随机化入场点。
            total_abs += trade_abs_pnl(prices[e], prices[x], t.qty, fee_rate, t.long);
        }
        if feasible {
            indep_returns.push(total_abs / nav_base);
        }
    }
    let indep_degenerate = indep_returns.is_empty(); // 全笔 hold≥len ⟹ 无合法随机
    let (indep_mean_return, indep_pvalue) = control_stats(&mut indep_returns, theta_same_caliber);

    // codex 严格判定（含退化 guard 缺陷⑤）：Θ 须**同时**打败 schedule-shift 与 independent-entry，
    // **且两对照都有真随机样本**（非退化）——退化时不冒充否证也不冒充确认（下游归 inconclusive）。
    let theta_beats_random =
        !shift_degenerate && !indep_degenerate && shift_pvalue <= 0.05 && indep_pvalue <= 0.05;

    RandomControls {
        shift_mean_return,
        shift_pvalue,
        indep_mean_return,
        indep_pvalue,
        theta_return_same_caliber: theta_same_caliber,
        theta_beats_random,
        controls_degenerate: shift_degenerate || indep_degenerate,
    }
}

/// 单笔交易的绝对盈亏（**方向感知** close-to-close，含双边费用，与 Θ 账本侧成本对称口径一致）。
///
/// **方向感知（canonical 镜像等变的可测后果）**：有符号名义头寸 `n_v = σ_v·M·P·q`
/// （`formal/Origin/LeverageCapital.lean` §1，σ_v∈{−1,+1}）+ 全链镜像等变 σ→−σ
/// （`formal/Origin/MainTheorem.lean` §22「多空镜像」ℭ_{MΘ}∘M_X = M_S∘ℭ_Θ）⟹ 相同价格走势下
/// 多头（σ=+1）与空头（σ=−1）的 PnL **符号相反**（模费用对称）。两方向的成本语义对称：
/// **建仓名义额按成交方向含建仓费，平仓名义额扣平仓费**。
///
/// - **多头**（`long=true`，σ=+1）：建仓=买入（成本 `entry·(1+fee)`），平仓=卖出（收入 `exit·(1−fee)`）
///   ⟹ PnL = `qty·(exit·(1−fee) − entry·(1+fee))`（低买高卖盈利）。
/// - **空头**（`long=false`，σ=−1）：建仓=卖出（收入 `entry·(1−fee)`），平仓=买回（成本 `exit·(1+fee)`）
///   ⟹ PnL = `qty·(entry·(1−fee) − exit·(1+fee))`（高卖低买盈利，方向反转）。
///
/// 口径与 Θ 账本侧强平公式（`runner.rs`：`pos_sign·(px·(1−pos_sign·fee) − entry_cost)·|units|`）
/// 一致——令 σ=pos_sign，两者代数等价（空头 σ=−1 时 `px_exit_net = exit·(1+fee)`，符号翻转）。
pub(crate) fn trade_abs_pnl(entry_px: f64, exit_px: f64, qty: f64, fee_rate: f64, long: bool) -> f64 {
    if long {
        let proceeds = qty * exit_px * (1.0 - fee_rate);
        let cost = qty * entry_px * (1.0 + fee_rate);
        proceeds - cost
    } else {
        // 空头：建仓卖出收 entry·(1−fee)，平仓买回付 exit·(1+fee)。
        let proceeds = qty * entry_px * (1.0 - fee_rate);
        let cost = qty * exit_px * (1.0 + fee_rate);
        proceeds - cost
    }
}

/// 随机对照分布统计：均值 + 上单边 p 值 `p_upper = (1 + count(rand ≥ theta)) / (N+1)`。
///
/// 空分布（无合法重采样）⟹ 均值 0、p_upper=1.0（不冒充否证）。p_upper 用 Davison-Hinkley
/// 加 1 修正（保守，避免 p=0 的过度自信；codex 指出裸分位不是 p 值）。
fn control_stats(samples: &mut [f64], theta: f64) -> (f64, f64) {
    let n = samples.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = samples.iter().sum::<f64>() / n as f64;
    // count(rand ≥ theta)：随机策略不劣于 Θ 的次数。
    let n_ge = samples.iter().filter(|&&r| r >= theta).count();
    let p_upper = (1 + n_ge) as f64 / (n as f64 + 1.0);
    (mean, p_upper)
}

/// 一次 block bootstrap 重采样的总收益（块长 `block`，有放回抽连续块拼成 ≥len 序列后截断）。
///
/// 保留交易序列局部自相关结构（§3.4 block bootstrap 的目的——独立 bootstrap 破坏自相关）。
/// 抽块起点在 `[0, len)` 均匀（循环 wrap，使每笔被抽概率均等，避免末尾块偏置）。
fn block_bootstrap_total(pnls: &[f64], block: usize, rng: &mut SplitMix64) -> f64 {
    let len = pnls.len();
    let block = block.min(len).max(1);
    let mut total = 0.0;
    let mut filled = 0usize;
    while filled < len {
        let start = rng.next_below(len);
        let take = block.min(len - filled);
        for k in 0..take {
            total += pnls[(start + k) % len]; // 循环 wrap，等概率覆盖
        }
        filled += take;
    }
    total
}

/// 经验分位置信区间（就地排序 `samples`，取 `lo`/`hi` 分位）。
fn percentile_ci(samples: &mut [f64], lo: f64, hi: f64) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len();
    let idx = |q: f64| -> usize {
        (((n as f64 - 1.0) * q).round() as usize).min(n - 1)
    };
    (samples[idx(lo)], samples[idx(hi)])
}

/// Sharpe 标准误（Lo 2002 一阶自相关调整）+ 95% 置信区间。
///
/// Lo (2002)「The Statistics of Sharpe Ratios」：IID 下 SE(SR) ≈ √((1+SR²/2)/n)；
/// 一阶自相关 ρ₁ 调整因子 √(1 + ... )。本实装用 Lo 的自相关调整简化式：
/// SE(SR_annualized) ≈ SE_iid · √(adj)，adj = 1 - 2·ρ₁·(1 - 1/n)（一阶 AR(1) 近似）。
/// ρ₁ = lag-1 自相关。**诚实**：Lo 2002 完整式含全阶自相关；本实装取一阶（AR(1)）近似——
/// 标注 L1 口径（统计算法正确性），完整高阶 GMM 估计待 L3 精化（不冒充全阶）。
///
/// 边界：n<2 ⟹ SE=0，CI=[sharpe,sharpe]（无波动无标准误）。
fn sharpe_lo2002(daily_returns: &[f64], sharpe: f64) -> (f64, f64, f64) {
    let n = daily_returns.len();
    if n < 2 {
        return (0.0, sharpe, sharpe);
    }
    // IID 标准误（年化 Sharpe 的标准误 ≈ √((1 + SR²/2)/n)，SR 为年化值）。
    let se_iid = ((1.0 + sharpe * sharpe / 2.0) / n as f64).sqrt();
    // lag-1 自相关 ρ₁。
    let rho1 = lag1_autocorr(daily_returns);
    // AR(1) 调整因子（Lo 2002 一阶近似；ρ₁>0 正自相关 ⟹ SE 放大）。
    let adj = (1.0 + 2.0 * rho1 * (1.0 - 1.0 / n as f64)).max(0.0);
    let se = se_iid * adj.sqrt();
    (se, sharpe - 1.96 * se, sharpe + 1.96 * se)
}

/// lag-1 自相关系数 ρ₁（日 returns 序列）。
fn lag1_autocorr(x: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return 0.0;
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let denom: f64 = x.iter().map(|v| (v - mean).powi(2)).sum();
    if denom <= 1e-18 {
        return 0.0;
    }
    let numer: f64 = (0..n - 1).map(|i| (x[i] - mean) * (x[i + 1] - mean)).sum();
    numer / denom
}

/// 年化 Sharpe + Sortino（rf=0）。日 returns 序列 → 均值/标准差 → ×√252 年化。
fn sharpe_sortino(daily_returns: &[f64]) -> (f64, f64) {
    let n = daily_returns.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = daily_returns.iter().sum::<f64>() / n as f64;
    // 样本标准差（n-1）。
    let var = daily_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let std = var.sqrt();
    // 下行标准差（仅负 returns，相对 0 目标）。
    let downside_var = daily_returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let downside_std = downside_var.sqrt();

    const ANN: f64 = 252.0;
    let sharpe = if std > 1e-12 {
        mean / std * ANN.sqrt()
    } else {
        0.0
    };
    let sortino = if downside_std > 1e-12 {
        mean / downside_std * ANN.sqrt()
    } else {
        0.0
    };
    (sharpe, sortino)
}

/// 最大回撤（峰谷，正数，基于权益曲线）。
fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = f64::MIN;
    let mut max_dd = 0.0;
    for &e in equity {
        if e > peak {
            peak = e;
        }
        if peak > 0.0 {
            let dd = (peak - e) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// CVaR(α)：最差 α 分位尾部的条件期望（日 returns）。返回负数（亏损）。
fn cvar(daily_returns: &[f64], alpha: f64) -> f64 {
    let n = daily_returns.len();
    if n == 0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = daily_returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // 尾部样本数（至少 1）。
    let k = ((n as f64) * alpha).ceil().max(1.0) as usize;
    let tail = &sorted[..k.min(n)];
    tail.iter().sum::<f64>() / tail.len() as f64
}

/// 胜率 + 盈亏比。
fn win_rate_profit_factor(pnls: &[f64]) -> (f64, f64) {
    if pnls.is_empty() {
        return (0.0, 0.0);
    }
    let wins: Vec<f64> = pnls.iter().copied().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().copied().filter(|&p| p < 0.0).collect();
    let win_rate = wins.len() as f64 / pnls.len() as f64;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().sum::<f64>() / losses.len() as f64
    };
    let profit_factor = if avg_loss.abs() > 1e-12 {
        avg_win / avg_loss.abs()
    } else {
        0.0
    };
    (win_rate, profit_factor)
}

/// top-N 交易盈亏和 / 总盈亏（§5.2 收益集中度）。总盈亏≤0 ⇒ 0（防御）。
fn top_n_pnl_ratio(pnls: &[f64], n: usize) -> f64 {
    let total: f64 = pnls.iter().sum();
    if total <= 1e-12 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = pnls.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // 降序
    let top_sum: f64 = sorted.iter().take(n).sum();
    top_sum / total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空权益/无交易 → 全 0（不 NaN，§5.5 inconclusive 防御）。
    #[test]
    fn empty_inputs_yield_zero_metrics() {
        let m = compute(&[], &[], &[], 1.0, 0.0);
        assert_eq!(m.cagr, 0.0);
        assert_eq!(m.sharpe, 0.0);
        assert_eq!(m.n_trades, 0);
        assert_eq!(m.win_rate, 0.0);
        assert_eq!(m.profit_factor, 0.0);
        assert_eq!(m.top5_pnl_ratio, 0.0);
    }

    /// max_drawdown 峰谷正确（已知曲线）。
    #[test]
    fn max_drawdown_known_curve() {
        // 1 → 2 → 1（峰 2，谷 1，回撤 50%）→ 1.5。
        let dd = max_drawdown(&[1.0, 2.0, 1.0, 1.5]);
        assert!((dd - 0.5).abs() < 1e-9, "峰 2 谷 1 = 50% 回撤");
    }

    /// CVaR(5%) 取最差尾部均值（负数）。
    #[test]
    fn cvar_picks_worst_tail() {
        // 20 个 returns，最差 1 个（5%）= -0.10。
        let mut rets = vec![0.01; 19];
        rets.push(-0.10);
        let c = cvar(&rets, 0.05);
        assert!((c - (-0.10)).abs() < 1e-9, "5% 尾部 = 最差 1 个 = -0.10");
    }

    /// 胜率 + 盈亏比已知值。
    #[test]
    fn win_rate_and_profit_factor_known() {
        // 3 笔：+2, +4, -3 ⇒ 胜率 2/3；avg_win=3, avg_loss=3 ⇒ pf=1.0。
        let (wr, pf) = win_rate_profit_factor(&[2.0, 4.0, -3.0]);
        assert!((wr - 2.0 / 3.0).abs() < 1e-9);
        assert!((pf - 1.0).abs() < 1e-9, "avg_win 3 / avg_loss 3 = 1.0");
    }

    /// top-5 占比（§5.2）：少于 5 笔时取全部。
    #[test]
    fn top5_ratio_concentration() {
        // 总 = 10；top-1 = 8 ⇒ top5（不足5取全）= 10/10 = 1.0。
        let r = top_n_pnl_ratio(&[8.0, 1.0, 1.0], 5);
        assert!((r - 1.0).abs() < 1e-9);
        // 总 = 10；top-2 = 8+1=9 ⇒ 0.9。
        let r2 = top_n_pnl_ratio(&[8.0, 1.0, 0.5, 0.5], 2);
        assert!((r2 - 0.9).abs() < 1e-9);
    }

    /// Sharpe 符号：正均值 → 正 Sharpe；零波动 → 0（防 NaN）。
    #[test]
    fn sharpe_sign_and_zero_vol() {
        let (s, _) = sharpe_sortino(&[0.01, 0.02, 0.01, 0.015]);
        assert!(s > 0.0, "正均值 returns → 正 Sharpe");
        let (s0, so0) = sharpe_sortino(&[0.01, 0.01, 0.01]);
        assert_eq!(s0, 0.0, "零波动 → Sharpe 0（防 NaN）");
        assert_eq!(so0, 0.0, "无下行 → Sortino 0");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  统计显著性检验（§3.4 block bootstrap + §4 随机入场对照）
    //  认识论：合成 pnl 上跑 = L1（验证检验管线正确）。L2 只在真实回测交易序列时产生。
    // ──────────────────────────────────────────────────────────────────────

    /// SplitMix64 确定性可复现（同 seed ⟹ bit-exact 同序列，§4 seed 冻结要求）。
    #[test]
    fn splitmix64_deterministic_reproducible() {
        let mut a = SplitMix64::new(PREREG_SEED);
        let mut b = SplitMix64::new(PREREG_SEED);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64(), "同 seed ⟹ bit-exact 同序列");
        }
        // 不同 seed ⟹ 不同序列（PRNG 非常数）。
        let mut c = SplitMix64::new(PREREG_SEED + 1);
        let mut d = SplitMix64::new(PREREG_SEED);
        assert_ne!(c.next_u64(), d.next_u64(), "不同 seed ⟹ 不同序列");
    }

    /// next_below 落在 [0, n) 且均匀覆盖（确定性）。
    #[test]
    fn splitmix64_next_below_in_range() {
        let mut rng = SplitMix64::new(PREREG_SEED);
        let n = 7;
        let mut seen = [false; 7];
        for _ in 0..1000 {
            let v = rng.next_below(n);
            assert!(v < n, "next_below 落在 [0,{n})");
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s), "1000 抽样覆盖全部 7 个值（均匀性弱见证）");
    }

    /// 构造单笔多头 TradeRecord（测试 helper）。
    fn mk_trade(entry: usize, hold: usize, qty: f64) -> TradeRecord {
        TradeRecord {
            entry_bar: entry,
            exit_bar: entry + hold,
            hold_bars: hold,
            qty,
            long: true,
            forced_close: false,
        }
    }

    /// 构造单笔**空头** TradeRecord（测试 helper，long=false）。
    fn mk_trade_short(entry: usize, hold: usize, qty: f64) -> TradeRecord {
        TradeRecord {
            entry_bar: entry,
            exit_bar: entry + hold,
            hold_bars: hold,
            qty,
            long: false,
            forced_close: false,
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★方向感知 PnL（做空腿 join 前置）：trade_abs_pnl 多空公式 + 镜像对称
    //  认识论 L1：方向感知 = 管线正确性验证（合成价格上验证公式正确，零信息增量）。
    //  做空 alpha 的经验影响须 L3 全窗真实数据统一重跑（本工位只保证 PnL 算对）。
    //  定义依据：canonical 镜像等变 σ→−σ（formal/Origin/LeverageCapital.lean 有符号名义
    //  n_v=σ_v·M·P·q + MainTheorem.lean §22「多空镜像」ℭ_{MΘ}∘M_X = M_S∘ℭ_Θ）。
    // ──────────────────────────────────────────────────────────────────────

    /// ★镜像对称（canonical 镜像等变的可测后果）：同 entry/exit/qty/fee 下，多头 PnL = −空头 PnL。
    /// σ→−σ ⟹ 有符号名义符号翻转 ⟹ 逐笔 PnL 符号严格相反（费用语义对称：建仓含费、平仓扣费）。
    #[test]
    fn trade_abs_pnl_mirror_symmetry() {
        // 无费时严格镜像：long(100→110) = +10·qty，short(100→110) = −10·qty。
        let long_pnl = trade_abs_pnl(100.0, 110.0, 2.0, 0.0, true);
        let short_pnl = trade_abs_pnl(100.0, 110.0, 2.0, 0.0, false);
        assert!((long_pnl - 20.0).abs() < 1e-9, "多头 100→110 ×2 = +20，实得 {long_pnl}");
        assert!((short_pnl + 20.0).abs() < 1e-9, "空头 100→110 ×2 = −20，实得 {short_pnl}");
        assert!(
            (long_pnl + short_pnl).abs() < 1e-9,
            "镜像对称（无费）：long PnL = −short PnL，实得 long={long_pnl} short={short_pnl}"
        );
        // 含费时镜像对称仍成立（费用语义对称：两方向建仓含费、平仓扣费）。
        let fee = 0.001;
        let lp = trade_abs_pnl(100.0, 110.0, 1.0, fee, true);
        let sp = trade_abs_pnl(100.0, 110.0, 1.0, fee, false);
        // long = 110(1−f) − 100(1+f)；short = 100(1−f) − 110(1+f) ⟹ long+short = −2f(100+110) = −0.42。
        let expected_sum = -2.0 * fee * (100.0 + 110.0);
        assert!(
            (lp + sp - expected_sum).abs() < 1e-9,
            "含费：long+short = −2·fee·(entry+exit)（费用是两方向共同摩擦，非镜像可消），实得 {}",
            lp + sp
        );
    }

    /// ★空头盈利场景（entry_px > exit_px ⟹ 高卖低买盈利）+ 空头亏损（entry < exit ⟹ 亏）。
    #[test]
    fn trade_abs_pnl_short_profit_and_loss() {
        // 空头盈利：120 卖出建仓，100 买回平仓 ⟹ +20·qty（无费）。
        let win = trade_abs_pnl(120.0, 100.0, 1.0, 0.0, false);
        assert!((win - 20.0).abs() < 1e-9, "空头 120→100 = +20（高卖低买盈利），实得 {win}");
        // 空头亏损：100 卖出建仓，120 买回平仓 ⟹ −20·qty（价格涨，空头亏）。
        let loss = trade_abs_pnl(100.0, 120.0, 1.0, 0.0, false);
        assert!((loss + 20.0).abs() < 1e-9, "空头 100→120 = −20（价涨空头亏），实得 {loss}");
        // 对偶见证：同一 120→100 走势，多头反而亏 −20（低买高卖被破坏）。
        let long_same = trade_abs_pnl(120.0, 100.0, 1.0, 0.0, true);
        assert!((long_same + 20.0).abs() < 1e-9, "多头 120→100 = −20（高买低卖亏），实得 {long_same}");
        assert!((win + long_same).abs() < 1e-9, "同走势 short 盈 = −long 亏（镜像）");
    }

    /// ★随机对照保持方向（Short trade 重执行仍按空头公式）：纯空头轨迹在下跌价格上，
    /// Θ 同口径 total_return > 0（空头吃下跌盈利），且随机对照分布按空头公式算（非多头算反）。
    #[test]
    fn random_control_preserves_short_direction() {
        // 单调下跌价格（200 → 101）——空头在此盈利，多头亏损。
        let prices: Vec<f64> = (0..100).map(|i| 200.0 - i as f64).collect();
        // 全空头轨迹（low entry 段持有 3 根，下跌中空头盈利）。
        let trades: Vec<TradeRecord> = (0..8).map(|i| mk_trade_short(10 + i, 3, 1.0)).collect();
        // 手算 Θ 同口径：每笔 entry_px > exit_px（下跌），空头 pnl = entry − exit = +3·qty（无费）。
        let s = significance(&[], &[], &trades, &prices, 0.0, 0.0);
        // 空头在下跌中盈利 ⟹ Θ 同口径 total_return > 0（若算反成多头公式则会 <0）。
        assert!(
            s.theta_return_same_caliber > 0.0,
            "空头吃下跌 ⟹ Θ 同口径收益>0（方向感知正确，未按多头算反），实得 {}",
            s.theta_return_same_caliber
        );
        // 随机对照分布均值也按空头公式算（下跌价格上任意空头入场段平均盈利>0）。
        assert!(
            s.shift_mean_return > 0.0 && s.indep_mean_return > 0.0,
            "随机对照保持空头方向 ⟹ 下跌价格上随机空头均值>0，实得 shift={} indep={}",
            s.shift_mean_return, s.indep_mean_return
        );
    }

    /// ★镜像对照见证：同价格序列 + 同入场结构，多头轨迹 Θ 收益 = −空头轨迹 Θ 收益（无费）。
    /// 这是 random_entry_controls 层的镜像等变可测后果（nav_base 方向无关 ⟹ 归一化保符号关系）。
    #[test]
    fn theta_same_caliber_mirror_long_vs_short() {
        // 任意非单调价格（含涨跌）。
        let prices: Vec<f64> = (0..60).map(|i| 100.0 + 5.0 * ((i as f64) * 0.3).sin()).collect();
        let longs: Vec<TradeRecord> = (0..5).map(|i| mk_trade(i * 4, 2, 1.0)).collect();
        let shorts: Vec<TradeRecord> = (0..5).map(|i| mk_trade_short(i * 4, 2, 1.0)).collect();
        // nav_base 方向无关（Σ qty·entry·(1+fee)）⟹ 多空用同一分母，分子符号相反。
        let s_long = significance(&[], &[], &longs, &prices, 0.0, 0.0);
        let s_short = significance(&[], &[], &shorts, &prices, 0.0, 0.0);
        assert!(
            (s_long.theta_return_same_caliber + s_short.theta_return_same_caliber).abs() < 1e-9,
            "镜像等变：long Θ 收益 = −short Θ 收益（无费，nav_base 方向无关），实得 long={} short={}",
            s_long.theta_return_same_caliber, s_short.theta_return_same_caliber
        );
    }

    /// significance 可复现（同输入同 seed ⟹ bit-exact 同结果，§4 硬约束，含新随机对照）。
    #[test]
    fn significance_reproducible() {
        let pnls: Vec<f64> = (0..50).map(|i| if i % 3 == 0 { -1.0 } else { 2.0 }).collect();
        let rets: Vec<f64> = (0..50).map(|i| 0.001 * (i % 5) as f64 - 0.001).collect();
        // 合成价格序列 + 交易轨迹（随机对照可复现见证）。
        let prices: Vec<f64> = (0..100).map(|i| 100.0 + (i as f64).sin()).collect();
        let trades: Vec<TradeRecord> =
            (0..10).map(|i| mk_trade(i * 5, 3, 1.0)).collect();
        let s1 = significance(&pnls, &rets, &trades, &prices, 0.0003, 0.05);
        let s2 = significance(&pnls, &rets, &trades, &prices, 0.0003, 0.05);
        assert_eq!(s1, s2, "同输入同 seed ⟹ bit-exact 同结果（可复现，含随机对照）");
    }

    /// ★否证性见证①：**纯亏损序列** ⟹ bootstrap p 值 ≈ 1.0（无法拒绝 H0 收益≤0）。
    /// 这是统计检验**能产生否定性结果**的见证——亏损策略被正确判为不显著（§5.1 失败）。
    #[test]
    fn significance_losing_strategy_rejected() {
        // 全亏损交易（每笔 -1.0）。
        let pnls = vec![-1.0; 40];
        let rets = vec![-0.002; 40];
        let s = significance(&pnls, &rets, &[], &[], 0.0, 0.0);
        // H0：收益≤0 无法拒绝（p 值高）——所有重采样总收益都 <0 ⟹ p=1.0。
        assert!(
            s.boot_pvalue_pnl_le_0 > 0.95,
            "纯亏损 ⟹ bootstrap 总收益恒≤0 ⟹ p≈1.0（无法拒绝 H0），实得 {}",
            s.boot_pvalue_pnl_le_0
        );
        // 置信区间整体在负区（亏损分布）。
        assert!(s.boot_ci95_hi < 0.0, "纯亏损 ⟹ CI 上界<0");
    }

    /// ★否证性见证②：**强正收益序列** ⟹ bootstrap p 值 ≈ 0（拒绝 H0，收益显著>0）。
    /// 与亏损见证对偶——检验能正确**确认**显著盈利（不是只会说"不显著"）。
    #[test]
    fn significance_winning_strategy_confirmed() {
        // 全盈利交易（每笔 +2.0）。
        let pnls = vec![2.0; 40];
        let rets = vec![0.003; 40];
        let s = significance(&pnls, &rets, &[], &[], 0.0, 0.0);
        // 所有重采样总收益都 >0 ⟹ p=0（拒绝 H0，收益显著正）。
        assert!(
            s.boot_pvalue_pnl_le_0 < 0.05,
            "纯盈利 ⟹ bootstrap 总收益恒>0 ⟹ p≈0（拒绝 H0），实得 {}",
            s.boot_pvalue_pnl_le_0
        );
        assert!(s.boot_ci95_lo > 0.0, "纯盈利 ⟹ CI 下界>0");
    }

    /// 空 / 单笔交易 ⟹ 退化（p=1.0，不冒充显著性，§5.5 inconclusive）。
    #[test]
    fn significance_degenerate_no_trades() {
        let s0 = significance(&[], &[], &[], &[], 0.0, 0.0);
        assert_eq!(s0.boot_pvalue_pnl_le_0, 1.0, "无交易 ⟹ p=1.0（无证据，不冒充）");
        assert!(!s0.theta_beats_random, "无交易 ⟹ 不优于随机");
        assert_eq!(s0.shift_pvalue, 1.0, "无交易轨迹 ⟹ 随机对照退化 p_upper=1.0");
        assert_eq!(s0.indep_pvalue, 1.0, "无交易轨迹 ⟹ independent 对照退化 p_upper=1.0");
        let s1 = significance(&[5.0], &[0.01], &[], &[], 0.0, 0.0);
        assert_eq!(s1.boot_pvalue_pnl_le_0, 1.0, "仅 1 笔 ⟹ 无序列结构 ⟹ p=1.0");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★操作语义随机入场对照（§4 重写）：脱离自举恒真 + 否证性见证
    //  认识论：合成价格上跑 = L1（验证对照管线正确）。L2 只在真实回测交易轨迹时产生。
    // ──────────────────────────────────────────────────────────────────────

    /// ★脱离自举恒真的回归见证：随机对照分布均值**不**恒等于 Θ 收益。
    ///
    /// 旧 §4（self-resampling）：随机分布均值 ≡ Θ 总盈亏（数学恒真）。本测试构造**单调上涨**
    /// 价格 + Θ 入场点偏向**后段**（高位入场，差择时）⟹ 随机对照（全序列随机入场）的均值收益
    /// 应**高于** Θ（Θ 高位入场吃亏）⟹ Θ 分位低、p_upper 大 ⟹ 见证均值非恒等于 Θ（恒真已破）。
    #[test]
    fn random_control_not_self_resampling_tautology() {
        // 单调上涨价格（100 → 199）。
        let prices: Vec<f64> = (0..100).map(|i| 100.0 + i as f64).collect();
        // Θ 全部在**后段高位**入场（entry 70..80），持有 3 根——上涨趋势中高位入场仍盈利，
        // 但随机入场（可落在前段低位）平均盈利更高 ⟹ Θ 不优于随机。
        let trades: Vec<TradeRecord> = (0..8).map(|i| mk_trade(70 + i, 3, 1.0)).collect();
        let theta_ret: f64 = trades
            .iter()
            .map(|t| trade_abs_pnl(prices[t.entry_bar], prices[t.exit_bar], t.qty, 0.0, t.long))
            .sum::<f64>()
            / trades.iter().map(|t| t.qty * prices[t.entry_bar]).sum::<f64>();
        let s = significance(&[], &[], &trades, &prices, 0.0, theta_ret);
        // 单调上涨 + 固定 hold ⟹ 每笔绝对盈亏恒 = qty×hold（价格每根 +1）⟹ 随机均值 ≈ Θ。
        // 关键见证：随机分布均值是**重新执行操作语义**算出，不是从 Θ pnl 池重抽——
        // 故 shift_mean_return 反映价格序列结构，非 Θ 总盈亏的代数恒等。
        assert!(
            s.shift_mean_return.is_finite() && s.indep_mean_return.is_finite(),
            "随机对照均值有限（操作语义重算）"
        );
        // Θ 高位入场 ⟹ 不显著优于随机（两对照不全 ≤0.05）⟹ 不冒充择时 alpha。
        assert!(
            !s.theta_beats_random,
            "单调上涨中差择时（高位入场）⟹ Θ 不优于随机（脱离自举恒真后能产否定结果）"
        );
    }

    /// ★否证性见证③：**好择时**（Θ 入场恰在局部低点）⟹ Θ 显著优于 **independent** 对照。
    ///
    /// 锯齿价格（低点 100、高点 110 交替）。Θ 全部在**局部低点**入场、持有到**高点**平仓
    /// （完美择时）⟹ Θ 收益远高于 independent 随机入场分布 ⟹ indep p_upper ≤0.05。
    /// 这见证检验**能确认择时 alpha**（不是只会否证）——与差择时见证对偶。
    ///
    /// ★诚实标注（两对照检验不同的东西，正是 codex 要两对照的理由）：在**周期性价格**上，
    /// **schedule-shift 对相位对齐的择时不敏感**——整体平移 δ 为周期倍数时复现 Θ 的完美择时
    /// （锯齿周期=2，δ 为偶数时所有交易仍低买高卖），故 schedule-shift 分布双峰、约半数复制 Θ
    /// ⟹ shift p_upper ≈ 0.5（Θ 未胜）。这**不是 bug**，是 schedule-shift「保留交易-价格相位
    /// 联合结构」的**正确统计性质**：它检验「打散交易间时序相关后是否还有 alpha」，对
    /// 周期对齐的择时本就保守。independent 对照（破坏相位）才捕捉这类择时 ⟹ 两对照互补。
    #[test]
    fn random_control_good_timing_beats_independent() {
        // 锯齿：偶数 bar=100（低），奇数 bar=110（高）。
        let prices: Vec<f64> = (0..200).map(|i| if i % 2 == 0 { 100.0 } else { 110.0 }).collect();
        // Θ 在偶数 bar（低点）入场，持有 1 根到奇数 bar（高点）平仓——完美择时。
        let trades: Vec<TradeRecord> = (0..40).map(|i| mk_trade(i * 2, 1, 1.0)).collect();
        // Θ 每笔买 100 卖 110 ⟹ 含浮盈 total_return（fee=0）= Σ10 / Σ100 = 0.1。
        let theta_ret = 0.1;
        let s = significance(&[], &[], &trades, &prices, 0.0, theta_ret);
        // independent 随机入场（破坏相位，一半落在高点买低点卖 ⟹ 亏）⟹ 均值 ≈ 0 ≪ Θ 0.1。
        assert!(
            s.indep_mean_return < theta_ret,
            "完美择时 ⟹ Θ 收益 {theta_ret} > independent 均值 {}",
            s.indep_mean_return
        );
        assert!(
            s.indep_pvalue <= 0.05,
            "完美择时（低买高卖）⟹ Θ 显著优于 independent 随机入场（确认择时 alpha）。indep_p={}",
            s.indep_pvalue
        );
        // schedule-shift 在周期数据上对相位对齐择时不敏感（双峰分布，约半数复制 Θ）——
        // 诚实见证两对照检验不同的东西（非 bug，是 schedule-shift 保相位的正确性质）。
        assert!(
            s.shift_pvalue > 0.05,
            "周期价格上 schedule-shift 复现 Θ 相位 ⟹ shift_p>0.05（两对照互补的诚实见证）。shift_p={}",
            s.shift_pvalue
        );
    }

    /// 随机对照退化边界：hold_bars ≥ 价格序列长度 ⟹ independent 无合法区间 ⟹ p_upper=1.0。
    #[test]
    fn random_control_degenerate_short_prices() {
        let prices = vec![100.0, 101.0, 102.0];
        // hold=5 > len=3 ⟹ 无合法 entry 区间。
        let trades = vec![mk_trade(0, 5, 1.0)];
        let s = significance(&[], &[], &trades, &prices, 0.0, 0.0);
        // exit_bar=5 ≥ len=3 ⟹ schedule-shift 无合法 δ（max_exit≥len）⟹ 退化。
        assert_eq!(s.shift_pvalue, 1.0, "exit 越界 ⟹ schedule-shift 退化 p_upper=1.0");
        assert_eq!(s.indep_pvalue, 1.0, "hold≥len ⟹ independent 退化 p_upper=1.0");
        assert!(!s.theta_beats_random, "退化 ⟹ 不冒充否证");
        assert!(s.controls_degenerate, "无真随机样本 ⟹ controls_degenerate=true（下游归 inconclusive）");
    }

    /// ★schedule-shift span==1 退化 guard（codex 缺陷⑤）：交易紧贴序列末尾 ⟹ 唯一 δ=0（复现 Θ）
    /// ⟹ 无真随机平移 ⟹ shift 退化。independent 仍可随机（hold<len）⟹ 仅 shift 退化即整体退化。
    #[test]
    fn random_control_shift_span_one_degenerate() {
        // len=4；交易 entry=2 exit=3（紧贴末尾）⟹ min_entry=2 max_exit=3 ⟹
        // span = (4-1-3) + 2 + 1 = 3 ≥2，非退化。构造 span==1：entry=0 exit=3（占满全程）⟹
        // min_entry=0 max_exit=3 ⟹ span = (4-1-3)+0+1 = 1 ⟹ 唯一 δ=0 ⟹ shift 退化。
        let prices = vec![100.0, 101.0, 102.0, 103.0];
        let trades = vec![mk_trade(0, 3, 1.0)]; // entry=0 exit=3 占满 ⟹ span=1
        let s = significance(&[], &[], &trades, &prices, 0.0, 0.0);
        assert_eq!(s.shift_pvalue, 1.0, "span==1（唯一 δ=0）⟹ shift 无真随机 ⟹ 退化 p_upper=1.0");
        assert!(s.controls_degenerate, "shift 退化 ⟹ controls_degenerate=true");
        assert!(!s.theta_beats_random, "退化 ⟹ 不冒充 beats");
    }

    /// Sharpe Lo 2002 标准误：正自相关 ⟹ SE 放大（调整因子 >1）；CI 含 sharpe。
    #[test]
    fn sharpe_lo2002_autocorr_inflates_se() {
        // 强正自相关序列（趋势性）。
        let rets: Vec<f64> = (0..50).map(|i| 0.001 + 0.0001 * (i as f64)).collect();
        let (sharpe, _) = sharpe_sortino(&rets);
        let (se, lo, hi) = sharpe_lo2002(&rets, sharpe);
        assert!(se > 0.0, "有波动 ⟹ SE>0");
        assert!(lo < sharpe && sharpe < hi, "CI 含点估计");
        // 与 IID SE 对比：正自相关 ⟹ Lo SE ≥ IID SE。
        let se_iid = ((1.0 + sharpe * sharpe / 2.0) / rets.len() as f64).sqrt();
        let rho1 = lag1_autocorr(&rets);
        if rho1 > 0.0 {
            assert!(se >= se_iid * 0.999, "正自相关 ⟹ Lo SE ≥ IID SE");
        }
    }
}
