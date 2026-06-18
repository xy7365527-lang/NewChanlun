//! # 缠论统一递归算子 T
//!
//! 用一个泛型递归算子 T 识别全部级别的中枢、走势类型、买卖点。
//! 设计文档：`docs/unified_recursive_operator_T.md`。
//!
//! ## 架构：standalone，不依赖 v3 nucleus
//!
//! 本模块是 T 四步循环的 **standalone 实现**：自造中枢/走势/背驰逻辑，不依赖
//! `crate::level`/`crate::moves`/v3 任何现有引擎。这证明 T 的形式不变性（第65课
//! `aₙ=f(aₙ₋₁)`）可独立于 v3 手动分层 ladder 自洽成立——所有级别共用同一对纯函数
//! （`center::find_centers` + `trend::segment_into_trends`），递归深度由数据涌现上界
//! `r*`（§1.5）决定，不是 `MAX_LADDER` 常量。
//!
//! 核心命题（第65课）：`aₙ = f(aₙ₋₁)`——所有级别的递归形式相同，唯一不同的是
//! 预先给出的 `a₀`。本模块把 `f` 形式化为算子 `T`，把 bi/segment/move/recL2/…
//! 分层全部还原为 `T` 的迭代深度。
//!
//! ## 边界（本模块只做结构识别）
//!
//! T 属于 H⁰ 形态学构造轴（设计文档 §0.4）：它生成结构和方向。它**不**包含：
//! - 操作 / 仓位（H¹ 轴，设计文档 §4）；
//! - MACD 度量背驰确认（groupoid 轴精化，设计文档 §6.5）；
//! - a₀ 构造（分型→包含处理→笔→线段是 `T₀`，独立于 T，由调用方提供笔序列）。
//!
//! ## 四步循环（设计文档 §1.3）
//!
//! 输入级别 k 的单元序列 `S_k`，T 内部执行：
//! - a) [`center::find_centers`]：重叠检测 → 中枢（格运算，区间交集）；
//! - b) [`trend::segment_into_trends`]：走势类型识别（盘整 / 趋势）+ 方向；
//! - c) [`divergence::judge_divergence`]：走势完美判定（纯结构性背驰）→ type1；
//! - d) `operator::encapsulate`：封装终完美走势 → `S_{k+1}`（`Move(k) ≡ Level-(k+1) 笔`）。
//!
//! ## 买卖点 = 迭代不变量（设计文档 §3）
//!
//! 买卖点不是独立信号，是 T 迭代中走势类型切换的临界点：
//! - type1 = 走势完美时刻（步骤c 直接涌现，THE 不变量）；
//! - type3 = 中枢离开 + 回抽不破（步骤a 派生）；
//! - type2 = 次级别 type1 的跨级投影（[`iterate`] 驱动器投影，定律一）。
//!
//! ## 认识论等级（formalization-validity-domain）
//! - 四步循环的结构识别（a/b/d）= **L0**（从第17课定义直接推导，零信息增量）。
//! - 步骤 c 纯结构走势完美 = **L0 候选**（第37课5条件 + 嵌套深度，替代率未经 L2/L3 验证）。

pub mod center;
pub mod divergence;
pub mod operator;
pub mod trend;
pub mod types;

pub use operator::apply_t;
pub use types::{
    Direction, RecursiveTree, TLevelOutput, TrendKind, TrendType, Unit, Zhongshu, BSPKind, BSP,
};

/// Tᵏ 迭代驱动器：从 a₀（笔序列）迭代到涌现上界 r*。
///
/// 设计文档 §1.5 终止条件：
/// - 向上：每级只能迭代到 r*(t)（已有足够次级别走势构成的最高级别）。
///   实现上，当某级别产出的上级单元数 < 3（不足以再形成中枢）时停止；
/// - 当某级别无中枢（无走势类型）时停止。
///
/// 迭代结束后做 type2 跨级投影（[`project_type2`]）。
pub fn iterate(a0: Vec<Unit>) -> RecursiveTree {
    let mut levels: Vec<TLevelOutput> = Vec::new();
    let mut current = a0;
    let mut k = 0usize;

    loop {
        let out = apply_t(&current, k);

        // 本级别无走势类型（无中枢）→ 无法升级，停止（不记录空层）。
        if out.trends.is_empty() {
            break;
        }

        let next = out.next_units.clone();
        levels.push(out);

        // 下一级单元不足以再形成中枢（<3）→ r* 到达。
        if next.len() < 3 {
            break;
        }

        current = next;
        k += 1;

        // 防御性安全阀：r* 实际由数据涌现，此上界仅防御无限循环。
        if k > 64 {
            break;
        }
    }

    project_type2(&mut levels);
    RecursiveTree { levels }
}

/// type2 跨级投影（定律一：任何级别的第二类买卖点都由次级别相应走势的第一类买点构成）。
///
/// `type2_k` = 紧跟某个 level-k type1 之后、**同极性**的第一个 level-(k-1) type1
/// （次级别回抽确认）。设计文档 §3.2：`type2_k = type1_{k-1}` 的投影。
fn project_type2(levels: &mut [TLevelOutput]) {
    // 先快照各级别 type1：(bar, is_buy, price)。避免后续可变借用冲突。
    let type1s: Vec<Vec<(i64, bool, f64)>> = levels
        .iter()
        .map(|l| {
            l.bsps
                .iter()
                .filter(|b| matches!(b.kind, BSPKind::Type1Buy | BSPKind::Type1Sell))
                .map(|b| (b.bar, b.kind.is_buy(), b.price))
                .collect()
        })
        .collect();

    for k in 1..levels.len() {
        let mut new_t2: Vec<BSP> = Vec::new();
        for &(bar_k, buy_k, _) in &type1s[k] {
            // level k-1 中第一个 bar > bar_k 且同极性的 type1。
            let sub = type1s[k - 1]
                .iter()
                .filter(|&&(b, sub_buy, _)| b > bar_k && sub_buy == buy_k)
                .min_by_key(|&&(b, _, _)| b);
            if let Some(&(bar_sub, _, price_sub)) = sub {
                let kind = if buy_k {
                    BSPKind::Type2Buy
                } else {
                    BSPKind::Type2Sell
                };
                new_t2.push(BSP {
                    kind,
                    bar: bar_sub,
                    price: price_sub,
                    level: k,
                });
            }
        }
        levels[k].bsps.extend(new_t2);
        levels[k].bsps.sort_by_key(|b| b.bar);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    /// 8 笔上涨趋势（2 中枢 + 背驰末段）。
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

    #[test]
    fn iterate_单级别上涨趋势() {
        let tree = iterate(上涨趋势八笔());
        assert_eq!(tree.levels.len(), 1, "8 笔只够 1 级（封装出 1 根上级单元，不足 3 根）");
        assert_eq!(tree.emergent_ceiling(), 1);

        let l0 = &tree.levels[0];
        assert_eq!(l0.level, 0);
        assert_eq!(l0.centers.len(), 2);
        assert_eq!(l0.trends.len(), 1);
        assert_eq!(l0.trends[0].kind, TrendKind::UpTrend);

        let has_sell = l0.bsps.iter().any(|b| b.kind == BSPKind::Type1Sell && b.price == 45.0);
        assert!(has_sell, "应涌现 45.0 处一类卖点");
    }

    #[test]
    fn iterate_空输入不panic() {
        let tree = iterate(Vec::new());
        assert_eq!(tree.levels.len(), 0);
        assert_eq!(tree.emergent_ceiling(), 0);
    }

    #[test]
    fn iterate_无中枢序列停止() {
        // 单调抬高，无重叠，无中枢 → 无走势 → 空塔。
        let units = vec![
            bi(10.0, 15.0, 0, 1, Direction::Up),
            bi(16.0, 20.0, 1, 2, Direction::Up),
            bi(21.0, 25.0, 2, 3, Direction::Up),
        ];
        let tree = iterate(units);
        assert_eq!(tree.levels.len(), 0);
    }

    #[test]
    fn 下跌趋势对称产生一类买点() {
        // 上涨趋势八笔的价格镜像（关于 0 取反），方向翻转 → 一类买点。
        let units = vec![
            bi(-22.0, -8.0, 0, 1, Direction::Down),
            bi(-18.0, -12.0, 1, 2, Direction::Up),
            bi(-16.0, -10.0, 2, 3, Direction::Down),
            bi(-35.0, -23.0, 3, 4, Direction::Down),
            bi(-40.0, -32.0, 4, 5, Direction::Up),
            bi(-42.0, -33.0, 5, 6, Direction::Down),
            bi(-39.0, -31.0, 6, 7, Direction::Up),
            bi(-45.0, -40.0, 7, 8, Direction::Down),
        ];
        let tree = iterate(units);
        let l0 = &tree.levels[0];
        assert_eq!(l0.trends[0].kind, TrendKind::DownTrend);
        let has_buy = l0.bsps.iter().any(|b| b.kind == BSPKind::Type1Buy && b.price == -45.0);
        assert!(has_buy, "下跌背驰应产生 -45.0 处一类买点");
    }
}
