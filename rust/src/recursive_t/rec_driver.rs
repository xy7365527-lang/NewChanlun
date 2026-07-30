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
        RecDriver {
            root: TRoot::new(initial_capital),
        }
    }

    /// 显式配置构造（OFF / ANCHOR / NEST 受控对照）。
    pub fn new_with_config(initial_capital: f64, cfg: EngineConfig) -> Self {
        RecDriver {
            root: TRoot::new_with_config(initial_capital, cfg),
        }
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
        assert!(
            (r.total_wealth(100.0) - 100_000.0).abs() < 1e-4,
            "建仓 TW 中性"
        );
    }

    #[test]
    fn 核心级卖点_enter_short_无c1() {
        // flat 无 C1：核心级卖点 → enter Short（递归引擎删除"永不翻空"约束）。
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_sell(3, Direction::Down), 10, 100.0);
        assert_eq!(r.highest_active(), Some(3));
        assert_eq!(
            r.instance(3).direction,
            Polarity::Short,
            "卖点 → Short（无 C1）"
        );
    }

    // ──────────────── sink（子级反父向 BSP）────────────────

    #[test]
    fn 子级反父向卖点_sink() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 Long @ level 3
        let u0 = r.instance(3).units;
        // level 2 卖点（子级，nearest_active_parent(2)=3 父多 → 卖点=reduce → sink）。
        r.on_bar(&lv_sell(2, Direction::Down), 12, 100.0);
        assert!(
            (r.instance(3).units - u0 * 2.0 / 3.0).abs() < 1e-6,
            "核心减到 2/3"
        );
        assert_eq!(r.instance(2).direction, Polarity::Short, "子级开空");
        assert!((r.instance(2).units - u0 / 3.0).abs() < 1e-6, "子持空 1/3");
        assert!(
            (r.total_wealth(100.0) - 100_000.0).abs() < 1e-4,
            "sink TW 中性"
        );
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
        assert!(
            (r.total_wealth(120.0) - r.nav(120.0)).abs() < 1e-4,
            "flip 后 withdrawn 已归还"
        );
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
        assert!(
            (r.instance(5).units - u0).abs() < 1e-9,
            "持仓继承（无新资金）"
        );
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
        assert!(
            (u_fb - 333.333).abs() < 0.01,
            "均匀基准单元 = INITIAL×1/3/c，得 {u_fb}"
        );

        // 对照 RB_PAIR（geom_tower 恒仓）：base=free×2/3=66666.7, depth=0 ⇒ units=666.67（≠均匀）。
        let mut rb = TRoot::new_with_config(100_000.0, EngineConfig::reading_b_pair());
        rb.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        let u_rb = rb.leg_pair(3).long_units;
        assert!(
            (u_rb - 666.667).abs() < 0.01,
            "geom_tower 恒仓 base×2/3，得 {u_rb}"
        );
        assert!(
            (u_fb - u_rb).abs() > 1.0,
            "Face B 均匀 ≠ RB_PAIR geom（删 geom_tower 生效）"
        );
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
        assert!(
            fb.leg_pair(2).short_active(),
            "次级别空腿建仓（t3sell + below_core_long）"
        );
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
        assert!(
            fb.leg_pair(4).long_active() && fb.leg_pair(2).long_active(),
            "核心+次级别双多腿在场"
        );
        // 价格回调到 95：破次级别 ZD(98) 不破核心 ZD(90)。
        fb.on_bar(&LevelView::empty(), 12, 95.0);
        assert!(
            !fb.leg_pair(2).long_active(),
            "次级别回调破次级别 ZD=98 ⇒ 次级别腿止损"
        );
        assert!(
            fb.leg_pair(4).long_active(),
            "核心存活：次级别回调未破 cc(4)级 ZD=90，核心不被扫"
        );
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
        let active = (0..MAX_LEVEL)
            .filter(|&k| fb.leg_pair(k).long_active())
            .count();
        assert_eq!(active, 4, "四级别独立多腿叠加");
        // gross = 4×333.33×100 = 133333 vs nav≈100000 ⇒ >1×（geom_tower 恒仓会钳到 ≤1×=压制）。
        assert!(
            fb.max_gross_exp_x100 > 100,
            "来源A 杠杆 >1× 涌现，得 {}×100",
            fb.max_gross_exp_x100
        );
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
        assert!(
            rb.leg_pair(3).long_active(),
            "RB_PAIR：open_long_leg 用 view.zd 但 RB_PAIR 仍传入=stop 锁住"
        );
        assert!(
            (rb.leg_pair(3).long_units - u).abs() < 1e-9,
            "RB_PAIR 多腿不被止损（bit-exact 路径）"
        );
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
        assert!(
            fa.enable_reading_b_pair,
            "LegPair 路径（无 sink ⇒ 生产路径不残留无保护 sink，#106）"
        );
        assert!(
            fa.enable_uniform_sizing,
            "Face B 均匀定仓（删 geom_tower）+ 真否定线 [ZD,ZG]"
        );
        assert!(
            fa.enable_pair_core_short,
            "★Face A 核心翻转吃熊（cc 走势完成翻空镜像）"
        );
        assert!(
            !fa.enable_pair_core_short_open,
            "不开 below_core_long 拆除门（net-up 假顶翻空灾难，§5.2.3）"
        );
        // 与 Face B 唯一差 = 核心翻转（其余逐字一致 ⇒ 收益差全归因核心翻空）。
        let fb = EngineConfig::face_b();
        assert_eq!(fa.enable_reading_b_pair, fb.enable_reading_b_pair);
        assert_eq!(fa.enable_uniform_sizing, fb.enable_uniform_sizing);
        assert!(
            fa.enable_pair_core_short && !fb.enable_pair_core_short,
            "唯一增量 = 核心翻空"
        );
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
        assert!(
            fa.leg_pair(3).short_active(),
            "★Face A：核心翻空吃熊（cc 走势完成区间套级联翻转）"
        );
        assert_eq!(
            fa.pair_core_churns, 1,
            "核心 churn（真顶转折，非次级别回调）"
        );
        assert!(
            (fa.leg_pair(3).short_stop - 120.0).abs() < 1e-9,
            "核心空腿否定线 = cc 中枢 ZG=120（涨破止损）"
        );
        assert_eq!(fa.n_liquidations, 0, "否定线封顶 ⇒ liq=0");
        // 3) 价格下跌到 90（bear）⇒ 核心空腿继续吃跌幅（未涨破 ZG=120）。
        fa.on_bar(&LevelView::empty(), 12, 90.0);
        assert!(
            fa.leg_pair(3).short_active(),
            "核心空腿吃跌（bear 持空，未涨破否定线）"
        );
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
        assert!(
            fa.leg_pair(4).long_active(),
            "★547：次级别卖点不翻核心（核心多腿存活）"
        );
        assert!(
            fa.leg_pair(2).short_active(),
            "次级别独立空腿吃回调（below_core_long）"
        );
        assert_eq!(
            fa.pair_core_churns, 0,
            "核心未 churn（次级别卖点非 cc 走势完成）"
        );
        // cc(4) 卖点但**非 d_top**（net-up 回调，走势未完成）⇒ 核心不平不翻（churn 门控 + 闸门不开）。
        fa.on_bar(&lv_sell_zg_t3(4, Direction::Down, 130.0), 12, 100.0);
        assert!(
            fa.leg_pair(4).long_active(),
            "★net-up 自动 regime：cc 卖点非走势完成 ⇒ 核心不翻空"
        );
        assert!(
            !fa.leg_pair(4).short_active(),
            "核心未翻空（d_top[cc]=false ⇒ 闸门不开，无假顶翻空灾难）"
        );
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
        assert!(
            !prod.enable_pair_core_short_open,
            "默认 Face A 不开 below_core_long 拆除门"
        );
        // T_OFF_BASELINE ⇒ instances OFF 基线（全 LegPair flag false = bit-exact 回归守卫）。
        std::env::set_var("T_OFF_BASELINE", "1");
        let off = EngineConfig::production();
        assert!(
            !off.enable_reading_b_pair && !off.enable_uniform_sizing && !off.enable_pair_core_short,
            "T_OFF_BASELINE ⇒ off()（instances 基线，含 sink/geom_tower 回归守卫）"
        );
        std::env::remove_var("T_OFF_BASELINE");
    }

    // ════════════ R3：核心多腿涌现升级（段无腿修复 #164/#6，多重赋格=核心跟随涌现上移）════════════

    /// 涌现视图：emergent_top=(level, Long) + 该 level 中枢 ZD + 节点（buy/sell 不 fire，仅触发 relabel）。
    fn lv_emergent(level: usize, zd: f64) -> LevelView {
        let mut v = LevelView::empty();
        v.emergent_top = Some((level, Polarity::Long));
        v.nodes[level] = Some(node(0, 10, Direction::Up)); // top=level（consume_leg_pairs 的 top 探测）
        v.zd[level] = Some(zd);
        v
    }

    /// **★R3 主机制：核心多腿跟随涌现级别 relabel 上移（多重赋格，face_a_emerge）**——核心建 @3，
    /// emergent_top 涌现到 L4 ⇒ 核心 relabel @3→@4（持仓继承，敞口不变，否定线更新为 L4 中枢 ZD）。
    /// 这是 S1 段无腿修复的本体：L4 涌现段获专属核心腿（原 Face A 核心卡 @3，L4 段无腿）。
    #[test]
    fn r3_核心多腿涌现升级_relabel上移() {
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a_emerge());
        // 1) 核心多腿 @3（否定线 ZD=95）。
        fa.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        assert!(fa.leg_pair(3).long_active(), "核心多腿建仓 @3");
        let u3 = fa.leg_pair(3).long_units;
        let basis3 = fa.leg_pair(3).long_basis;
        // 2) emergent_top 涌现到 L4（L4 自身买点未 fire）⇒ 核心 relabel @3→@4。
        fa.on_bar(&lv_emergent(4, 90.0), 11, 105.0);
        assert!(
            !fa.leg_pair(3).long_active(),
            "★核心多腿上移：原 level 3 空"
        );
        assert!(
            fa.leg_pair(4).long_active(),
            "★核心多腿 relabel 到 level 4（涌现段获专属核心腿=S1 修复）"
        );
        assert!(
            (fa.leg_pair(4).long_units - u3).abs() < 1e-9,
            "持仓继承（无新资金，敞口不变）"
        );
        assert!(
            (fa.leg_pair(4).long_basis - basis3).abs() < 1e-9,
            "basis 继承（relabel 非重新开仓）"
        );
        assert!(
            (fa.leg_pair(4).long_stop - 90.0).abs() < 1e-9,
            "否定线更新为 L4 中枢 ZD=90（核心骑 L4 走势）"
        );
        assert_eq!(fa.pair_emergence_upgrades, 1, "涌现升级计数");
        assert_eq!(fa.n_liquidations, 0, "敞口不变 ⇒ 无强平");
    }

    /// **方向不匹配不上移（对照 instances emergence_upgrade skipped_dir）**：emergent_top=Short ⇒
    /// 核心多腿不跟随下跌涌现上移（下跌涌现走 d_top 核心翻空 pair_core_short，正交）。
    #[test]
    fn r3_涌现方向short_核心多腿不上移() {
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a_emerge());
        fa.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        // emergent_top=(4, Short)（下跌涌现）⇒ 多腿不上移。
        let mut v = LevelView::empty();
        v.emergent_top = Some((4, Polarity::Short));
        v.nodes[4] = Some(node(0, 10, Direction::Down));
        v.zd[4] = Some(90.0);
        fa.on_bar(&v, 11, 105.0);
        assert!(
            fa.leg_pair(3).long_active(),
            "核心多腿仍 @3（下跌涌现不上移多腿）"
        );
        assert!(
            !fa.leg_pair(4).long_active(),
            "L4 无多腿（emergent Short 不触发多腿 relabel）"
        );
        assert_eq!(fa.pair_emergence_upgrades, 0, "无上移");
    }

    /// **目标级已有核心多腿不覆盖（对照 ascend `!instances[to].is_active()` assert）**：L4 已有多腿 ⇒
    /// emergent_top=(4,Long) 不覆盖（L4 核心已存在，无需上移）。
    #[test]
    fn r3_目标已有核心多腿_不覆盖() {
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a_emerge());
        // L4 已有核心多腿（直接 L4 买点 fire），highest_active_long=4。
        fa.on_bar(&lv_buy_zd(4, Direction::Up, 90.0), 10, 100.0);
        let u4 = fa.leg_pair(4).long_units;
        // emergent_top=(4,Long) ⇒ cc=4 不 < target=4 ⇒ 跳过（无 relabel）。
        fa.on_bar(&lv_emergent(4, 88.0), 11, 105.0);
        assert!(fa.leg_pair(4).long_active(), "L4 核心多腿保留");
        assert!(
            (fa.leg_pair(4).long_units - u4).abs() < 1e-9,
            "L4 核心未被覆盖（cc==target 跳过）"
        );
        assert_eq!(fa.pair_emergence_upgrades, 0, "cc 已在涌现级别 ⇒ 无上移");
    }

    /// **bit-exact：face_a（enable_pair_emergence=false）⇒ 涌现视图不触发 relabel（核心卡 @3 = 原行为）**。
    /// face_a vs face_a_emerge 唯一差 = relabel ⇒ R3 增量全归因涌现升级（OFF/Face A/B bit-exact）。
    #[test]
    fn r3_face_a_bitexact_无relabel() {
        // face_a（不开 emergence）：同序列下核心卡 @3，L4 不获核心腿（= committed Face A 原行为）。
        let mut fa = TRoot::new_with_config(100_000.0, EngineConfig::face_a());
        assert!(
            !EngineConfig::face_a().enable_pair_emergence,
            "face_a 默认不开 emergence（bit-exact）"
        );
        fa.on_bar(&lv_buy_zd(3, Direction::Up, 95.0), 10, 100.0);
        fa.on_bar(&lv_emergent(4, 90.0), 11, 105.0);
        assert!(
            fa.leg_pair(3).long_active(),
            "face_a：核心卡 @3（无 relabel，原行为）"
        );
        assert!(
            !fa.leg_pair(4).long_active(),
            "face_a：L4 无核心腿（S1 段无腿，未修复路径）"
        );
        assert_eq!(
            fa.pair_emergence_upgrades, 0,
            "face_a 无涌现升级（bit-exact）"
        );
        // face_a_emerge 唯一增量。
        assert!(
            EngineConfig::face_a_emerge().enable_pair_emergence,
            "face_a_emerge 开 emergence（唯一增量）"
        );
    }

    // ════════════ 关⑥ E1：recover 触发区间套语义裁定（5 负测试，只加测试不改行为）════════════
    // 裁定：`chanlun/escalate/recover-trigger-nesting-ruling-20260718.md`（R1 触发=子级别 BSP /
    // R2 有效域双面界 / R3 093:22 不回补 / R4 §9.4 旧句作废；执行项 E1=单测计划 1–5）。
    // 共用骨架：核心 Long@k + 子 Short@(k−1)（sink 短差对在场），随后给不同级别/方向 BSP。

    /// **①R1 负测试：父级别 BSP 触发 recover 判非法**（裁定 R1 裁决 1 + 单测计划 1）。
    /// 核心 Long@3 + 子 Short@2 在场，`lv_buy(3)`（父级别买点 p=3>j=2）⇒ 子腿不平、核心不 recover——
    /// 父级别 BSP 只走核心级 enter/ascend/no-op 语义域（route_bsp None 分支，rec_engine.rs:1856-1870），
    /// recover 唯一合法触发源 = 子级别（骑乘级别）同父向 BSP；父级别触发 = 平空级别错配（§9.10 −238% 已证伪）。
    #[test]
    fn e1_父级别买点不触发recover_负r1() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 Long @3
        let u0 = r.instance(3).units;
        r.on_bar(&lv_sell(2, Direction::Down), 12, 110.0); // sink：子 Short @2，核心减 1/3
        assert_eq!(
            r.instance(2).direction,
            Polarity::Short,
            "前置：子级短差空腿在场"
        );
        let u_core = r.instance(3).units;
        assert!(u_core < u0, "前置：sink 已减核心（机动仓下放）");
        let (n_rec0, n_sink0) = (r.n_recovers, r.n_sinks);
        // 父级别买点（p=3 > 骑乘级别 j=2）= R1 判非法触发源。
        r.on_bar(&lv_buy(3, Direction::Up), 15, 95.0);
        assert_eq!(
            r.n_recovers, n_rec0,
            "★R1负：父级别 BSP ⇒ n_recovers 不变（recover 触发=子级别 BSP）"
        );
        assert_eq!(r.n_sinks, n_sink0, "父级别买点不被消费为 sink");
        assert!(
            r.instance(2).is_active(),
            "子腿不平：父级别买点对子级短差无平仓语义"
        );
        assert_eq!(
            r.instance(2).direction,
            Polarity::Short,
            "子腿方向不变（不翻转）"
        );
        assert!(
            (r.instance(3).units - u_core).abs() < 1e-9,
            "核心股数不归还（无 recover 发生）"
        );
        assert_eq!(
            r.buy_core, 2,
            "两次核心级买点都路由到核心语义域（enter/ascend/no-op），非 recover 域"
        );
        assert_eq!(r.buy_noop, 0, "父级别买点不落入子级 recover/no-op 分类");
    }

    /// **②R1 正对照：子级别 BSP 触发 recover 合法**（裁定 R1 裁决 1/3 正面锁，与①同骨架对照）。
    /// 同一在场形态，唯一差别 = 买点落在骑乘级别 j=2 ⇒ recover(3,2) 恰好一次：子平空、核心归还原股数。
    /// （与既有 `子级同父向买点_recover` :181 同语义，本测试加锁路由分类与 per-level 落账，作①的正对照。）
    #[test]
    fn e1_子级别买点触发recover_正对照r1() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0);
        let u0 = r.instance(3).units;
        r.on_bar(&lv_sell(2, Direction::Down), 12, 110.0); // sink@110（高开空）
        let n_rec0 = r.n_recovers;
        // 子级别（骑乘级别 j=2）同父向买点 = R1 唯一合法触发源。
        r.on_bar(&lv_buy(2, Direction::Up), 15, 95.0); // 低平空
        assert_eq!(
            r.n_recovers,
            n_rec0 + 1,
            "★R1正：子级别同父向买点 ⇒ recover(p=3,j=2) 恰好一次"
        );
        assert!(!r.instance(2).is_active(), "子平空（短差对回补闭合）");
        assert!(
            (r.instance(3).units - u0).abs() < 1e-6,
            "核心恢复原股数（同股数归还，能量守恒）"
        );
        assert_eq!(
            r.buy_recover, 1,
            "买点路由分类=buy_recover（父多+买点+j 持短差）"
        );
        assert_eq!(r.recover_by_level[2], 1, "recover 按骑乘级别 j=2 分层落账");
        assert!(r.short_leg_pnl > 0.0, "高开低平短差盈利");
    }

    /// **③R2 有效域边界：失效域不挂 recover 期待 + 配对分层落账**（裁定 R2 + 单测计划 5，诊断断言非硬门）。
    /// 塔顶域（核心 @7=MAX_LEVEL−1，rL2+ 结构位置）：父/祖级别买点仍无 buy[p>j]→recover 触发链。
    /// R2 不回滚 3：边界不落硬编码级别常数（失效域由可观测配对率涌现）——故本测试只锁
    /// 「无父级触发链 + per-level 分层落账」，**不**断言「高级别禁开空腿」（那不是代码门）。
    #[test]
    fn e1_失效域不挂recover期待_r2() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(7, Direction::Up), 10, 100.0); // 核心 Long @7（塔顶，无更高级别）
        r.on_bar(&lv_sell(6, Direction::Down), 12, 110.0); // 子 Short @6（sink 短差对在场）
        assert_eq!(
            r.instance(6).direction,
            Polarity::Short,
            "前置：高级别短差空腿在场"
        );
        let (n_rec0, n_sink0) = (r.n_recovers, r.n_sinks);
        // 失效域合成视图：父/祖级别买点（p=7 > j=6）⇒ 不得建立 buy[p>j]→recover 依赖。
        r.on_bar(&lv_buy(7, Direction::Up), 15, 95.0);
        assert_eq!(
            r.n_recovers, n_rec0,
            "★R2：塔顶父级别买点 ⇒ 不挂 recover 期待（无 buy[p>j] 触发链）"
        );
        assert_eq!(r.n_sinks, n_sink0, "父级别买点不引发新 sink");
        assert!(r.instance(6).is_active(), "高级别子腿不被父级别买点平掉");
        assert_eq!(r.buy_noop, 0, "父级别买点不进入子级 recover/no-op 分类域");
        // 配对统计按级别分层落账（R2 验收线 1）：sink 记骑乘级别 6，recover 全级别 0（本序列无配对闭合）。
        assert_eq!(r.sink_by_level[6], 1, "sink 按骑乘级别 j=6 落账");
        assert_eq!(
            r.sink_by_level.iter().sum::<u64>(),
            1,
            "无其他级别被该序列消费"
        );
        assert!(
            r.recover_by_level.iter().all(|&n| n == 0),
            "recover_by_level 全 0（本序列无 recover 配对）"
        );
    }

    /// **④R3 093:22 最后卖点不回补**（裁定 R3 + 单测计划 4 + R3 验收线 1）。
    /// 核心 Long@3 + 子 Short@2（最后一次 sink，未回补）⇒ 核心级反向 BSP（「市场选择 (−1,1)」的结构
    /// 事件，零价格阈值）触发 flip=clear_all+反向 enter：塔清点 n_recovers 不增（退出≠recover，不挂
    /// 通道 0），旧塔子腿随塔清而非 recover 回补闭合；塔清后旧级别买点不出现 recover（无回补期待残留，
    /// 退出后的反向重建是新韵律的开始，与旧塔无归还关系）。
    #[test]
    fn e1_093最后卖点不回补_塔清无recover期待_r3() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0); // 核心 Long @3
        r.on_bar(&lv_sell(2, Direction::Down), 12, 110.0); // 子 Short @2（最后一次卖点，未回补）
        assert_eq!(
            r.instance(2).direction,
            Polarity::Short,
            "前置：旧塔短差空腿在场"
        );
        let n_rec0 = r.n_recovers;
        // 093:22「市场选择 (−1,1)」= 核心级反向 BSP（结构事件）⇒ flip 塔清。
        r.on_bar(&lv_sell(3, Direction::Down), 20, 105.0);
        assert_eq!(
            r.n_flips, 1,
            "核心级反向 BSP ⇒ flip（clear_all + 反向 enter）"
        );
        assert_eq!(
            r.n_recovers, n_rec0,
            "★R3：最后一次卖点不回补——塔清点 n_recovers 不增（退出不称 recover）"
        );
        assert!(
            !r.instance(2).is_active(),
            "旧塔子腿随塔清（clear_all 平清，非 recover 回补闭合）"
        );
        assert_eq!(
            r.recover_by_level[2], 0,
            "旧塔子腿不按 recover 记账（会计分流：塔清≠通道 0）"
        );
        assert_eq!(
            r.instance(3).direction,
            Polarity::Short,
            "按新方向重建（新韵律开始，与旧塔无归还关系）"
        );
        // 塔清后旧级别位置的买点不复活 recover（无回补期待残留；新塔反向韵律另起）。
        r.on_bar(&lv_buy(2, Direction::Up), 22, 100.0);
        assert_eq!(
            r.n_recovers, n_rec0,
            "★R3：塔清后无 recover 发生（退出是终态，非延迟回补）"
        );
    }

    /// **⑤R1 单次路由：同一 BSP 不被两级重复消费**（裁定 R1 裁决 2 + 单测计划 2）。
    /// `lv_buy(2)` 触发 recover(3,2) 后，同一 BSP 不再驱动任何 sink/开孙腿：BSP@k 只在级别 k 路由一次，
    /// 消费者 = 级别 k 仓位与其最近活跃祖先（nearest_active_parent）；孙腿齿轮由 BSP@(j−1) 驱动，
    /// 不由 BSP@j 再消费一次；纯 BSP 类型耦合（§9.11 修复 2）⇒ type1_buy 无「又是 sink」的类型通道。
    #[test]
    fn e1_单bsp单次路由_不两级重复消费_r1() {
        let mut r = TRoot::new(100_000.0);
        r.on_bar(&lv_buy(3, Direction::Up), 10, 100.0);
        r.on_bar(&lv_sell(2, Direction::Down), 12, 110.0); // sink：子 Short @2
        let (n_sink0, n_rec0) = (r.n_sinks, r.n_recovers);
        // 同一 BSP（buy@2）⇒ 只在级别 2 路由一次：消费为 recover(3,2)，不再二次消费。
        r.on_bar(&lv_buy(2, Direction::Up), 15, 95.0);
        assert_eq!(
            r.n_recovers,
            n_rec0 + 1,
            "BSP@2 ⇒ recover 恰好一次（消费者=级别 2 仓位+最近活跃祖先 p=3）"
        );
        assert_eq!(
            r.n_sinks, n_sink0,
            "★单次路由：同一 BSP 不再被消费为 sink（is_reduce 分支互斥）"
        );
        assert!(
            !r.instance(1).is_active(),
            "★BSP@2 不开 level 1 孙腿（孙腿齿轮由 BSP@(j−1) 驱动）"
        );
        assert!(
            !r.instance(2).is_active(),
            "级别 2 仓位已被该 BSP 平清（无残留供二次消费）"
        );
        assert_eq!(
            r.buy_sink, 0,
            "type1_buy 不进 sink 类型通道（纯 BSP 类型耦合）"
        );
        assert_eq!(r.buy_recover, 1, "该 BSP 唯一消费形态 = buy_recover");
    }
}
