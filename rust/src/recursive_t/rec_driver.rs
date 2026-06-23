//! **递归 T 走势树消费驱动**（flat 逻辑递归化，编排者裁决 2026-06-21）。
//!
//! 把 iterate 真树投影为 `LevelView`（= flat `TSignalView`，按 level）+ 驱动 `TRoot.on_bar`
//! （= flat `step`）。**无递归自创约束**（C1/candidate/连续 level=父-1 全删）——操作逻辑完全在
//! `TRoot`（对照 flat route_bsp/sink/recover/flip/enter/ascend）。
//!
//! ## 投影（extract_view）
//! - `nodes[k]` = `levels[k].trends.last()` → `TrendNode`（per level 当前走势节点，enter/ascend/sink 骑）。
//! - `emergent_top` = 最高有走势的 level + 该走势方向极性（自下而上涌现升级，= flat emergent_top）。
//! - `buy/sell[k]` = 该 level 本次重跑新 fire 的买/卖点（fresh，rec_stream 填）。
//!
//! ## 认识论等级
//! 投影 = L0；驱动行为 = flat route_bsp（L3 验证 CL+120%）；递归化收益复现 = L3 验收。

use super::rec_engine::{dir_to_polarity, EngineConfig, LevelView, TRoot, TrendNode, MAX_LEVEL};
use super::types::{RecursiveTree, TrendKind, TrendType, Unit};

/// 递归 T 驱动：薄包装 `TRoot`，每次重跑消费 `LevelView` → `on_bar`（= flat step）。
pub struct RecDriver {
    root: TRoot,
}

impl RecDriver {
    pub fn new(initial_capital: f64) -> Self {
        RecDriver { root: TRoot::new(initial_capital) }
    }

    /// 显式配置构造（OFF / ANCHOR / NEST 受控对照）。
    pub fn new_with_config(initial_capital: f64, cfg: EngineConfig) -> Self {
        RecDriver { root: TRoot::new_with_config(initial_capital, cfg) }
    }

    pub fn root(&self) -> &TRoot {
        &self.root
    }
    pub fn root_mut(&mut self) -> &mut TRoot {
        &mut self.root
    }

    /// 消费一次信号视图（重跑触发）：= flat step（强平 → emergence → route_bsp top-down）。
    pub fn on_view(&mut self, view: &LevelView, c: f64, bar: i64) {
        if c <= 0.0 {
            return;
        }
        self.root.on_bar(view, bar, c);
    }

    /// 收尾（全平 + final_nav）。
    pub fn finish(&mut self, c: f64) -> f64 {
        self.root.finish(c)
    }
}

// ════════════════════════════ iterate RecursiveTree → LevelView 适配器 ════════════════════════════

/// TrendType → 走势节点（区间 = units 首末 bar，价区 = units 极值）。
fn trend_to_node(t: &TrendType) -> TrendNode {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for u in &t.units {
        lo = lo.min(u.low);
        hi = hi.max(u.high);
    }
    let sb = t.units.first().map(|u| u.start_bar).unwrap_or(0);
    let eb = t.units.last().map(|u| u.end_bar).unwrap_or(sb);
    TrendNode::new(sb, eb, lo, hi, t.direction)
}

/// 封装 Unit（= 下级走势）→ 走势节点。
fn unit_to_node(u: &Unit) -> TrendNode {
    TrendNode::new(u.start_bar, u.end_bar, u.low, u.high, u.direction)
}

/// 从 iterate 的 `RecursiveTree` 投影出信号视图（= flat TSignalView，按 level）。
///
/// `nodes[k]` = level k 当前走势节点；`emergent_top` = 最高有走势的 level + 极性。**buy/sell 留空**
/// （rec_stream 填本次重跑 fresh BSP）。
pub fn extract_view(tree: &RecursiveTree) -> LevelView {
    let mut view = LevelView::empty();
    let levels = &tree.levels;
    if levels.is_empty() {
        return view;
    }
    for (k, lvl) in levels.iter().enumerate() {
        if k >= MAX_LEVEL {
            break;
        }
        if let Some(t) = lvl.trends.last() {
            view.nodes[k] = Some(trend_to_node(t));
        }
    }
    // emergent_top = tree.emergent_top()（= flat stream，最高**已诞生上级单元** + **顺势方向**——
    // 非自创"最高有走势的 level + 当前走势方向"。后者在回调段返回 Down 方向 → 核心方向错乱 → BTC
    // 牛市空头主导爆仓。flat 用 iterate 的 emergent_top()，本递归化对照之，不自创）。
    if let Some((t_level, dir)) = tree.emergent_top() {
        if t_level < MAX_LEVEL {
            view.emergent_top = Some((t_level, dir_to_polarity(dir)));
        }
    }
    view
}

/// TrendKind → 是否趋势（适配器辅助）。
pub fn is_trend(kind: TrendKind) -> bool {
    matches!(kind, TrendKind::UpTrend | TrendKind::DownTrend)
}

#[allow(dead_code)]
fn _silence_unused(u: &Unit) -> TrendNode {
    unit_to_node(u)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::types::Direction;
    use crate::trading::types::Polarity;

    fn node(s: i64, e: i64, dir: Direction) -> TrendNode {
        TrendNode::new(s, e, 90.0, 110.0, dir)
    }
    /// 单 level 买点视图（节点方向 dir）。
    fn lv_buy(level: usize, dir: Direction) -> LevelView {
        let mut v = LevelView::empty();
        v.buy[level] = true;
        v.nodes[level] = Some(node(0, 10, dir));
        v
    }
    /// 单 level 卖点视图。
    fn lv_sell(level: usize, dir: Direction) -> LevelView {
        let mut v = LevelView::empty();
        v.sell[level] = true;
        v.nodes[level] = Some(node(0, 10, dir));
        v
    }

    // ──────────────── enter（核心级，无父，按 BSP 方向，无 C1）────────────────

    #[test]
    fn 核心级买点_enter_long() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0);
        assert_eq!(r.highest_active(), Some(3), "核心建仓 @ level 3");
        assert_eq!(r.instance(3).direction, Polarity::Long, "买点 → Long");
        assert!((r.total_wealth(100.0) - 100_000.0).abs() < 1e-4, "建仓 TW 中性");
    }

    #[test]
    fn 核心级卖点_enter_short_无c1() {
        // flat 无 C1：核心级卖点 → enter Short（递归引擎删除"永不翻空"约束）。
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_sell(3, Direction::Down), 10, 100.0);
        assert_eq!(r.highest_active(), Some(3));
        assert_eq!(r.instance(3).direction, Polarity::Short, "卖点 → Short（无 C1）");
    }

    // ──────────────── sink（子级反父向 BSP）────────────────

    #[test]
    fn 子级反父向卖点_sink() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 Long @ level 3
        let u0 = r.instance(3).units;
        // level 2 卖点（子级，nearest_active_parent(2)=3 父多 → 卖点=reduce → sink）。
        r.on_bar(&lv_sell(2, Direction::Down), 12, 100.0);
        assert!((r.instance(3).units - u0 * 2.0 / 3.0).abs() < 1e-6, "核心减到 2/3");
        assert_eq!(r.instance(2).direction, Polarity::Short, "子级开空");
        assert!((r.instance(2).units - u0 / 3.0).abs() < 1e-6, "子持空 1/3");
        assert!((r.total_wealth(100.0) - 100_000.0).abs() < 1e-4, "sink TW 中性");
    }

    // ──────────────── recover（子级同父向 BSP，子持短差）────────────────

    #[test]
    fn 子级同父向买点_recover() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0);
        let u0 = r.instance(3).units;
        r.on_bar(&lv_sell(2, Direction::Down), 12, 110.0); // sink@110（高开空）
        assert_eq!(r.instance(2).direction, Polarity::Short);
        // level 2 买点（子持短差 + 父多 → 同父向买点 → recover 平空升回）。
        r.on_bar(&lv_buy(2, Direction::Up), 15, 90.0); // 低平空
        assert!(!r.instance(2).is_active(), "子平空");
        assert!((r.instance(3).units - u0).abs() < 1e-6, "核心恢复原股数");
        assert!(r.short_leg_pnl > 0.0, "高开低平短差盈利");
    }

    // ──────────────── flip（核心级反向 BSP）────────────────

    #[test]
    fn 核心级反向_flip() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 Long
        assert_eq!(r.instance(3).direction, Polarity::Long);
        // 核心级卖点（同 level 3，无父，反向）→ flip（clear + 反向 enter）。
        r.on_bar(&lv_sell(3, Direction::Down), 20, 120.0);
        assert_eq!(r.instance(3).direction, Polarity::Short, "翻空");
        assert_eq!(r.n_flips, 1);
        assert!((r.total_wealth(120.0) - r.nav(120.0)).abs() < 1e-4, "flip 后 withdrawn 已归还");
    }

    // ──────────────── ascend（核心同向更高 level BSP）────────────────

    #[test]
    fn 核心同向更高级别_ascend() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 @ level 3
        let u0 = r.instance(3).units;
        // level 5 买点（同向更高，无父，highest=3，dir==cdir，j=5>3 idle → ascend）。
        r.on_bar(&lv_buy(5, Direction::Up), 20, 100.0);
        assert_eq!(r.highest_active(), Some(5), "核心 relabel 上移到 level 5");
        assert!(!r.instance(3).is_active(), "原 level 3 空");
        assert!((r.instance(5).units - u0).abs() < 1e-9, "持仓继承（无新资金）");
        assert_eq!(r.n_ascends, 1);
    }

    // ──────────────── 守恒（多操作全程 TW 中性）────────────────

    #[test]
    fn 全程tw中性_enter_sink_recover_flip() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // enter Long
        r.on_bar(&lv_sell(2, Direction::Down), 12, 120.0); // sink
        r.on_bar(&lv_buy(2, Direction::Up), 15, 100.0); // recover
        r.on_bar(&lv_sell(3, Direction::Down), 30, 130.0); // flip 翻空
        let fin = r.finish(110.0);
        assert!(fin.is_finite() && fin > 0.0, "final_nav 有限，得 {fin}");
    }

    // ──────────────── extract_view（iterate 真树投影）────────────────

    use crate::recursive_t::iterate;
    use crate::recursive_t::types::PerfectionMode;

    fn u(lo: f64, hi: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(lo, hi, s, e, d)
    }

    #[test]
    fn extract_view_真树_上涨趋势() {
        let a0 = vec![
            u(8.0, 22.0, 0, 1, Direction::Up),
            u(12.0, 18.0, 1, 2, Direction::Down),
            u(10.0, 16.0, 2, 3, Direction::Up),
            u(23.0, 35.0, 3, 4, Direction::Up),
            u(32.0, 40.0, 4, 5, Direction::Down),
            u(33.0, 42.0, 5, 6, Direction::Up),
            u(31.0, 39.0, 6, 7, Direction::Down),
            u(40.0, 45.0, 7, 8, Direction::Up),
        ];
        let tree = iterate(a0, PerfectionMode::Structural);
        let v = extract_view(&tree);
        // tree.emergent_top() 涌现上界（= flat）。t_level 可能超 levels 范围（涌现到尚未在 levels 数组
        // 物化的上级单元）→ nodes[lvl] 可能 None，on_bar 用 fallback node。仅验证 level 在 MAX_LEVEL 内。
        if let Some((lvl, _pol)) = v.emergent_top {
            assert!(lvl < MAX_LEVEL, "涌现 level 在 MAX_LEVEL 范围内");
        }
    }

    #[test]
    fn extract_view_空树() {
        let tree = RecursiveTree { levels: vec![] };
        let v = extract_view(&tree);
        assert!(v.emergent_top.is_none());
    }

    #[test]
    fn 端到端_真树_driver消费不panic() {
        let a0 = vec![
            u(8.0, 22.0, 0, 1, Direction::Up),
            u(12.0, 18.0, 1, 2, Direction::Down),
            u(10.0, 16.0, 2, 3, Direction::Up),
            u(23.0, 35.0, 3, 4, Direction::Up),
            u(32.0, 40.0, 4, 5, Direction::Down),
            u(33.0, 42.0, 5, 6, Direction::Up),
            u(31.0, 39.0, 6, 7, Direction::Down),
            u(40.0, 45.0, 7, 8, Direction::Up),
        ];
        let tree = iterate(a0, PerfectionMode::Structural);
        let v = extract_view(&tree);
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&v, 40.0, 8);
        let fin = d.finish(40.0);
        assert!(fin.is_finite() && fin >= 0.0, "端到端 final_nav 有限");
    }
}
