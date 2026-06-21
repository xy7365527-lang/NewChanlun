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

use super::rec_engine::{dir_to_polarity, flip_pol, TRoot, TrendNode};
use super::types::{Direction, RecursiveTree, TrendKind, TrendType, Unit};

/// 操作链中的一个走势节点（视图）。
#[derive(Debug, Clone, Copy)]
pub struct ChainNode {
    /// 走势节点身份（start_bar 重锚键）。
    pub node: TrendNode,
    /// 该走势是否已完成（背驰确认 / 被反向走势终结）。完成的回调 → recover。
    pub completed: bool,
}

/// 操作链视图：`nodes[0]` = 最高走势（core）；`nodes[i+1]` = `nodes[i]` 的当前回调子走势。
#[derive(Debug, Clone, Default)]
pub struct ChainView {
    pub nodes: Vec<ChainNode>,
}

impl ChainView {
    pub fn new(nodes: Vec<ChainNode>) -> Self {
        ChainView { nodes }
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// 递归 T 驱动：包 `TRoot`，每次重跑用 `ChainView` reconcile TInstance 链。
pub struct RecDriver {
    root: TRoot,
}

impl RecDriver {
    pub fn new(initial_capital: f64) -> Self {
        RecDriver { root: TRoot::new(initial_capital) }
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
                self.root.enter(view.nodes[0].node, c, bar);
            }
            Some(root) => {
                let rnode = self.root.instance(root).node;
                let rdir = self.root.instance(root).direction;
                let d0 = view.nodes[0];
                if d0.node.start_bar != rnode.start_bar {
                    if dir_to_polarity(d0.node.direction) == rdir {
                        // 更高同向走势涌现（包含 root）→ spawn relabel 升格（核心骑趋势上行）。
                        self.root.spawn(d0.node, c, bar);
                    } else {
                        // 顶级反向（转折，§9）：当前回调子 T 短头已持新方向 ⟹ **promote 升格为新核心**
                        // （敞口连续零真空，§9.3）。无回调子 T 可升（退化边界 §9.5）→ clear_root 退出观望。
                        let new_dir = dir_to_polarity(d0.node.direction);
                        let promotable = self
                            .root
                            .instance(root)
                            .child
                            .and_then(|cr| self.root.resolve_ref(cr))
                            .map(|cs| self.root.instance(cs).direction == new_dir)
                            .unwrap_or(false);
                        if promotable {
                            self.root.promote(d0.node, c, bar);
                            // promote 后 root = 升格的子 T（已骑 new_node）；继续 reconcile 其回调链。
                        } else {
                            self.root.clear_root(c, bar);
                            return; // 退化：本重跑退出观望；下一重跑 root=None → enter 重新入场
                        }
                    }
                }
            }
        }
        // ── 2. 回调链逐层 reconcile（top 已对齐）──
        self.reconcile_chain(view, c, bar);
    }

    /// 自顶向下逐层 reconcile 回调子 T（按 start_bar 身份匹配）。
    fn reconcile_chain(&mut self, view: &ChainView, c: f64, bar: i64) {
        let Some(mut cur) = self.root.root_slot() else { return };
        let mut depth = 0usize;
        loop {
            let cur_dir = self.root.instance(cur).direction;
            let want = view.nodes.get(depth + 1).copied(); // cur 的回调应是这个

            // (a) 现有回调子 T：完成/消失/换节点 → recover。
            if let Some(cref) = self.root.instance(cur).child {
                if let Some(cslot) = self.root.resolve_ref(cref) {
                    let cnode = self.root.instance(cslot).node;
                    let keep = want
                        .map(|w| !w.completed && w.node.start_bar == cnode.start_bar)
                        .unwrap_or(false);
                    if !keep {
                        let pref = self.root.ref_of(cur);
                        self.root.recover(pref, cref, c, bar);
                    }
                }
                // resolve 失败（ABA）：引用失效，视作无 child（孤儿，下一步可能 sink 新回调）。
            }

            // (b) 无回调子 T 且视图有新回调（反父向、未完成）→ sink 建子 T。
            if self.root.instance(cur).child.is_none() {
                if let Some(w) = want {
                    if !w.completed && dir_to_polarity(w.node.direction) == flip_pol(cur_dir) {
                        self.root.sink(cur, w.node, c, bar);
                    }
                }
            }

            // (c) 下降到回调子 T（若有），继续 reconcile 更深嵌套回调。
            match self.root.instance(cur).child {
                Some(cref) => match self.root.resolve_ref(cref) {
                    Some(cslot) => {
                        cur = cslot;
                        depth += 1;
                    }
                    None => break,
                },
                None => break,
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
    ChainView::new(nodes)
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
    fn 出现回调_sink建子T持空() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let root = d.root().root_slot().unwrap();
        let u0 = d.root().instance(root).units;
        // 视图新增一条回调下跌 node（反父向）→ sink。
        d.on_view(&view(&[up(0, 12, false), down(10, 12, false)]), 100.0, 12);
        assert_eq!(d.root().n_active(), 2, "root 核心 + 回调子 T");
        assert!((d.root().instance(root).units - u0 * 2.0 / 3.0).abs() < 1e-6, "父减到 2/3");
        let (_, su) = d.root().exposure();
        assert!((su - u0 / 3.0).abs() < 1e-6, "子 T 持空 1/3（同股数）");
    }

    #[test]
    fn 回调完成_recover子T升回() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        let root = d.root().root_slot().unwrap();
        let u0 = d.root().instance(root).units;
        d.on_view(&view(&[up(0, 12, false), down(10, 12, false)]), 110.0, 12); // sink @110
        assert_eq!(d.root().n_active(), 2);
        // 回调完成（completed=true，底背驰 @90）→ recover。
        d.on_view(&view(&[up(0, 15, false), down(10, 14, true)]), 90.0, 15);
        assert_eq!(d.root().n_active(), 1, "回调完成，子 T 平空升回");
        assert!((d.root().instance(root).units - u0).abs() < 1e-6, "核心恒仓恢复原股数");
        assert!(d.root().short_leg_pnl > 0.0, "高开低平短差降成本");
    }

    #[test]
    fn 回调消失_也触发recover() {
        // 视图里回调 node 不再出现（被吸收）→ 也应 recover（keep=false）。
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        d.on_view(&view(&[up(0, 12, false), down(10, 12, false)]), 110.0, 12);
        assert_eq!(d.root().n_active(), 2);
        d.on_view(&view(&[up(0, 15, false)]), 95.0, 15); // 回调 node 消失
        assert_eq!(d.root().n_active(), 1, "回调消失 → recover");
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
    fn top反向走势_promote回调子T升格_零真空() {
        // §9（编排者裁决 2026-06-20）：顶级反转时回调子 T 短头已持新方向 → promote 升格为新核心
        // （敞口连续零真空），取代 clear 退出观望。
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        d.on_view(&view(&[up(0, 12, false), down(10, 12, false)]), 100.0, 12); // 回调子 T 短(down)
        assert_eq!(d.root().n_active(), 2);
        // top 变为反向（Down）走势 → 回调子 T（short,down）在新方向 → promote 升格。
        d.on_view(&view(&[down(12, 20, false)]), 100.0, 20);
        assert_eq!(d.root().n_active(), 1, "回调子 T 升格为单一新核心（零真空，无清仓等待）");
        let nr = d.root().root_slot().unwrap();
        assert_eq!(d.root().instance(nr).direction, Polarity::Short, "新核心持空（顺势 down）");
        assert_eq!(d.root().n_promotes, 1);
        assert_eq!(d.root().n_clears, 0, "有回调子 T → promote 非 clear");
        assert_eq!(d.root().n_flips, 0, "顶级不 flip");
        assert!((d.root().total_wealth(100.0) - 100_000.0).abs() < 1e-4, "promote TW 中性");
    }

    #[test]
    fn top反向无回调子T_clear退化退出观望() {
        // §9.5 退化边界：顶级反转但无回调子 T 可升 → clear_root 退出观望（罕见，如缺口直接跳变）。
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10); // 仅核心，无回调子 T
        assert_eq!(d.root().n_active(), 1);
        d.on_view(&view(&[down(10, 20, false)]), 100.0, 20); // 顶反转，无回调先行
        assert_eq!(d.root().n_active(), 0, "无回调子 T → clear 退出观望");
        assert!(d.root().root_slot().is_none(), "root=None");
        assert_eq!(d.root().n_clears, 1);
        assert_eq!(d.root().n_promotes, 0);
        // 下一重跑重新入场（down 走势 → 顺势 short core）。
        d.on_view(&view(&[down(10, 25, false)]), 100.0, 25);
        assert_eq!(d.root().n_active(), 1, "下一重跑重新入场");
        assert_eq!(d.root().instance(d.root().root_slot().unwrap()).direction, Polarity::Short);
    }

    // ──────────────── 嵌套回调（自相似递归）────────────────

    #[test]
    fn 嵌套回调_子T再生孙T() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        // 三层链：root(Up) → 回调(Down,子T空) → 回调的回调(Up,孙T多)。
        d.on_view(
            &view(&[up(0, 14, false), down(10, 14, false), up(12, 14, false)]),
            100.0,
            14,
        );
        assert_eq!(d.root().n_active(), 3, "root + 子 T + 孙 T（自相似三层链）");
        let (lu, su) = d.root().exposure();
        assert!(lu > 0.0 && su > 0.0, "多（root核心+孙T）与空（子T）并存");
        assert!((d.root().total_wealth(100.0) - 100_000.0).abs() < 1e-4, "三层链 TW 中性");
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
    fn 全程tw中性_sink_recover_spawn_flip() {
        let mut d = RecDriver::new(100_000.0);
        d.on_view(&view(&[up(0, 10, false)]), 100.0, 10);
        d.on_view(&view(&[up(0, 12, false), down(10, 12, false)]), 120.0, 12);
        d.on_view(&view(&[up(0, 15, false), down(10, 14, true)]), 100.0, 15);
        d.on_view(&view(&[up(0, 30, false)]), 130.0, 30); // spawn
        d.on_view(&view(&[down(30, 40, false)]), 110.0, 40); // flip
        let fin = d.finish(110.0);
        // 全程同价段内 TW 中性守卫已逐操作 panic 验收；末态 final_nav 有限且合理。
        assert!(fin.is_finite() && fin > 0.0, "final_nav 有限，得 {fin}");
    }
}
