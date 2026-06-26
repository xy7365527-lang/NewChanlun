//! Θ_exec 执行（reference-theta-v0.md:49-54）。
//!
//! ## 范围
//!
//! - 延迟成交（spec:50）：信号确认后延迟 `config.exec.entry_delay_bars` 根基础 K，以下一根
//!   open 成交；无下一根等下一可交易 open。
//! - 费用（spec:51）：commission/slippage/tax（config，bp/side）；买入加 slippage、卖出减。
//! - 止损成交（spec:52）：long bar open 低于止损按 open 出，否则 low 触及止损按 stop 出；
//!   short 镜像。
//! - 不可交易过滤（spec:53）：`Bar.untradable`（缺 OHLC / high<max(open,close,low) /
//!   low>min(open,close,high) / volume=0 / halt / limit flag）。
//! - 冲突顺序（spec:54）：止损/退出先于开仓；高 level 先于低 level；同 level 按 1/2/3 类；
//!   仍平局按 (timestamp, source_index)。
//!
//! ## 认识论等级
//!
//! L0（定义内蕴）：延迟/费用/止损成交/过滤/排序是给定 Θ_exec 参数后的确定函数。
//! ★诚实：commission/slippage/tax/entry_delay 全是 **Θ_exec 参数**（设计选择，spec:51-54
//! 标 [设计选择]/[L3经验待标定]）。本模块证「给定这些参数后成交价/排序唯一确定」，不证盈利。

use super::super::config::ExecConfig;
use super::super::types::{Bar, Tick};

/// 成交方向（买入/卖出，决定 slippage 加减，spec:51）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillSide {
    Buy,
    Sell,
}

/// 在 `signal_index` 确认信号后，求延迟成交的目标 bar 索引（reference-theta-v0.md:50）。
///
/// 信号在 `signal_index` 确认 ⟹ 延迟 `config.exec.entry_delay_bars` 根，以**下一根**的
/// open 成交。若延迟落点或其后的 bar 不可交易（`untradable`），**顺延到下一可交易 open**
/// （spec:50「无下一根等下一可交易 open」）。
///
/// 返回成交 bar 的索引；若到序列末尾仍无可交易 bar ⟹ 返回 `None`（信号作废，无成交）。
///
/// 边界条件：`signal_index + entry_delay_bars` 越界 ⟹ `None`（无下一根）。
/// `entry_delay_bars=1`（默认）⟹ 成交在 `signal_index + 1`（若可交易）。
pub fn fill_bar_index(signal_index: usize, bars: &[Bar], config: &ExecConfig) -> Option<usize> {
    let target = signal_index.checked_add(config.entry_delay_bars as usize)?;
    // 从延迟落点起，找第一个可交易 bar（顺延，spec:50）。
    let mut i = target;
    while i < bars.len() {
        if !bars[i].untradable {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// 费用率折算（reference-theta-v0.md:51）：commission + slippage(方向) + tax，单位价格 tick。
///
/// 费用按 bp/side（万分之一）作用于成交价。买入**加** slippage、卖出**减**（spec:51）；
/// commission 与 tax 对买卖同向（都是成本，从有利方向偏移）。返回**每单位的费用调整后
/// 成交价**（整数 tick）。
///
/// ## bit-exact 浮点约简顺序（舍入方向为 Θ_exec 设计选择）
///
/// 总 bp = commission + slippage + tax（固定加法顺序：commission → slippage → tax）。价格调整
/// = `base ± base·总成本bp/10000`，先算 bp 和，再乘 base，除 10000，**`round`（四舍五入到最近
/// tick）** 到 tick。买入成交价上调（更不利），卖出下调。
///
/// ★`round` 舍入方向 [设计选择,Θ_exec]：spec:51 **未钉死**费用折算的舍入方向。本实装固定用
/// `f64::round`（ties-to-even，与 `types::quantize` 的 `round` 一致，避免方向偏置）——这是
/// bit-exact 的确定选择，非 spec 推论。若需与其他实装对齐到 bit，须共享此舍入约定。
///
/// 边界条件：所有 bp=0（如 tax 默认 0）⟹ 该项不偏移。slippage 是唯一买卖**不对称**项
/// （买加卖减体现执行滑点方向）；commission/tax 是对称成本。
pub fn apply_fees(base: Tick, side: FillSide, config: &ExecConfig) -> Tick {
    // 成本 bp 之和（固定加法顺序：commission → slippage → tax）。
    let total_bps = config.commission_bps + config.slippage_bps + config.tax_bps;
    let adj = (base as f64) * total_bps / 10_000.0; // base·bp/10000
    match side {
        // 买入：成交价上调（成本使买价更高 = 更不利）。
        FillSide::Buy => base + adj.round() as Tick,
        // 卖出：成交价下调（成本使卖价更低 = 更不利）。
        FillSide::Sell => base - adj.round() as Tick,
    }
}

/// 止损成交价（reference-theta-v0.md:52）。
///
/// 多头止损（`FillSide::Sell` 平多 / 卖出方向，止损在下方）：
/// - bar open **低于**止损（`open < stop`）⟹ 跳空击穿，按 **open** 出（更坏的实际开盘价）；
/// - 否则 bar low **触及**止损（`low <= stop`）⟹ 按 **stop** 出；
/// - bar 未触及（`low > stop`）⟹ 不成交（`None`）。
///
/// 空头止损（`FillSide::Buy` 平空 / 买入方向，止损在上方，镜像）：
/// - bar open **高于**止损（`open > stop`）⟹ 按 **open** 出；
/// - 否则 bar high **触及**止损（`high >= stop`）⟹ 按 **stop** 出；
/// - bar 未触及（`high < stop`）⟹ 不成交。
///
/// `exit_side` = 平仓方向：平多头 = Sell（止损在下）；平空头 = Buy（止损在上）。
///
/// 边界条件：`untradable` bar 在调用前已过滤（本函数假设 bar 可交易）。返回的是**原始
/// 止损成交价**（未含费用——费用由 `apply_fees` 单独施加，保持成交价与费用解耦）。
pub fn stop_fill_price(bar: &Bar, stop: Tick, exit_side: FillSide) -> Option<Tick> {
    match exit_side {
        // 平多头（卖出），止损在下方。
        FillSide::Sell => {
            if bar.open < stop {
                Some(bar.open) // 跳空低开击穿，按 open 出
            } else if bar.low <= stop {
                Some(stop) // low 触及止损，按 stop 出
            } else {
                None // 未触及
            }
        }
        // 平空头（买入），止损在上方（镜像）。
        FillSide::Buy => {
            if bar.open > stop {
                Some(bar.open) // 跳空高开击穿，按 open 出
            } else if bar.high >= stop {
                Some(stop) // high 触及止损，按 stop 出
            } else {
                None
            }
        }
    }
}

/// 订单冲突排序键（reference-theta-v0.md:54）。
///
/// 冲突顺序（升序排，越小越先执行）：
/// 1. **止损/退出先于开仓**：`exit_first`（退出/止损=0，开仓=1）。
/// 2. **高 level 先于低 level**：`level` 降序 ⟹ 用 `u32::MAX - level` 升序（高 level 小键）。
/// 3. **同 level 按 1/2/3 类**：`bsp_class`（1 类=1，2 类=2，3 类=3）。
/// 4. **仍平局按 (timestamp, source_index)**：`timestamp` 然后 `source_index`。
/// 5. **声部深度终局裁决**：`depth`——保证两声部的键**不可能完全相同**（每声部深度唯一），
///    使排序结果**不依赖输入顺序**（订单唯一性，StrategyFamily.lean `given_theta_total_unique`）。
///
/// 这是 spec:54 的字典序裁决——保证同一时刻多订单的执行顺序**唯一确定**（无歧义）。
/// 对应 Nest.lean `selOrder` 风格的确定平局裁决（已确认结构不可回写，spec:16）。
///
/// ★`depth` 是 spec:54 五键之外的**第六键**：spec:54 的四级裁决（exit/level/class/
/// (ts,idx)）在多声部赋格中**可能不足以全序**——两个不同深度的声部若共享同一触发结构
/// （同 level/class/timestamp/source_index，如依赖同一中枢），前五键相同。声部深度在赋格
/// 树中唯一（`Origin.VoiceTree.depth_decreasing` 严格递减），作终局键保证全序、订单流不依赖输入顺序。
/// 这是 spec:54「仍平局」兜底的结构补全（非新规则，是把「平局」裁到底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConflictKey {
    /// 退出/止损 = 0，开仓 = 1（止损/退出先，spec:54）。
    pub exit_first: u8,
    /// 高 level 先：存为 `u32::MAX - level`（高 level → 小键 → 先排）。
    pub level_desc: u32,
    /// 同 level 按 1/2/3 类（类号 1/2/3）。
    pub bsp_class: u8,
    /// 平局裁决：timestamp 升序。
    pub timestamp: i64,
    /// 平局裁决：source_index 升序（触发结构对象的原始 bar 序号，spec:16）。
    pub source_index: usize,
    /// 终局裁决：声部深度升序（保证全序，订单流不依赖输入顺序）。
    pub depth: u32,
}

impl ConflictKey {
    /// 构造冲突排序键（reference-theta-v0.md:54 字典序 + 声部深度终局裁决）。
    ///
    /// `is_exit` = 是否退出/止损订单（true ⟹ 先于开仓）；`level` = 订单所属决策级别
    /// （高 level 先）；`bsp_class` ∈ {1,2,3}（同 level 内 1/2/3 类顺序）；
    /// `timestamp`/`source_index` = 触发结构对象的平局键（spec:16）；
    /// `depth` = 声部深度（终局裁决，保证两键不全等）。
    pub fn new(
        is_exit: bool,
        level: u32,
        bsp_class: u8,
        timestamp: i64,
        source_index: usize,
        depth: u32,
    ) -> ConflictKey {
        ConflictKey {
            exit_first: if is_exit { 0 } else { 1 },
            level_desc: u32::MAX - level, // 高 level → 小键
            bsp_class,
            timestamp,
            source_index,
            depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(idx: usize, ts: i64, o: Tick, h: Tick, l: Tick, c: Tick, untradable: bool) -> Bar {
        Bar {
            source_index: idx,
            timestamp: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: if untradable { 0 } else { 100 },
            untradable,
        }
    }

    /// 延迟成交（spec:50）：默认延迟 1 根，下一根 open 成交。
    #[test]
    fn fill_delay_one_bar() {
        let cfg = ExecConfig::default(); // entry_delay_bars=1
        let bars = vec![
            bar(0, 0, 100, 110, 90, 105, false),
            bar(1, 1, 106, 112, 104, 108, false),
        ];
        // 信号在 index 0 ⟹ 成交在 index 1。
        assert_eq!(fill_bar_index(0, &bars, &cfg), Some(1));
    }

    /// 延迟成交顺延不可交易 bar（spec:50「等下一可交易 open」）。
    #[test]
    fn fill_skips_untradable_bars() {
        let cfg = ExecConfig::default();
        let bars = vec![
            bar(0, 0, 100, 110, 90, 105, false),
            bar(1, 1, 0, 0, 0, 0, true), // 延迟落点不可交易
            bar(2, 2, 106, 112, 104, 108, false),
        ];
        // 信号 0 ⟹ 延迟落点 1 不可交易 ⟹ 顺延到 2。
        assert_eq!(fill_bar_index(0, &bars, &cfg), Some(2));
    }

    /// 延迟成交越界 ⟹ None（无下一根）。
    #[test]
    fn fill_out_of_range_yields_none() {
        let cfg = ExecConfig::default();
        let bars = vec![bar(0, 0, 100, 110, 90, 105, false)];
        // 信号在最后一根 ⟹ 延迟 1 越界。
        assert_eq!(fill_bar_index(0, &bars, &cfg), None);
        // 末尾全不可交易 ⟹ None。
        let bars2 = vec![
            bar(0, 0, 100, 110, 90, 105, false),
            bar(1, 1, 0, 0, 0, 0, true),
        ];
        assert_eq!(fill_bar_index(0, &bars2, &cfg), None);
    }

    /// 费用（spec:51）：买入加、卖出减；默认 commission=1,slippage=2,tax=0 bp ⟹ 3bp。
    #[test]
    fn fees_buy_adds_sell_subtracts() {
        let cfg = ExecConfig::default(); // 1+2+0 = 3 bp
        // base=1_000_000 tick：adj = 1e6 * 3/10000 = 300。
        assert_eq!(apply_fees(1_000_000, FillSide::Buy, &cfg), 1_000_300);
        assert_eq!(apply_fees(1_000_000, FillSide::Sell, &cfg), 999_700);
    }

    /// 止损成交多头（spec:52）：open<stop 按 open；否则 low 触及按 stop；未触及 None。
    #[test]
    fn stop_fill_long_rules() {
        let stop = 90;
        // open 低于止损（跳空击穿）：按 open 出。
        let b_gap = bar(0, 0, 85, 95, 80, 88, false);
        assert_eq!(stop_fill_price(&b_gap, stop, FillSide::Sell), Some(85));
        // open 未破，low 触及止损：按 stop 出。
        let b_touch = bar(0, 0, 100, 105, 88, 95, false);
        assert_eq!(stop_fill_price(&b_touch, stop, FillSide::Sell), Some(90));
        // low 未触及止损：不成交。
        let b_safe = bar(0, 0, 100, 105, 95, 102, false);
        assert_eq!(stop_fill_price(&b_safe, stop, FillSide::Sell), None);
    }

    /// 止损成交空头（spec:52 镜像）：open>stop 按 open；否则 high 触及按 stop；未触及 None。
    #[test]
    fn stop_fill_short_rules() {
        let stop = 110;
        // open 高于止损（跳空击穿）：按 open。
        let b_gap = bar(0, 0, 115, 120, 112, 118, false);
        assert_eq!(stop_fill_price(&b_gap, stop, FillSide::Buy), Some(115));
        // open 未破，high 触及止损：按 stop。
        let b_touch = bar(0, 0, 100, 112, 98, 105, false);
        assert_eq!(stop_fill_price(&b_touch, stop, FillSide::Buy), Some(110));
        // high 未触及：不成交。
        let b_safe = bar(0, 0, 100, 108, 95, 102, false);
        assert_eq!(stop_fill_price(&b_safe, stop, FillSide::Buy), None);
    }

    /// 冲突排序（spec:54）：止损先于开仓；高 level 先；同 level 1/2/3；平局 ts/idx；终局 depth。
    #[test]
    fn conflict_key_ordering() {
        // 止损（exit）先于开仓。
        let exit = ConflictKey::new(true, 3, 1, 100, 5, 0);
        let open = ConflictKey::new(false, 3, 1, 100, 5, 0);
        assert!(exit < open);
        // 高 level 先（level 5 先于 level 3）。
        let hi = ConflictKey::new(false, 5, 1, 100, 5, 0);
        let lo = ConflictKey::new(false, 3, 1, 100, 5, 0);
        assert!(hi < lo);
        // 同 level 按 1/2/3 类（1 类先于 3 类）。
        let c1 = ConflictKey::new(false, 3, 1, 100, 5, 0);
        let c3 = ConflictKey::new(false, 3, 3, 100, 5, 0);
        assert!(c1 < c3);
        // 平局按 timestamp 升序。
        let t_early = ConflictKey::new(false, 3, 1, 100, 5, 0);
        let t_late = ConflictKey::new(false, 3, 1, 200, 5, 0);
        assert!(t_early < t_late);
        // 次终局按 source_index 升序。
        let i_small = ConflictKey::new(false, 3, 1, 100, 1, 0);
        let i_large = ConflictKey::new(false, 3, 1, 100, 9, 0);
        assert!(i_small < i_large);
        // 终局按 depth 升序（前五键全同，仅 depth 区分 ⟹ 全序，无平局）。
        let d_shallow = ConflictKey::new(false, 3, 1, 100, 5, 0);
        let d_deep = ConflictKey::new(false, 3, 1, 100, 5, 2);
        assert!(d_shallow < d_deep);
        assert_ne!(d_shallow, d_deep); // 两声部键不可能完全相同
    }

    /// 冲突排序优先级层级（exit_first 主导 level 主导 class）：止损低 level 仍先于开仓高 level。
    #[test]
    fn conflict_exit_dominates_level() {
        // 止损在 level 1（低）vs 开仓在 level 5（高）：止损仍先（exit_first 是第一键）。
        let exit_low = ConflictKey::new(true, 1, 3, 100, 9, 2);
        let open_high = ConflictKey::new(false, 5, 1, 100, 1, 0);
        assert!(exit_low < open_high);
    }
}
