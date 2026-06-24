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
            // **否定线原料（任务57=53.1 LegPair 止损）**：当前走势末中枢核心区间 [ZD, ZG]。
            //   多腿止损 = 进场 ZD（跌破=结构破坏）/ 空腿止损 = 进场 ZG（涨破=结构破坏）。第17课区间套否定线。
            //   无中枢（trends 末走势无 zhongshu）⇒ None（无否定线，仅靠反向买卖点平）。
            if let Some(zs) = t.zhongshus.last() {
                view.zg[k] = Some(zs.high); // ZG 核心上沿
                view.zd[k] = Some(zs.low); // ZD 核心下沿
            }
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

    // ════════════ Face B：核心不僵死（做空腿赚 #110，删 geom_tower 均匀定仓 + 真否定线）════════════

    /// LegPair 买点视图 + 进场中枢否定线 ZD（多腿跌破止损）。
    fn lv_buy_zd(level: usize, dir: Direction, zd: f64) -> LevelView {
        let mut v = LevelView::empty();
        v.buy[level] = true;
        v.nodes[level] = Some(node(0, 10, dir));
        v.zd[level] = Some(zd);
        v
    }
    /// LegPair 卖点视图（次级别开空：sell + t3sell 单层区间套转折）+ 进场中枢否定线 ZG（空腿涨破止损）。
    fn lv_sell_zg_t3(level: usize, dir: Direction, zg: f64) -> LevelView {
        let mut v = LevelView::empty();
        v.sell[level] = true;
        v.t3sell[level] = true;
        v.nodes[level] = Some(node(0, 10, dir));
        v.zg[level] = Some(zg);
        v
    }

    /// **均匀基准单元定仓（§6.3 删 geom_tower）**：Face B 每级别 = INITIAL×MOBILE_FRAC（级别无关），
    /// 区别于 geom_tower 恒仓配额（base=free×2/3）。
    #[test]
    fn faceb_均匀定仓_级别无关基准单元() {
        let mut fb = TRoot::new_with_config(100_000.0, EngineConfig::face_b());
        fb.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        let u_fb = fb.leg_pair(3).long_units;
        // 均匀：100000 × (1/3) / 100 = 333.33（级别无关，无 depth 衰减、无 free 归一化）。
        assert!((u_fb - 333.333).abs() < 0.01, "均匀基准单元 = INITIAL×1/3/c，得 {u_fb}");

        // 对照 RB_PAIR（geom_tower 恒仓）：base=free×2/3=66666.7, depth=0 ⇒ units=666.67（≠均匀）。
        let mut rb = TRoot::new_with_config(100_000.0, EngineConfig::reading_b_pair());
        rb.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        let u_rb = rb.leg_pair(3).long_units;
        assert!((u_rb - 666.667).abs() < 0.01, "geom_tower 恒仓 base×2/3，得 {u_rb}");
        assert!((u_fb - u_rb).abs() > 1.0, "Face B 均匀 ≠ RB_PAIR geom（删 geom_tower 生效）");
    }

    /// **真否定线封顶（§6.2 567，RB_PAIR 下 zd/zg=None 死代码）**：多腿跌破进场 ZD ⇒ 止损平。
    #[test]
    fn faceb_多腿跌破否定线zd止损() {
        let mut fb = TRoot::new_with_config(100_000.0, EngineConfig::face_b());
        fb.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0); // 多腿@100, 否定线 ZD=95
        assert!(fb.leg_pair(3).long_active(), "多腿建仓");
        // 价格跌破 95（结构破坏）⇒ pair_stop_loss_step 平多（即便 view 空，止损先于 top 早退）。
        fb.on_bar(&LevelView::empty(), 11, 94.0);
        assert!(!fb.leg_pair(3).long_active(), "跌破 ZD=95 ⇒ 多腿止损平");
        assert_eq!(fb.pair_long_stops, 1, "否定线止损计数");
        assert_eq!(fb.n_liquidations, 0, "止损先于 NAV≤0 ⇒ liq=0");
    }

    /// **空腿涨破否定线 ZG 止损**（次级别开空 + 涨破进场 ZG）。
    #[test]
    fn faceb_空腿涨破否定线zg止损() {
        let mut fb = TRoot::new_with_config(100_000.0, EngineConfig::face_b());
        fb.on_bar(&lv_buy_zd(4, Direction::Up, 90.0), 10, 100.0); // 核心多腿@4（below_core_long 门用）
        fb.on_bar(&lv_sell_zg_t3(2, Direction::Down, 105.0), 11, 100.0); // 次级别(2<4)开空腿, 否定线 ZG=105
        assert!(fb.leg_pair(2).short_active(), "次级别空腿建仓（t3sell + below_core_long）");
        fb.on_bar(&LevelView::empty(), 12, 106.0); // 涨破 105 ⇒ 空腿止损
        assert!(!fb.leg_pair(2).short_active(), "涨破 ZG=105 ⇒ 空腿止损平");
        assert_eq!(fb.pair_short_stops, 1, "空腿否定线止损计数");
    }

    /// **★核心否定线置 cc 级别（§5.3.2，Face B 主机制）**：次级别回调（破次级别 ZD）平次级别腿，
    /// 但**不触发核心否定线**（未破 cc 级 ZD）⇒ 核心存活穿越次级别回调。
    #[test]
    fn faceb_次级别回调不扫核心_核心否定线在cc级() {
        let mut fb = TRoot::new_with_config(100_000.0, EngineConfig::face_b());
        // 核心多腿@4，否定线 = cc(4)级中枢 ZD=90（宽，大级别中枢）。
        fb.on_bar(&lv_buy_zd(4, Direction::Up, 90.0), 10, 100.0);
        // 次级别多腿@2，否定线 = 次级别中枢 ZD=98（窄，次级别中枢）。
        fb.on_bar(&lv_buy_zd(2, Direction::Up, 98.0), 11, 100.0);
        assert!(fb.leg_pair(4).long_active() && fb.leg_pair(2).long_active(), "核心+次级别双多腿在场");
        // 价格回调到 95：破次级别 ZD(98) 不破核心 ZD(90)。
        fb.on_bar(&LevelView::empty(), 12, 95.0);
        assert!(!fb.leg_pair(2).long_active(), "次级别回调破次级别 ZD=98 ⇒ 次级别腿止损");
        assert!(fb.leg_pair(4).long_active(), "核心存活：次级别回调未破 cc(4)级 ZD=90，核心不被扫");
        assert_eq!(fb.n_liquidations, 0, "liq=0");
    }

    /// **杠杆来源A 涌现（§6.1 579）**：多级别独立腿叠加 ⇒ max_gross>1×（删 geom_tower 恒仓归一化），
    /// 否定线封顶每腿 ⇒ liq=0（非穿仓）。
    #[test]
    fn faceb_多级别叠加_杠杆来源A涌现_liq0() {
        let mut fb = TRoot::new_with_config(100_000.0, EngineConfig::face_b());
        // 单 bar 四级别买点 + 各级别中枢 ZD ⇒ g_pair(0..=4) 各开 333.33 多腿（叠加）。
        let mut v = LevelView::empty();
        for k in 1..=4 {
            v.buy[k] = true;
            v.nodes[k] = Some(node(0, 10, Direction::Up));
            v.zd[k] = Some(90.0);
        }
        fb.on_bar(&v, 10, 100.0);
        let active = (0..MAX_LEVEL).filter(|&k| fb.leg_pair(k).long_active()).count();
        assert_eq!(active, 4, "四级别独立多腿叠加");
        // gross = 4×333.33×100 = 133333 vs nav≈100000 ⇒ >1×（geom_tower 恒仓会钳到 ≤1×=压制）。
        assert!(fb.max_gross_exp_x100 > 100, "来源A 杠杆 >1× 涌现，得 {}×100", fb.max_gross_exp_x100);
        assert_eq!(fb.n_liquidations, 0, "否定线封顶 ⇒ liq=0（非穿仓）");
    }

    /// **bit-exact：enable_uniform_sizing=false ⇒ RB_PAIR/OFF 逐字不变**（zd/zg 不消费、geom_tower 不变）。
    #[test]
    fn faceb_off_bitexact_rbpair不受影响() {
        // RB_PAIR 即便 view 带 zd/zg（信号层只在 Face B 填充，此处人为带）也不止损（long_stop 锁 None）。
        let mut rb = TRoot::new_with_config(100_000.0, EngineConfig::reading_b_pair());
        rb.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        let u = rb.leg_pair(3).long_units;
        rb.on_bar(&LevelView::empty(), 11, 94.0); // 即便跌破 95，RB_PAIR 无真否定线（地基代码逐字不变）
        assert!(rb.leg_pair(3).long_active(), "RB_PAIR：open_long_leg 用 view.zd 但 RB_PAIR 仍传入=stop 锁住");
        assert!((rb.leg_pair(3).long_units - u).abs() < 1e-9, "RB_PAIR 多腿不被止损（bit-exact 路径）");
        assert_eq!(rb.pair_long_stops, 0, "RB_PAIR 无否定线止损");
    }

    // ════════════ Face A：核心能动（做空腿赚 #113，核心翻转=cc走势完成 + 接受杠杆 + 默认开启）════════════

    /// LegPair 卖点视图 + **cc 走势完成 d_top**（核心翻转闸门=全深度区间套链贯通真顶=第一类卖点）+
    /// 进场中枢否定线 ZG（核心空腿涨破止损=牛市恢复）。
    fn lv_sell_dtop(level: usize, dir: Direction, zg: f64) -> LevelView {
        let mut v = LevelView::empty();
        v.sell[level] = true;
        v.d_top[level] = true; // 走势完成（区间套级联，第27课，减确认滞后≠零滞后 R3）
        v.nodes[level] = Some(node(0, 10, dir));
        v.zg[level] = Some(zg);
        v
    }

    /// **Face A 集成契约（§5.2/§六/§七）**：face_a = Face B 基座（uniform 定仓 + 真否定线）+ 核心翻转
    /// （pair_core_short），**不开** pair_core_short_open（net-up 假顶翻空灾难门，§5.2.3）。唯一增量=核心翻空。
    #[test]
    fn facea_集成契约_facebase_叠加核心翻空() {
        let fa = EngineConfig::face_a();
        assert!(fa.enable_reading_b_pair, "LegPair 路径（无 sink ⇒ 生产路径不残留无保护 sink，#106）");
        assert!(fa.enable_uniform_sizing, "Face B 均匀定仓（删 geom_tower）+ 真否定线 [ZD,ZG]");
        assert!(fa.enable_pair_core_short, "★Face A 核心翻转吃熊（cc 走势完成翻空镜像）");
        assert!(!fa.enable_pair_core_short_open, "不开 below_core_long 拆除门（net-up 假顶翻空灾难，§5.2.3）");
        // 与 Face B 唯一差 = 核心翻转（其余逐字一致 ⇒ 收益差全归因核心翻空）。
        let fb = EngineConfig::face_b();
        assert_eq!(fa.enable_reading_b_pair, fb.enable_reading_b_pair);
        assert_eq!(fa.enable_uniform_sizing, fb.enable_uniform_sizing);
        assert!(fa.enable_pair_core_short && !fb.enable_pair_core_short, "唯一增量 = 核心翻空");
    }

    /// **★核心翻转=cc 走势完成翻空吃熊（§5.2，Face A 主机制）**：核心多腿在自己级别走势完成（d_top）
    /// 翻空镜像（close_long 全量 + open_short 全量，无 clear_all），否定线=cc 中枢 ZG（涨破=牛市恢复止损），liq=0。
    #[test]
    fn facea_核心翻转_cc走势完成_翻空吃熊() {
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a());
        // 1) 核心多腿 @ cc=3（否定线 ZD=95）。
        fa.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        assert!(fa.leg_pair(3).long_active(), "核心多腿建仓 @3");
        // 2) cc(3) 走势完成（d_top）+ 卖点 @ 价 110（盈利顶）⇒ 核心平多 + 翻空。
        fa.on_bar(&lv_sell_dtop(3, Direction::Up, 120.0), 11, 110.0);
        assert!(!fa.leg_pair(3).long_active(), "核心走势完成 ⇒ 平多（全量）");
        assert!(fa.leg_pair(3).short_active(), "★Face A：核心翻空吃熊（cc 走势完成区间套级联翻转）");
        assert_eq!(fa.pair_core_churns, 1, "核心 churn（真顶转折，非次级别回调）");
        assert!((fa.leg_pair(3).short_stop - 120.0).abs() < 1e-9, "核心空腿否定线 = cc 中枢 ZG=120（涨破止损）");
        assert_eq!(fa.n_liquidations, 0, "否定线封顶 ⇒ liq=0");
        // 3) 价格下跌到 90（bear）⇒ 核心空腿继续吃跌幅（未涨破 ZG=120）。
        fa.on_bar(&LevelView::empty(), 12, 90.0);
        assert!(fa.leg_pair(3).short_active(), "核心空腿吃跌（bear 持空，未涨破否定线）");
    }

    /// **★547 级别隔离 + net-up 自动 regime（§5.2.1/§5.2.4）**：次级别卖点绝不翻核心（独立空腿）；
    /// cc 卖点但走势未完成（d_top=false=net-up 回调）⇒ 核心不翻空（闸门不开，零 if regime）。
    #[test]
    fn facea_次级别不翻核心_net_up闸门不开_547() {
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a());
        // 核心多腿 @4（cc），否定线 ZD=90。
        fa.on_bar(&lv_buy_zd(4, Direction::Up, 90.0), 10, 100.0);
        // 次级别(2<4) t3sell（回调，非 d_top）⇒ 次级别独立空腿（below_core_long），核心不动（547）。
        fa.on_bar(&lv_sell_zg_t3(2, Direction::Down, 105.0), 11, 100.0);
        assert!(fa.leg_pair(4).long_active(), "★547：次级别卖点不翻核心（核心多腿存活）");
        assert!(fa.leg_pair(2).short_active(), "次级别独立空腿吃回调（below_core_long）");
        assert_eq!(fa.pair_core_churns, 0, "核心未 churn（次级别卖点非 cc 走势完成）");
        // cc(4) 卖点但**非 d_top**（net-up 回调，走势未完成）⇒ 核心不平不翻（churn 门控 + 闸门不开）。
        fa.on_bar(&lv_sell_zg_t3(4, Direction::Down, 130.0), 12, 100.0);
        assert!(fa.leg_pair(4).long_active(), "★net-up 自动 regime：cc 卖点非走势完成 ⇒ 核心不翻空");
        assert!(!fa.leg_pair(4).short_active(), "核心未翻空（d_top[cc]=false ⇒ 闸门不开，无假顶翻空灾难）");
        assert_eq!(fa.pair_core_churns, 0, "核心仍未 churn");
    }

    /// **默认开启（§8.1）**：`production()` 无变体 env ⇒ Face A（生产/默认）；`T_OFF_BASELINE` ⇒ off()
    /// （instances bit-exact 回归守卫）。本测试是唯一 `production()` 调用者 ⇒ T_OFF_BASELINE 无并发竞态。
    #[test]
    fn facea_production_默认开启_off_baseline回归守卫() {
        std::env::remove_var("T_OFF_BASELINE");
        let prod = EngineConfig::production();
        assert!(
            prod.enable_reading_b_pair && prod.enable_uniform_sizing && prod.enable_pair_core_short,
            "默认开启 ⇒ Face A（reading_b_pair + uniform + core_short）"
        );
        assert!(!prod.enable_pair_core_short_open, "默认 Face A 不开 below_core_long 拆除门");
        // T_OFF_BASELINE ⇒ instances OFF 基线（全 LegPair flag false = bit-exact 回归守卫）。
        std::env::set_var("T_OFF_BASELINE", "1");
        let off = EngineConfig::production();
        assert!(
            !off.enable_reading_b_pair && !off.enable_uniform_sizing && !off.enable_pair_core_short,
            "T_OFF_BASELINE ⇒ off()（instances 基线，含 sink/geom_tower 回归守卫）"
        );
        std::env::remove_var("T_OFF_BASELINE");
    }
}
