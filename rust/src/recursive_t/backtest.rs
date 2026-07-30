//! 统一递归算子 T 的回测引擎（standalone，全 Rust）。
//!
//! GUARD-ROLE: t-engine-rec-branch-zero-external-caller——名分：deprecated 待退役
//! （详见 `rec_engine.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 直接消费 T 算子的买卖点输出驱动交易模拟，对照 Structural/AND/OR 三种**步骤c 背驰
//! 判定模式**（编排者 2026-06-18 裁决「三条路实测」，escalation `2026-06-18-1752-t-
//! stepc-macd-scope-vs-direction.md`）。三路**同 a₀ 同操作层**，唯一区别 = [`PerfectionMode`]
//! ——这是受控实验：回测收益差异**全部**归因于 MACD 收紧/放宽，不掺操作层噪声。
//!
//! ## 认识论等级（formalization-validity-domain）
//!
//! **L2**（真实数据，可否证）——但带有效域边界，回测数字不可直接用于实盘：
//! 1. **成交价 = 成交 bar 的 close（价格维 look-ahead 已消除）**：`BSP.price` 是段端点
//!    极值（type1=背驰/末段极值、type3=回抽段极值，见 `divergence.rs`/`operator.rs`）；
//!    在端点用极值价成交 = 假设「恰好买在最低 / 卖在最高」= 未来函数。本版改用
//!    `closes[raw_end]`（成交 bar 收盘可成交价），`BSP.price` 仅用于信号定位、不进成交。
//! 2. **batch 回测，时间维确认滞后（残留 look-ahead）**：T 在全历史 a₀ 上一次跑完，
//!    进入 a₀ 的线段都已 `confirmed && settled`。线段确认时刻 **晚于** 其端点 bar
//!    （区间套确认滞后，记忆 `interval_nesting_forward`），但成交 bar 仍取端点 raw_end
//!    ⇒ 实盘信号可见时刻更晚，绝对收益仍偏乐观。三模式**同口径** ⇒ 模式间对比有效。
//! 3. **操作层简化**：见 [`apply_bsp`]——只做多、减仓不回补、向下趋势空仓。目的是隔离
//!    步骤c 变量（用户指令「保持简单」），非最优交易策略。
//!
//! ## 坐标系（记忆 `current_strokes_i1_merged_coord`）
//!
//! BSP.bar 是 **merged bar 坐标**（K 线包含处理后）；逐 bar MtM / max_drawdown 须对齐
//! **raw bar**。唯一合法桥梁 = `merged_to_raw[m] = (raw_start, raw_end)`。本模块统一用
//! `merged_to_raw[bsp.bar].1`（raw_end，该 merged bar 完成时最晚可见的 raw bar）。

use super::iterate;
use super::types::{BSPKind, Direction, PerfectionMode, Unit, BSP};
use crate::macd::{compute_macd_batch, macd_area_for_range};
use crate::segment::{SegKind, Segment};
use crate::stroke::{Direction as StrokeDir, Stroke};

/// 回测背驰判定模式（一对一映射 [`PerfectionMode`]，仅为回测出参/命名分离）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacktestMode {
    /// 纯结构 5 条件（§6.6 默认）。
    Structural,
    /// 结构 ∧ MACD 面积衰减（收紧门，信号单调减）。
    And,
    /// 结构 ∨ MACD 面积衰减（放宽门，M 可绕过 F∧S）。
    Or,
}

impl BacktestMode {
    pub fn to_perfection(self) -> PerfectionMode {
        match self {
            BacktestMode::Structural => PerfectionMode::Structural,
            BacktestMode::And => PerfectionMode::And,
            BacktestMode::Or => PerfectionMode::Or,
        }
    }

    /// JSON / 文件名用小写标签（与 `run_recursive_t` 的 mode 参数一致）。
    pub fn as_str(self) -> &'static str {
        match self {
            BacktestMode::Structural => "structural",
            BacktestMode::And => "and",
            BacktestMode::Or => "or",
        }
    }

    pub const ALL: [BacktestMode; 3] = [
        BacktestMode::Structural,
        BacktestMode::And,
        BacktestMode::Or,
    ];
}

/// 仓位方向（简单版只产 Long；Short 为下游扩展预留，见 [`apply_bsp`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeDir {
    Long,
    Short,
}

impl TradeDir {
    pub fn as_str(self) -> &'static str {
        match self {
            TradeDir::Long => "long",
            TradeDir::Short => "short",
        }
    }
}

/// 单笔（平仓动作）交易记录。一次建仓可对应多笔平仓（减仓短差 + 最终清仓）。
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    /// 建仓 raw bar。
    pub entry_bar: i64,
    /// 平仓 raw bar。
    pub exit_bar: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub direction: TradeDir,
    /// 本次平仓的股数（减仓 = 建仓股数的 1/3；清仓 = 剩余全部）。
    pub units: f64,
    /// 本次平仓盈亏 = units × (exit_price − entry_price)（Long）。
    pub pnl: f64,
    /// 入场信号类型（建仓那笔 BSP 的 kind）。
    pub entry_bsp_kind: BSPKind,
    /// 入场信号级别（T 迭代深度，0 = 最低走势级别）。
    pub entry_level: usize,
    /// 平仓原因（"trim"=次级别卖点减仓短差；"clear"=最高级别卖点清仓）。
    pub exit_reason: &'static str,
}

/// 回测汇总指标。
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// 策略收益率（%）= (期末权益 / 初始资金 − 1) × 100。
    pub strat_pct: f64,
    /// buy-and-hold 收益率（%）= (末 close / 首 close − 1) × 100。
    pub bh_pct: f64,
    pub n_trades: usize,
    pub n_long: usize,
    pub n_short: usize,
    /// 全部平仓盈亏之和（归一化资金口径）。
    pub total_pnl: f64,
    /// 最大回撤（%，逐 raw bar 权益曲线，正数）。
    pub max_drawdown: f64,
    pub n_bars: usize,
    pub n_segs: usize,
    /// a₀ 过滤（confirmed && settled）后进入 T 的单元数。
    pub n_a0_units: usize,
    pub n_bsps: usize,
    /// 涌现上界 r*（最高完整走势级别）。
    pub ceiling: usize,
}

/// 回测结果（交易明细 + 汇总）。
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestResult {
    pub mode: BacktestMode,
    pub trades: Vec<Trade>,
    pub summary: Summary,
}

/// stroke 方向 → recursive_t 方向（两个独立 Direction 枚举，构造 a₀ Unit 时桥接）。
fn conv_dir(d: StrokeDir) -> Direction {
    match d {
        StrokeDir::Up => Direction::Up,
        StrokeDir::Down => Direction::Down,
    }
}

/// 从线段序列构造 T 的 a₀ 单元（带 MACD 面积，供 And/Or 模式）。
///
/// 过滤口径 `confirmed && kind==Settled`（与 `ffi::run_recursive_t` / v3
/// `zhongshu_from_segments` 一致，保证 a₀ 同源）。MACD 面积经 `compute_macd_batch`
/// （batch hist，raw bar 对齐）+ `macd_area_for_range`（端点 merged→raw 换算）注入：
/// 上涨段存 `area_pos`，所有段 `area_neg` 取绝对值（divergence.rs `leg_macd_force` 约定：
/// 上涨段 Σarea_pos / 下跌段 Σarea_neg，二者均非负）。
///
/// Structural 模式下 area 不被读取（纯结构路径），但仍注入——三模式共用同一 a₀，
/// 只 [`PerfectionMode`] 不同（受控实验前提）。
pub fn build_a0_from_segments(
    segs: &[Segment],
    merged_to_raw: &[(usize, usize)],
    closes: &[f64],
) -> Vec<Unit> {
    let hist = compute_macd_batch(closes, 12, 26, 9).hist;
    segs.iter()
        .filter(|s| s.confirmed && s.kind == SegKind::Settled)
        .map(|s| {
            let raw_i0 = merged_to_raw[s.i0].0 as i64;
            let raw_i1 = merged_to_raw[s.i1].1 as i64;
            let area = macd_area_for_range(&hist, raw_i0, raw_i1);
            Unit {
                high: s.high,
                low: s.low,
                start_bar: s.i0 as i64,
                end_bar: s.i1 as i64,
                direction: conv_dir(s.direction),
                level: 0,
                inner_zhongshu_count: 0,
                area_pos: area.area_pos,
                area_neg: area.area_neg.abs(),
            }
        })
        .collect()
}

/// 从**笔**序列构造 T 的 a₀ 单元（batch 路径，与 [`build_a0_from_segments`] 并列）。
///
/// 递归底座下移 a₀=笔（谱系 525/526，第65课 065:182「区别仅在 a0」）：过滤口径 `confirmed`
/// ——笔无线段的 Settled 特征序列递归确认语义（bi.md:151「最后一笔始终 confirmed=False，直到
/// 下一笔生成后才结算」），完成 = confirmed。单元构造（坐标/面积）与线段版逐字一致：笔端点
/// i0/i1 与线段端点同为 merged bar 坐标（笔 i0/i1 = 分型中心 df_merged iloc，stroke.rs:27），
/// 故 `merged_to_raw` 换算 + `macd_area_for_range` 注入口径不变——a₀ 来源差异仅在过滤口径。
///
/// 与 fast 流式 [`stream::build_a0_fast`]（`A0Source::Stroke`）同口径；batch 用
/// `compute_macd_batch`（adjust=False EWM）而非 online MACD，故两路 area 路径不同（见模块头）。
pub fn build_a0_from_strokes(
    strokes: &[Stroke],
    merged_to_raw: &[(usize, usize)],
    closes: &[f64],
) -> Vec<Unit> {
    let hist = compute_macd_batch(closes, 12, 26, 9).hist;
    strokes
        .iter()
        .filter(|s| s.confirmed)
        .map(|s| {
            let raw_i0 = merged_to_raw[s.i0].0 as i64;
            let raw_i1 = merged_to_raw[s.i1].1 as i64;
            let area = macd_area_for_range(&hist, raw_i0, raw_i1);
            Unit {
                high: s.high,
                low: s.low,
                start_bar: s.i0 as i64,
                end_bar: s.i1 as i64,
                direction: conv_dir(s.direction),
                level: 0,
                inner_zhongshu_count: 0,
                area_pos: area.area_pos,
                area_neg: area.area_neg.abs(),
            }
        })
        .collect()
}

/// 仓位状态（回测主循环内部）。
struct Position {
    /// 当前持仓股数（0 = 空仓）。
    units: f64,
    /// 建仓时的股数（减仓配额 1/3 以此为基；清仓后归零）。
    base_units: f64,
    entry_price: f64,
    entry_bar: i64,
    entry_kind: BSPKind,
    entry_level: usize,
}

/// 操作层语义（H¹ 操作轴，设计文档 §4）——**回测的可调决策点**。
///
/// 从缠论四步跨级别循环导出的**简化**仓位逻辑（用户指令「保持简单」，目的是隔离步骤c
/// 变量，非最优策略）。当前默认读法（自洽、可跑）：
///
/// | 信号 | 条件 | 动作 |
/// |------|------|------|
/// | 买点(任意类) | 空仓 | 全额建多（cash→units） |
/// | 买点 | 已持多仓 | 忽略（规格「无仓则建多」；不加仓、不回补） |
/// | 卖点(任意类) | 持多 ∧ level==ceiling−1（最高涌现级别） | 清仓 |
/// | 卖点 | 持多 ∧ level<ceiling−1（次级别） | 减仓 1/3（H¹ 短差，减后不回补） |
/// | 任意 | 空仓遇卖点 / 持仓遇买点 | 不动 |
///
/// **下游可替换的领域语义**（用户作为交易者的领域知识可改进处，非本受控实验必需）：
/// - **短差回补**：当前减 1/3 后单调，"短差"退化为分级减仓。回补需追踪「待回补」状态，
///   在后续次级别买点补回（H¹ 配额释放，记忆 `hold26` 系列）。
/// - **向下趋势做空**：当前最高级别向下时只空仓观望。反手做空需 `TradeDir::Short` 单相
///   账本（记忆 `bidirectional_accounting`，原文 15 课认沽期权是唯一实操空头载体）。
///
/// `fill_price` = 成交 bar（raw_end）的 close —— 实际成交价（**非** `BSP.price` 段极值，
/// 后者只定位信号；见模块头 L2 边界 1）。返回本次产生的平仓 Trade（可能 0 笔）；
/// `cash` 用 `&mut` 就地更新。
fn apply_bsp(
    bsp: &BSP,
    raw_bar: i64,
    fill_price: f64,
    ceiling: usize,
    pos: &mut Option<Position>,
    cash: &mut f64,
) -> Option<Trade> {
    let is_top_level = ceiling > 0 && bsp.level + 1 == ceiling;
    match (bsp.kind.is_buy(), pos.as_mut()) {
        // 买点 + 空仓 → 全额建多。
        (true, None) => {
            if fill_price > 0.0 && *cash > 0.0 {
                let units = *cash / fill_price;
                *cash = 0.0;
                *pos = Some(Position {
                    units,
                    base_units: units,
                    entry_price: fill_price,
                    entry_bar: raw_bar,
                    entry_kind: bsp.kind,
                    entry_level: bsp.level,
                });
            }
            None
        }
        // 买点 + 已持仓 → 忽略（不加仓、不回补，保持简单）。
        (true, Some(_)) => None,
        // 卖点 + 空仓 → 不动（简单版不做空）。
        (false, None) => None,
        // 卖点 + 持多。
        (false, Some(p)) => {
            // 平仓股数：最高级别卖点清仓；否则减 1/3 配额（不超过剩余）。
            let (sell_units, reason) = if is_top_level {
                (p.units, "clear")
            } else {
                ((p.base_units / 3.0).min(p.units), "trim")
            };
            if sell_units <= 0.0 {
                return None;
            }
            let pnl = sell_units * (fill_price - p.entry_price);
            *cash += sell_units * fill_price;
            p.units -= sell_units;
            let trade = Trade {
                entry_bar: p.entry_bar,
                exit_bar: raw_bar,
                entry_price: p.entry_price,
                exit_price: fill_price,
                direction: TradeDir::Long,
                units: sell_units,
                pnl,
                entry_bsp_kind: p.entry_kind,
                entry_level: p.entry_level,
                exit_reason: reason,
            };
            // 仓位耗尽（清仓，或减仓减到 ~0）→ 平仓。
            if p.units <= 1e-12 {
                *pos = None;
            }
            Some(trade)
        }
    }
}

/// 回测主入口：a₀ → T(mode) → 仓位模拟 → 交易明细 + 汇总。
///
/// `closes` 为清洗后 raw close 序列（与喂给 orchestrator 的 bar 序列同索引 ⇒ 与
/// `merged_to_raw` 的 raw 索引一致）。`merged_to_raw` 来自同一 orchestrator。
pub fn run_backtest(
    a0_units: Vec<Unit>,
    mode: BacktestMode,
    closes: &[f64],
    merged_to_raw: &[(usize, usize)],
    n_segs: usize,
) -> BacktestResult {
    let n_a0 = a0_units.len();
    let tree = iterate(a0_units, mode.to_perfection());
    let ceiling = tree.emergent_ceiling();

    // 全塔 BSP → (raw_bar, bsp)，按 raw_bar 稳定排序（同 bar 多信号按塔内顺序）。
    let mut events: Vec<(i64, BSP)> = tree
        .all_bsps()
        .into_iter()
        .filter_map(|b| {
            let m = b.bar as usize;
            // merged idx 越界防御（理论不发生，pub 边界严格化）。
            merged_to_raw
                .get(m)
                .map(|&(_, raw_end)| (raw_end as i64, b))
        })
        .collect();
    let n_bsps = events.len();
    events.sort_by_key(|(raw_bar, _)| *raw_bar);

    let mut trades: Vec<Trade> = Vec::new();
    let mut pos: Option<Position> = None;
    let mut cash: f64 = 1.0; // 归一化初始资金

    // 逐 raw bar 推进：先处理本 bar 的 BSP 事件，再 MtM 记权益（max_drawdown）。
    let mut ev_idx = 0usize;
    let mut peak = 1.0f64;
    let mut max_dd = 0.0f64;
    let n = closes.len();
    for i in 0..n {
        while ev_idx < events.len() && events[ev_idx].0 == i as i64 {
            if let Some(t) = apply_bsp(
                &events[ev_idx].1,
                i as i64,
                closes[i],
                ceiling,
                &mut pos,
                &mut cash,
            ) {
                trades.push(t);
            }
            ev_idx += 1;
        }
        // MtM 权益 = cash + 持仓市值。
        let held = pos.as_ref().map_or(0.0, |p| p.units * closes[i]);
        let equity = cash + held;
        if equity > peak {
            peak = equity;
        }
        if peak > 0.0 {
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    // 末 bar 之后仍可能有 raw_bar 越界事件（merged_to_raw raw_end == n 边界），兜底处理。
    // 成交价取末 bar close（与主循环 closes[i] 同口径——非 BSP.price 段极值）。
    let last_bar = n.saturating_sub(1);
    let last_fill = *closes.last().unwrap_or(&0.0);
    while ev_idx < events.len() {
        if let Some(t) = apply_bsp(
            &events[ev_idx].1,
            last_bar as i64,
            last_fill,
            ceiling,
            &mut pos,
            &mut cash,
        ) {
            trades.push(t);
        }
        ev_idx += 1;
    }

    // 期末权益：未平仓位按末 close MtM（不强制平仓，反映持仓状态）。
    let last_close = *closes.last().unwrap_or(&0.0);
    let final_equity = cash + pos.as_ref().map_or(0.0, |p| p.units * last_close);
    let strat_pct = (final_equity - 1.0) * 100.0;
    let bh_pct = if !closes.is_empty() && closes[0] > 0.0 {
        (closes[n - 1] / closes[0] - 1.0) * 100.0
    } else {
        0.0
    };
    let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
    let n_long = trades
        .iter()
        .filter(|t| t.direction == TradeDir::Long)
        .count();
    let n_short = trades
        .iter()
        .filter(|t| t.direction == TradeDir::Short)
        .count();

    BacktestResult {
        mode,
        summary: Summary {
            strat_pct,
            bh_pct,
            n_trades: trades.len(),
            n_long,
            n_short,
            total_pnl,
            max_drawdown: max_dd * 100.0,
            n_bars: n,
            n_segs,
            n_a0_units: n_a0,
            n_bsps,
            ceiling,
        },
        trades,
    }
}

// ════════════════════════════════════════════════════════════
// JSON 序列化（手写，零依赖——避免 polars/serde 重依赖污染 cdylib 构建）
// ════════════════════════════════════════════════════════════

/// f64 → JSON 数字（非有限值 → null，保 6 位有效信息）。
fn jf(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.6}")
    } else {
        "null".to_string()
    }
}

/// 单笔 Trade → JSON 对象。
fn trade_json(t: &Trade) -> String {
    format!(
        "{{\"entry_bar\":{},\"exit_bar\":{},\"entry_price\":{},\"exit_price\":{},\
\"direction\":\"{}\",\"units\":{},\"pnl\":{},\"bsp_kind\":\"{}\",\"level\":{},\"exit_reason\":\"{}\"}}",
        t.entry_bar,
        t.exit_bar,
        jf(t.entry_price),
        jf(t.exit_price),
        t.direction.as_str(),
        jf(t.units),
        jf(t.pnl),
        t.entry_bsp_kind.as_str(),
        t.entry_level,
        t.exit_reason,
    )
}

/// 完整回测结果 → JSON（含 symbol/mode、summary、trades 列表）。
pub fn result_to_json(symbol: &str, res: &BacktestResult) -> String {
    let s = &res.summary;
    let trades: Vec<String> = res.trades.iter().map(trade_json).collect();
    format!(
        "{{\"symbol\":\"{}\",\"mode\":\"{}\",\"summary\":{{\
\"strat_pct\":{},\"bh_pct\":{},\"n_trades\":{},\"n_long\":{},\"n_short\":{},\
\"total_pnl\":{},\"max_drawdown\":{},\"n_bars\":{},\"n_segs\":{},\"n_a0_units\":{},\
\"n_bsps\":{},\"ceiling\":{}}},\"trades\":[{}]}}",
        symbol,
        res.mode.as_str(),
        jf(s.strat_pct),
        jf(s.bh_pct),
        s.n_trades,
        s.n_long,
        s.n_short,
        jf(s.total_pnl),
        jf(s.max_drawdown),
        s.n_bars,
        s.n_segs,
        s.n_a0_units,
        s.n_bsps,
        s.ceiling,
        trades.join(","),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    /// 上涨趋势八笔（2 中枢 + 背驰末段，复用 mod.rs 测试夹具）→ 一类卖点 @45。
    fn 上涨趋势八笔() -> Vec<Unit> {
        vec![
            bi(8.0, 22.0, 0, 1, Direction::Up),
            bi(12.0, 18.0, 1, 2, Direction::Down),
            bi(10.0, 16.0, 2, 3, Direction::Up),
            bi(23.0, 35.0, 3, 4, Direction::Up),
            bi(32.0, 40.0, 4, 5, Direction::Down),
            bi(33.0, 42.0, 5, 6, Direction::Up),
            bi(31.0, 39.0, 6, 7, Direction::Down),
            bi(40.0, 45.0, 7, 8, Direction::Up),
        ]
    }

    /// merged==raw 的恒等映射（单元测试用合成 a₀，bar 直接对齐）。
    fn identity_m2r(n: usize) -> Vec<(usize, usize)> {
        (0..n).map(|i| (i, i)).collect()
    }

    #[test]
    fn mode_映射一致() {
        assert_eq!(
            BacktestMode::Structural.to_perfection(),
            PerfectionMode::Structural
        );
        assert_eq!(BacktestMode::And.to_perfection(), PerfectionMode::And);
        assert_eq!(BacktestMode::Or.to_perfection(), PerfectionMode::Or);
    }

    #[test]
    fn 回测空a0不panic() {
        let closes = vec![10.0, 11.0, 12.0];
        let res = run_backtest(
            Vec::new(),
            BacktestMode::Structural,
            &closes,
            &identity_m2r(3),
            0,
        );
        assert_eq!(res.summary.n_trades, 0);
        assert_eq!(res.summary.strat_pct, 0.0, "无交易 → 策略持平");
        assert!(
            (res.summary.bh_pct - 20.0).abs() < 1e-9,
            "bh = 12/10-1 = 20%"
        );
    }

    #[test]
    fn 单趋势端到端建仓平仓闭环() {
        // a₀ = 8 笔上涨：中枢1 离开+回抽产 type3_buy，背驰末段产 type1_sell。
        // ⇒ 买点先(bar5)建仓、卖点后(bar8)清仓（level0=最高级别 ceiling=1）⇒ ≥1 笔盈利。
        let closes: Vec<f64> = (0..9).map(|i| 10.0 + i as f64 * 5.0).collect();
        let res = run_backtest(
            上涨趋势八笔(),
            BacktestMode::Structural,
            &closes,
            &identity_m2r(9),
            8,
        );
        assert!(res.summary.ceiling == 1, "8 笔 → 1 级");
        assert!(res.summary.n_bsps >= 2, "应同时含买点(type3)与卖点(type1)");
        assert!(res.summary.n_trades >= 1, "买点建仓 + 卖点平仓 ⇒ ≥1 笔");
        assert_eq!(res.summary.n_short, 0, "简单版只做多");
        assert!(
            (res.summary.bh_pct - 400.0).abs() < 1e-9,
            "bh = 50/10-1 = 400%"
        );
    }

    #[test]
    fn 买点建仓卖点清仓闭环() {
        // 下跌八笔（镜像）产一类买点 @-45 @level0；随后人工注入一个更高 close 让 MtM 体现。
        // 这里直接测 apply_bsp 状态机闭环：买点建多 → 最高级别卖点清仓 → 1 笔盈利。
        let mut pos: Option<Position> = None;
        let mut cash = 1.0;
        let buy = BSP {
            kind: BSPKind::Type1Buy,
            bar: 0,
            price: 10.0,
            level: 0,
        };
        let t0 = apply_bsp(&buy, 0, 10.0, 1, &mut pos, &mut cash);
        assert!(t0.is_none(), "建仓不产平仓交易");
        assert!(pos.is_some());
        assert_eq!(cash, 0.0, "全额建多 cash 清零");

        let sell = BSP {
            kind: BSPKind::Type1Sell,
            bar: 5,
            price: 20.0,
            level: 0,
        };
        let t1 = apply_bsp(&sell, 5, 20.0, 1, &mut pos, &mut cash).expect("最高级别卖点清仓");
        assert_eq!(t1.exit_reason, "clear");
        // units = 1.0/10 = 0.1 股；pnl = 0.1 × (20−10) = 1.0。
        assert!((t1.units - 0.1).abs() < 1e-12);
        assert!((t1.pnl - 1.0).abs() < 1e-9, "0.1 股 ×(20−10) = 1.0");
        assert!(pos.is_none(), "清仓后空仓");
        assert!((cash - 2.0).abs() < 1e-9, "0.1×20 = 2.0 现金");
    }

    #[test]
    fn 次级别卖点减仓三分之一不清仓() {
        // ceiling=2 ⇒ level0 是次级别（level+1=1 < 2）⇒ 卖点减 1/3。
        let mut pos: Option<Position> = None;
        let mut cash = 1.0;
        let buy = BSP {
            kind: BSPKind::Type1Buy,
            bar: 0,
            price: 10.0,
            level: 0,
        };
        apply_bsp(&buy, 0, 10.0, 2, &mut pos, &mut cash);
        let base = pos.as_ref().unwrap().base_units; // 0.1

        let sell = BSP {
            kind: BSPKind::Type1Sell,
            bar: 5,
            price: 12.0,
            level: 0,
        };
        let t = apply_bsp(&sell, 5, 12.0, 2, &mut pos, &mut cash).expect("次级别减仓");
        assert_eq!(t.exit_reason, "trim");
        assert!((t.units - base / 3.0).abs() < 1e-12, "减 1/3 配额");
        assert!(pos.is_some(), "减仓后仍持仓");
        let remain = pos.as_ref().unwrap().units;
        assert!((remain - base * 2.0 / 3.0).abs() < 1e-12, "剩 2/3");
    }
}
