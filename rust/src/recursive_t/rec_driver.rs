//! **递归 T 走势树消费驱动**（on_bar）：把走势结构映射到 `TRoot` 的 TInstance 链。
//!
//! 设计：docs/recursive_t_architecture_v2.md §3（spawn/flip 三分支）+ §2（回调 sink/recover）。
//! 解耦策略（编排者：先合成视图测 driver，再接 iterate 真树）：driver 消费**操作链视图**
//! `ChainView`（一个中间表示），不直接依赖 `iterate` 的 `RecursiveTree`。`extract_chain` 适配器
//! 把真树投影为 `ChainView`（下一步实装）。
//!
//! ## 操作链（§2.1 自相似：core 与短差是同一 T 不同深度）
//! 任一时刻活跃结构 = 从最高走势向下穿过**嵌套当前回调**的一条**链**（每个 T 至多一个活跃回调子 T）：
//! `chain[0]` = 最高涌现走势（root core）；`chain[i+1]` = `chain[i]` 走势内**当前回调子走势**
//! （反父向，子 T 持空）。几何塔（1/3、1/9…）= 这条链的递归深度。
//!
//! ## reconcile（每次重跑：现有 TInstance 链 → 期望 ChainView）
//! 1. **top 对齐**：root 空 → `enter`；top 节点变（按 start_bar 身份）且同向 → `spawn`（relabel 升格）；
//!    反向 → `flip`（全树塌缩）。
//! 2. **回调逐层 reconcile**：自顶向下，每层比对「现有回调子 T」vs「视图期望回调」——
//!    完成/消失 → `recover`（子 T 平空升回）；新回调 → `sink`（建子 T 持空）。匹配靠 `start_bar`
//!    节点身份（A1，跨重跑稳定；§8.6 前缀冻结假设）。
//!
//! ## 认识论等级
//! reconcile 逻辑 = **L0**（从 v2 设计推导）；合成视图测试验证 driver 逻辑正确；`extract_chain`
//! 真树投影 + 回测收益 = 待实装 / L3。

use super::rec_engine::{dir_to_polarity, TRoot, TrendNode};
use super::types::{BSPKind, Direction, RecursiveTree, TrendKind, TrendType, Unit};
use crate::trading::types::Polarity;

/// BSP 触发判定（P3b 消费，C1 北极星：引擎消费买卖点而非走势结构）：视图 `bsps` 中该 `level`
/// 是否存在 type1 买点（`want_buy`）/ 卖点。多核心 sink 由本级别 type1_sell 触发，子 T 平空 recover
/// 由子级别 type1_buy 触发（ε 对称：空核心 sink 用买点、recover 用卖点）。
fn bsp_fires(view: &ChainView, level: usize, want_buy: bool) -> bool {
    let kind = if want_buy { BSPKind::Type1Buy } else { BSPKind::Type1Sell };
    view.bsps.iter().any(|b| b.level == level && b.kind == kind)
}

/// 操作链中的一个走势节点（视图）。
#[derive(Debug, Clone, Copy)]
pub struct ChainNode {
    /// 走势节点身份（start_bar 重锚键）。
    pub node: TrendNode,
    /// 该走势是否已完成（背驰确认 / 被反向走势终结）。完成的回调 → recover。
    pub completed: bool,
}

/// 操作层消费的买卖点（BSP 消费重构，docs/bsp_consumption_redesign.md）：携 kind+level+price+bar。
/// driver `route_bsp` 按 (kind, level vs core_level, 方向) 分层路由（本级别翻转/次级别齿轮短差）。
#[derive(Debug, Clone, Copy)]
pub struct ChainBsp {
    pub kind: BSPKind,
    pub level: usize,
    pub price: f64,
    pub bar: i64,
}

/// 操作链视图：`nodes[0]` = 最高走势（core，级别上下文 + sink 载体）；`bsps` = 操作触发器（分层消费）。
#[derive(Debug, Clone, Default)]
pub struct ChainView {
    pub nodes: Vec<ChainNode>,
    /// BSP 消费重构：操作层消费对象（全 6 类，携级别）。core_level 之上不存在。
    pub bsps: Vec<ChainBsp>,
    /// 核心级别 = 最高涌现级别（levels.len()-1）；route_bsp 判 level vs core_level 分层。
    pub core_level: usize,
}

impl ChainView {
    pub fn new(nodes: Vec<ChainNode>) -> Self {
        ChainView { nodes, bsps: Vec::new(), core_level: 0 }
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// 递归 T 驱动：包 `TRoot`，每次重跑消费 `ChainView` 的 BSP——**所有级别并行**（多重赋格，编排者 2026-06-21）。
pub struct RecDriver {
    root: TRoot,
    // ── 多重赋格诊断（每级别独立消费 BSP）──
    /// sink 触发级别分布（type1_sell@k 触发核心减仓 + 级别-k 开空）。
    pub sink_lvl: [u64; 10],
    /// recover 触发级别分布（type1_buy@k 触发级别-k 平空 + 核心升回）。
    pub recover_lvl: [u64; 10],
}

impl RecDriver {
    pub fn new(initial_capital: f64) -> Self {
        RecDriver { root: TRoot::new(initial_capital), sink_lvl: [0; 10], recover_lvl: [0; 10] }
    }

    pub fn root(&self) -> &TRoot {
        &self.root
    }
    pub fn root_mut(&mut self) -> &mut TRoot {
        &mut self.root
    }

    /// 消费一次走势视图（重跑触发）：reconcile TInstance 链到期望视图。
    pub fn on_view(&mut self, view: &ChainView, c: f64, bar: i64) {
        if view.is_empty() || c <= 0.0 {
            return;
        }
        // ── 1. top 走势对齐（§3.5）──
        match self.root.root_slot() {
            None => {
                // 编排者裁决 C1：核心持多骑牛（永不翻空）。仅在最高走势上行（Up）时建**多**核心；
                // 下行时不建（等上行/买点）——避免按 chain[0] 方向入场卡空（P3a bug：首入 down→100% 持空）。
                if view.nodes[0].node.direction == Direction::Up {
                    self.root.enter(view.nodes[0].node, c, bar);
                }
            }
            Some(root) => {
                let rnode = self.root.instance(root).node;
                let rdir = self.root.instance(root).direction;
                let d0 = view.nodes[0];
                let d0_pol = dir_to_polarity(d0.node.direction);
                // 编排者裁决 C1（2026-06-20）：**核心永不整仓翻空**（删 flip/promote/Z₂）。
                // 仅**同向**更高走势涌现 → spawn 骑乘上移（核心持多骑趋势）。
                // 反向 chain[0]（最高走势下行）= 大回调，核心**不翻转**——由各级别 sink 短差对冲，
                // 全量清仓只在最高级别超大卖点触发（第31课，本版暂不实装超大卖点 → 核心骑到底）。
                if d0_pol == rdir && d0.node.start_bar != rnode.start_bar {
                    self.root.spawn(d0.node, c, bar);
                }
            }
        }
        // ── 2. 所有级别并行消费 BSP：每级别独立 sink/recover（C1 北极星 + 编排者 2026-06-21 多重赋格）──
        self.reconcile_all_levels(view, c, bar);
    }

    /// **所有级别并行消费 BSP**（多重赋格 = 多声部，编排者 2026-06-21；删单链下钻 + 走势节点 gate）。
    ///
    /// 遍历每个**绝对级别** k（0..=core_level），每级别独立检查自己的 type1 BSP——不依赖上级 sink 是否成功
    /// （区间套解决时序：低级别卖点先 fire 先行动，不等最高级别确认）。低频高级别 + 高频低级别齿轮并行：
    /// - **sink**（级别 k 无短差腿 + type1 卖点 fire）：多核心→type1_sell@k / 空核心→type1_buy@k（ε 对称）。
    ///   核心减 quota + 建级别-k 短差（载体=卖点自身，非走势节点）。
    /// - **recover**（级别 k 有短差腿 + type1 平空点 fire）：多核心→type1_buy@k / 空核心→type1_sell@k。
    ///   级别-k 平空 + 核心升回。
    fn reconcile_all_levels(&mut self, view: &ChainView, c: f64, bar: i64) {
        let Some(core) = self.root.root_slot() else { return };
        let core_dir = self.root.instance(core).direction; // 核心方向（持多骑牛）
        for k in 0..=view.core_level.min(9) {
            if self.root.has_level_short(k) {
                // recover：平空点 fire（多核心→type1_buy@k / 空核心→type1_sell@k）。
                let recover_buy = core_dir == Polarity::Long;
                if bsp_fires(view, k, recover_buy) && self.root.recover(k, c, bar) {
                    self.recover_lvl[k] += 1;
                }
            } else {
                // sink：卖点 fire（多核心→type1_sell@k / 空核心→type1_buy@k）。
                let sink_buy = core_dir == Polarity::Short;
                if bsp_fires(view, k, sink_buy) && self.root.sink(k, c, bar).is_some() {
                    self.sink_lvl[k] += 1;
                }
            }
        }
    }

    /// 收尾（全平 + final_nav）。
    pub fn finish(&mut self, c: f64) -> f64 {
        self.root.finish(c)
    }
}

// ════════════════════════════ iterate RecursiveTree → ChainView 适配器（下一步真树接入）════════════════════════════

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

/// 在某级别 trends 中按 start_bar 找对应走势（encapsulate 跨级同坐标，operator.rs:28）。
fn find_trend_by_start(trends: &[TrendType], start_bar: i64) -> Option<&TrendType> {
    trends
        .iter()
        .find(|t| t.units.first().map(|u| u.start_bar) == Some(start_bar))
}

/// 从 iterate 的 `RecursiveTree` 投影出操作链视图（适配器，§3.2 自上而下）。
///
/// 算法：`chain[0]` = 最高级别当前走势（`levels.last().trends.last()`，core 骑乘）；逐层下钻——
/// 取当前走势的**最后一段** `units.last()`：若反父向 = **当前回调**（子 T 持空的 node），入链并按
/// `start_bar` 映射到下级 `TrendType` 继续下钻；若顺父向 = 顺势腿（属核心，无活跃回调）则停。回调
/// 完成判定 = 映射到的下级走势 `completed`（完成 → driver recover）。
///
/// 节点身份 = `start_bar`（A1，跨重跑稳定，依赖 encapsulate 同坐标 + 前缀冻结，§8.6 N4 验收对象）。
/// **L0 投影逻辑**；真实数据下节点身份稳定性 + 回测收益 = L2/L3 待验证。
pub fn extract_chain(tree: &RecursiveTree) -> ChainView {
    let levels = &tree.levels;
    if levels.is_empty() {
        return ChainView::default();
    }
    let top_k = levels.len() - 1;
    let Some(top_trend) = levels[top_k].trends.last() else {
        return ChainView::default();
    };
    let mut nodes = vec![ChainNode { node: trend_to_node(top_trend), completed: top_trend.completed }];

    // 逐层下钻当前回调。
    let mut cur_trend: &TrendType = top_trend;
    let mut cur_level = top_k;
    loop {
        let Some(last_leg) = cur_trend.units.last() else { break };
        // 最后一段顺父向 = 顺势腿（核心），无活跃回调 → 停。
        if last_leg.direction == cur_trend.direction {
            break;
        }
        // 反父向 = 当前回调。映射到下级 TrendType 取完成态 + 继续下钻。
        let sub = if cur_level >= 1 {
            find_trend_by_start(&levels[cur_level - 1].trends, last_leg.start_bar)
        } else {
            None // a0 笔层：回调是一根笔，无下级 TrendType
        };
        nodes.push(ChainNode {
            node: unit_to_node(last_leg),
            completed: sub.map(|t| t.completed).unwrap_or(false),
        });
        match sub {
            Some(t) => {
                cur_trend = t;
                cur_level -= 1;
            }
            None => break,
        }
    }
    // BSP 消费重构 P1：投影全级别 BSP（操作触发器）+ core_level。走势节点保留作级别上下文 + sink 载体。
    let mut bsps = Vec::new();
    for lvl in levels.iter() {
        for b in &lvl.bsps {
            bsps.push(ChainBsp { kind: b.kind, level: b.level, price: b.price, bar: b.bar });
        }
    }
    ChainView { nodes, bsps, core_level: top_k }
}

/// 走势方向 → ChainNode 构造辅助（适配器/测试用）。
pub fn chain_node(start_bar: i64, end_bar: i64, dir: Direction, completed: bool) -> ChainNode {
    ChainNode {
        node: TrendNode::new(start_bar, end_bar, 90.0, 110.0, dir),
        completed,
    }
}

/// TrendKind → 是否趋势（适配器用：盘整 vs 趋势的回调识别辅助，下一步）。
pub fn is_trend(kind: TrendKind) -> bool {
    matches!(kind, TrendKind::UpTrend | TrendKind::DownTrend)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(s: i64, e: i64, done: bool) -> ChainNode {
        chain_node(s, e, Direction::Up, done)
    }
    fn down(s: i64, e: i64, done: bool) -> ChainNode {
        chain_node(s, e, Direction::Down, done)
    }
    fn view(nodes: &[ChainNode]) -> ChainView {
        ChainView::new(nodes.to_vec())
    }
    /// P3b：构造一个 type1 BSP 触发器（price/bar 不影响触发判定）。
    fn cb(kind: BSPKind, level: usize) -> ChainBsp {
        ChainBsp { kind, level, price: 0.0, bar: 0 }
    }
    /// P3b：带 BSP 触发器 + core_level 的视图（sink/recover 现由 BSP 触发，非走势结构）。
    fn view_b(nodes: &[ChainNode], bsps: Vec<ChainBsp>, core_level: usize) -> ChainView {
        ChainView { nodes: nodes.to_vec(), bsps, core_level }
    }
    use crate::trading::types::Polarity;

    // ──────────────── 首仓 ────────────────

    #[test]
    fn 首个走势视图_enter根核心() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let root = d.root().root_slot().expect("已建 root");
        assert_eq!(d.root().instance(root).direction, Polarity::Long, "Up 走势 → Long 核心");
        assert!((d.root().total_wealth(100.0) - 100_000.0).abs() < 1e-4, "建仓 TW 中性");
    }

    #[test]
    fn 空视图_不动() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[]), 100.0, 0);
        assert!(d.root().root_slot().is_none(), "空视图不建仓");
    }

    // ──────────────── 回调 sink / recover ────────────────

    #[test]
    fn type1_sell触发sink_级别短差持空() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let root = d.root().root_slot().unwrap();
        let u0 = d.root().instance(root).units;
        // type1_sell@级别1 → 级别1 sink（核心减 + 级别1 开空）。不需走势回调节点（删 want gate）。
        d.on_view(&view_b(&[up(0, 12, false)], vec![cb(BSPKind::Type1Sell, 1)], 2), 100.0, 12);
        assert_eq!(d.root().n_active(), 2, "核心 + 级别1 短差腿");
        assert!((d.root().instance(root).units - u0 * 2.0 / 3.0).abs() < 1e-6, "核心减到 2/3");
        let (_, su) = d.root().exposure();
        assert!((su - u0 / 3.0).abs() < 1e-6, "级别1 持空 1/3（同股数）");
    }

    #[test]
    fn type1_buy触发recover_级别短差升回() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let root = d.root().root_slot().unwrap();
        let u0 = d.root().instance(root).units;
        d.on_view(&view_b(&[up(0, 12, false)], vec![cb(BSPKind::Type1Sell, 1)], 2), 110.0, 12); // sink@110
        assert_eq!(d.root().n_active(), 2);
        // type1_buy@级别1（同级别平空点）@90 → 级别1 平空升回核心。
        d.on_view(&view_b(&[up(0, 15, false)], vec![cb(BSPKind::Type1Buy, 1)], 2), 90.0, 15);
        assert_eq!(d.root().n_active(), 1, "平空点 fire，级别1 平空升回");
        assert!((d.root().instance(root).units - u0).abs() < 1e-6, "核心恒仓恢复原股数");
        assert!(d.root().short_leg_pnl > 0.0, "高开低平短差降成本");
    }

    #[test]
    fn 无平空点_级别短差持有不recover() {
        // 纯 BSP 驱动（删 node_gone 兜底）：无 type1_buy 平空点 → 级别短差持有。
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        d.on_view(&view_b(&[up(0, 12, false)], vec![cb(BSPKind::Type1Sell, 1)], 2), 110.0, 12);
        assert_eq!(d.root().n_active(), 2);
        d.on_view(&view(&[up(0, 15, false)]), 95.0, 15); // 无平空点
        assert_eq!(d.root().n_active(), 2, "无平空点，级别短差持有（纯 BSP 驱动）");
    }

    // ──────────────── spawn / flip（top 变化）────────────────

    #[test]
    fn top更高同向走势_spawn升格() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(5, 10, false)]), 100.0, 10);
        let r0 = d.root().root_slot().unwrap();
        let u0 = d.root().instance(r0).units;
        // 更高同向走势涌现（start_bar 0 < 5，包含 root）→ spawn relabel。
        d.on_view(&view(&[up(0, 20, false)]), 100.0, 20);
        let r1 = d.root().root_slot().unwrap();
        assert_ne!(r1, r0, "relabel 到新槽");
        assert!((d.root().instance(r1).units - u0).abs() < 1e-9, "持仓继承（relabel）");
        assert_eq!(d.root().n_spawns, 1);
    }

    #[test]
    fn top反向走势_核心不翻转持多骑趋势() {
        // 编排者裁决 C1（2026-06-20）：核心永不整仓翻空（删 flip/promote）。最高走势反向（下行）=大回调，
        // 核心持多不动（由 sink 短差对冲，全量清仓只在超大卖点）。修 82% 持空根因。
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let r0 = d.root().root_slot().unwrap();
        assert_eq!(d.root().instance(r0).direction, Polarity::Long, "核心持多");
        // 最高走势反向（Down）→ 核心**不翻转**（C1：无 flip）。
        d.on_view(&view(&[down(10, 20, false)]), 100.0, 20);
        assert_eq!(d.root().root_slot(), Some(r0), "核心槽不变（不翻空）");
        assert_eq!(d.root().instance(r0).direction, Polarity::Long, "核心持多骑趋势，不翻空");
        assert!((d.root().total_wealth(100.0) - 100_000.0).abs() < 1e-4, "TW 中性");
    }

    // ──────────────── 多重赋格（所有级别并行消费 BSP）────────────────

    #[test]
    fn 多级别并行短差_多声部() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        // type1_sell@级别1 + 级别2 同时 fire → 两级别并行各自 sink（多声部，非单链下钻）。
        d.on_view(
            &view_b(
                &[up(0, 14, false)],
                vec![cb(BSPKind::Type1Sell, 1), cb(BSPKind::Type1Sell, 2)],
                2,
            ),
            100.0,
            14,
        );
        assert_eq!(d.root().n_active(), 3, "核心 + 级别1 + 级别2 短差腿（并行多声部）");
        let (lu, su) = d.root().exposure();
        assert!(lu > 0.0 && su > 0.0, "多（核心）与空（两级别短差）并存");
        assert_eq!(d.sink_lvl[1], 1, "级别1 sink 计数");
        assert_eq!(d.sink_lvl[2], 1, "级别2 sink 计数");
        assert!((d.root().total_wealth(100.0) - 100_000.0).abs() < 1e-4, "多声部 TW 中性");
    }

    // ──────────────── 守恒（全程）────────────────

    // ──────────────── extract_chain：iterate 真树投影 ────────────────

    use crate::recursive_t::iterate;
    use crate::recursive_t::types::{
        PerfectionMode, RecursiveTree, TLevelOutput, TrendKind, TrendType, Unit,
    };

    fn u(lo: f64, hi: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(lo, hi, s, e, d)
    }
    fn mk_trend(dir: Direction, units: Vec<Unit>, completed: bool) -> TrendType {
        TrendType {
            kind: if dir == Direction::Up { TrendKind::UpTrend } else { TrendKind::DownTrend },
            zhongshus: vec![],
            units,
            level: 0,
            direction: dir,
            completed,
            bsp: None,
        }
    }

    #[test]
    fn extract_chain_iterate真树_上涨趋势末段顺势无回调() {
        // 上涨趋势八笔（与 mod.rs 测试同构）：末段 笔8 Up = 顺势腿 → 无活跃回调 → chain len 1。
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
        let v = extract_chain(&tree);
        assert!(!v.is_empty(), "iterate 真树投影非空");
        assert_eq!(v.nodes[0].node.direction, Direction::Up, "顶层 Up 走势");
        assert_eq!(v.nodes.len(), 1, "末段顺势腿，无活跃回调");
        // BSP 消费重构 P1：core_level=最高级别；bsps 投影自全级别 levels[k].bsps（操作触发器）。
        assert_eq!(v.core_level, tree.levels.len() - 1, "core_level=最高涌现级别");
        let want_bsps: usize = tree.levels.iter().map(|l| l.bsps.len()).sum();
        assert_eq!(v.bsps.len(), want_bsps, "投影全级别 BSP（P1：暴露给操作层，未消费）");
    }

    #[test]
    fn extract_chain_回调下钻两层() {
        // 构造 2 级树：level1 Up 走势末段为 Down 回调 → 映射到 level0 Down 走势(completed)。
        let l0 = TLevelOutput {
            level: 0,
            centers: vec![],
            trends: vec![mk_trend(
                Direction::Down,
                vec![u(8.0, 18.0, 5, 7, Direction::Down), u(9.0, 15.0, 8, 9, Direction::Down)],
                true,
            )],
            bsps: vec![],
            next_units: vec![],
        };
        let l1 = TLevelOutput {
            level: 1,
            centers: vec![],
            trends: vec![mk_trend(
                Direction::Up,
                vec![u(10.0, 20.0, 0, 4, Direction::Up), u(8.0, 18.0, 5, 9, Direction::Down)],
                false,
            )],
            bsps: vec![],
            next_units: vec![],
        };
        let tree = RecursiveTree { levels: vec![l0, l1] };
        let v = extract_chain(&tree);
        assert_eq!(v.nodes.len(), 2, "顶走势 + 一层回调");
        assert_eq!(v.nodes[0].node.direction, Direction::Up, "顶层 Up");
        assert_eq!(v.nodes[1].node.direction, Direction::Down, "回调反父向 Down");
        assert_eq!(v.nodes[1].node.start_bar, 5, "回调节点 start_bar=5（跨级同坐标）");
        assert!(v.nodes[1].completed, "回调映射到 level0 completed 走势 → 完成");
    }

    #[test]
    fn extract_chain_空树() {
        let tree = RecursiveTree { levels: vec![] };
        assert!(extract_chain(&tree).is_empty());
    }

    // ──────────────── extract_chain + driver 端到端 ────────────────

    #[test]
    fn 端到端_iterate真树_driver消费不panic() {
        // iterate 真树 → extract_chain → driver.on_view：守恒守卫逐操作 panic 验收，端到端零 panic。
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
        let v = extract_chain(&tree);
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&v, 40.0, 8); // 顶层 Up 走势 → enter 核心
        assert!(d.root().root_slot().is_some(), "真树驱动建仓");
        let fin = d.finish(40.0);
        assert!(fin.is_finite() && fin > 0.0, "端到端 final_nav 有限");
    }

    #[test]
    fn 全程tw中性_sink_recover_spawn() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        // sink（type1_sell@1）→ recover（type1_buy@1 同级别）→ spawn（同向更高）→ 反向走势 C1 核心不翻转。
        d.on_view(&view_b(&[up(0, 12, false)], vec![cb(BSPKind::Type1Sell, 1)], 2), 120.0, 12);
        d.on_view(&view_b(&[up(0, 15, false)], vec![cb(BSPKind::Type1Buy, 1)], 2), 100.0, 15);
        d.on_view(&view(&[up(0, 30, false)]), 130.0, 30); // spawn
        d.on_view(&view(&[down(30, 40, false)]), 110.0, 40); // 反向走势：C1 核心不翻转（删 flip）
        let fin = d.finish(110.0);
        // 全程同价段内 TW 中性守卫已逐操作 panic 验收；末态 final_nav 有限且合理。
        assert!(fin.is_finite() && fin > 0.0, "final_nav 有限，得 {fin}");
    }
}
