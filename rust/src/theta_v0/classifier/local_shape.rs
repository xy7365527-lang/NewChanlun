//! CC-006 `local_shape`：无包含三K的四种局部形态（#1370 TB-01-A）。
//!
//! ## 契约锚（[#1339 R2 SPEC](/home/agent/workspace/.chanlun/review-results/issue1339-r2-spec-20260909)）
//!
//! - 目录叶 `CC-006`（`SPEC-COVERAGE-INPUT.json` `/classification_axes/5`）：三根顺序标准 K，
//!   每对相邻均非包含、指标值完整。四分支：
//!   - `RISING`：`dir(a,b)=UP ∧ dir(b,c)=UP`
//!   - `TOP`：`dir(a,b)=UP ∧ dir(b,c)=DOWN`
//!   - `BOTTOM`：`dir(a,b)=DOWN ∧ dir(b,c)=UP`
//!   - `FALLING`：`dir(a,b)=DOWN ∧ dir(b,c)=DOWN`
//! - 域前件缺失（相邻包含 / 方向不确定 / 不足三根）**另报**，不造第五「Other」分型。
//!
//! ## 同次真实（不写第二份判据）
//!
//! `dir`/「非包含」直接复用 [`super::super::parser::inclusion`] 的 `strict_dir`/`contains`
//! （inclusion 同一判断），故本函数与现役 parser 的包含处理是同一份判据——不是另起一查法。
//! 本函数只做「三K窗口上的四分支分区」，分型强弱/中继描述另轴（ST-003 retained boundary），
//! 不在这里偷加。

use super::super::parser::inclusion::{contains, strict_dir};
use super::super::types::{Bar, Direction, Tick};

/// CC-006 四分支（目录叶名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cc006Branch {
    Rising,
    Top,
    Bottom,
    Falling,
}

impl Cc006Branch {
    pub fn as_str(self) -> &'static str {
        match self {
            Cc006Branch::Rising => "RISING",
            Cc006Branch::Top => "TOP",
            Cc006Branch::Bottom => "BOTTOM",
            Cc006Branch::Falling => "FALLING",
        }
    }
}

/// CC-006 域前件违反类型（不造第五「Other」分型，只报违反原因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cc006DomainViolation {
    /// 相邻两对至少一对存在包含（`contains(a,b) || contains(b,c)`）。
    AdjacentInclusion,
    /// 方向不确定（`strict_dir` 返回 None：等值/矛盾；理论上被非包含前件排除，防御保留）。
    NonStrictDir,
}

impl Cc006DomainViolation {
    pub fn as_str(self) -> &'static str {
        match self {
            Cc006DomainViolation::AdjacentInclusion => "adjacent_inclusion",
            Cc006DomainViolation::NonStrictDir => "non_strict_dir",
        }
    }
}

/// 比较轴（high / low）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonAxis {
    High,
    Low,
}

impl ComparisonAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            ComparisonAxis::High => "high",
            ComparisonAxis::Low => "low",
        }
    }
}

/// 相邻对（dir(a,b) 或 dir(b,c)）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cc006Pair {
    Ab,
    Bc,
}

impl Cc006Pair {
    pub fn as_str(self) -> &'static str {
        match self {
            Cc006Pair::Ab => "ab",
            Cc006Pair::Bc => "bc",
        }
    }
}

/// 一次严格比较（CC-006 四个严格比较之一，携带实际数值供同源展示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cc006Comparison {
    /// 比较轴：high / low。
    pub axis: ComparisonAxis,
    /// 相邻对：dir(a,b) 或 dir(b,c)。
    pub pair: Cc006Pair,
    /// 前一根 K 的该轴值（`a` 或 `b`）。
    pub prev: Tick,
    /// 后一根 K 的该轴值（`b` 或 `c`）。
    pub cur: Tick,
    /// 严格关系：`cur > prev`（Up）或 `cur < prev`（Down）。
    pub strict_up: bool,
    /// 该严格关系是否成立（四分支分区内恒 true）。
    pub held: bool,
}

/// CC-006 三K窗口判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cc006LocalShape {
    /// 恰一四分支 + 两次方向 + 四个严格比较（完整见证）。
    Classified {
        branch: Cc006Branch,
        dir_ab: Direction,
        dir_bc: Direction,
        comparisons: [Cc006Comparison; 4],
    },
    /// 不足三根（前两点缺右邻 = 知识不足，ST-003 / CC-006 domain_obligation）。
    InsufficientKnowledge,
    /// 域前件不满足（相邻包含 / 方向不确定），另报不造第五类，携带违反原因。
    DomainNotSatisfied { reason: Cc006DomainViolation },
}

/// CC-006 域前件检查（复用 [`contains`]/[`strict_dir`] 同一判断，不另起第二查法）。
///
/// 供 S 会话在喂入 parser 前对**原始**三K做域检查：相邻两对是否均非包含、方向是否确定。
/// 返回 `None` = 域满足；`Some(reason)` = 域违反（相邻包含 / 方向不确定）。
/// 此谓词与 [`classify_local_shape`] 内部的域门是同一判断——只把「是否违反」暴露给调用方，
/// 用于把**原始**输入里的包含（parser 会合并隐藏）作为「域不满足」正式结果发布，而不是
/// 让 parser 静默合并成更少根标准 K 后误报「知识不足」。
pub fn window_domain_violation(a: &Bar, b: &Bar, c: &Bar) -> Option<Cc006DomainViolation> {
    if contains(a, b) || contains(b, c) {
        Some(Cc006DomainViolation::AdjacentInclusion)
    } else if strict_dir(a, b).is_none() || strict_dir(b, c).is_none() {
        Some(Cc006DomainViolation::NonStrictDir)
    } else {
        None
    }
}

/// 对三根顺序 merged K 计算 CC-006 `local_shape`（四分支分区 + 见证）。
///
/// 边界条件（ST-003：顶同时 high/low 严格高于两邻、底对偶、等值不成分型、首尾/不足三根
/// 无可判分型；CC-006：前两点缺右邻为知识不足、不满足域另报）：
/// - 相邻包含（`contains(a,b) || contains(b,c)`）⟹ [`Cc006LocalShape::DomainNotSatisfied`]。
/// - `strict_dir` 返回 `None`（等值/矛盾，理论上被非包含前件排除，防御性保留）⟹ 同上。
/// - 四分支按 `(dir_ab, dir_bc)` 唯一确定。
pub fn classify_local_shape(a: &Bar, b: &Bar, c: &Bar) -> Cc006LocalShape {
    // 域前件：相邻两对均非包含（CC-006 quantified_domain.additional_domain_guard）。
    if let Some(reason) = window_domain_violation(a, b, c) {
        return Cc006LocalShape::DomainNotSatisfied { reason };
    }
    // 域满足 ⟹ 两次方向必为 Some（strict_dir 的 None 已被 window_domain_violation 拦截）。
    let dir_ab = strict_dir(a, b).expect("域满足时 dir(a,b) 必有方向");
    let dir_bc = strict_dir(b, c).expect("域满足时 dir(b,c) 必有方向");

    let branch = match (dir_ab, dir_bc) {
        (Direction::Up, Direction::Up) => Cc006Branch::Rising,
        (Direction::Up, Direction::Down) => Cc006Branch::Top,
        (Direction::Down, Direction::Up) => Cc006Branch::Bottom,
        (Direction::Down, Direction::Down) => Cc006Branch::Falling,
    };

    // 四个严格比较：dir_ab 与 dir_bc 各贡献 high/low 两条。`prev`/`cur` 分别取前一根/后一根
    // 的轴值，`strict_up` = 该方向要求 `cur > prev`。
    let cmp = |axis: ComparisonAxis, pair: Cc006Pair, prev: &Bar, cur: &Bar, up: bool| {
        let pv = match axis {
            ComparisonAxis::High => prev.high,
            ComparisonAxis::Low => prev.low,
        };
        let cv = match axis {
            ComparisonAxis::High => cur.high,
            ComparisonAxis::Low => cur.low,
        };
        let held = if up { cv > pv } else { cv < pv };
        Cc006Comparison {
            axis,
            pair,
            prev: pv,
            cur: cv,
            strict_up: up,
            held,
        }
    };

    let up_ab = dir_ab == Direction::Up;
    let up_bc = dir_bc == Direction::Up;
    let comparisons = [
        cmp(ComparisonAxis::High, Cc006Pair::Ab, a, b, up_ab),
        cmp(ComparisonAxis::Low, Cc006Pair::Ab, a, b, up_ab),
        cmp(ComparisonAxis::High, Cc006Pair::Bc, b, c, up_bc),
        cmp(ComparisonAxis::Low, Cc006Pair::Bc, b, c, up_bc),
    ];

    Cc006LocalShape::Classified {
        branch,
        dir_ab,
        dir_bc,
        comparisons,
    }
}

/// 对整段 merged K 序列滑窗计算 CC-006 `local_shape`（每窗口恰一叶或另报）。
///
/// 窗口 = `(merged[i], merged[i+1], merged[i+2])`。返回每窗口 `(start_merged_index, 结果)`，
/// 不足三根的窗口（`merged.len() < 3`）由调用方按「无实例」处理，本函数不产 phantom 叶。
pub fn classify_local_shape_sliding(merged: &[Bar]) -> Vec<(usize, Cc006LocalShape)> {
    if merged.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(merged.len() - 2);
    for i in 0..merged.len() - 2 {
        out.push((
            i,
            classify_local_shape(&merged[i], &merged[i + 1], &merged[i + 2]),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(src: usize, price: Tick) -> Bar {
        Bar {
            source_index: src,
            timestamp: src as i64,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 1.0,
            untradable: false,
        }
    }

    fn expect_branch(srcs: [usize; 3], prices: [Tick; 3], want: Cc006Branch) {
        let bars = [
            tick(srcs[0], prices[0]),
            tick(srcs[1], prices[1]),
            tick(srcs[2], prices[2]),
        ];
        match classify_local_shape(&bars[0], &bars[1], &bars[2]) {
            Cc006LocalShape::Classified {
                branch,
                dir_ab: _,
                dir_bc: _,
                comparisons,
            } => {
                assert_eq!(branch, want);
                // 四个严格比较全部 held（四分支分区内恒真）。
                assert!(comparisons.iter().all(|c| c.held));
            }
            other => panic!("期望 Classified，得到 {other:?}"),
        }
    }

    /// ★#1370 AC3 四分支 oracle：TestOnly 逐笔退化 OHLC（O=H=L=C），价格即单一极值。
    #[test]
    fn cc006_four_branch_oracle() {
        expect_branch([0, 1, 2], [10000, 10500, 11000], Cc006Branch::Rising);
        expect_branch([0, 1, 2], [10000, 11000, 10500], Cc006Branch::Top);
        expect_branch([0, 1, 2], [11000, 10000, 10500], Cc006Branch::Bottom);
        expect_branch([0, 1, 2], [11000, 10500, 10000], Cc006Branch::Falling);
    }

    /// 等值（含相等区间，属包含前件）⟹ DomainNotSatisfied，不造第五 Other。
    #[test]
    fn cc006_equal_is_domain_not_satisfied() {
        let a = tick(0, 10000);
        let b = tick(1, 10000);
        let c = tick(2, 11000);
        assert!(matches!(
            classify_local_shape(&a, &b, &c),
            Cc006LocalShape::DomainNotSatisfied {
                reason: Cc006DomainViolation::AdjacentInclusion
            }
        ));
    }

    /// 相邻包含 ⟹ DomainNotSatisfied（域前件）。
    #[test]
    fn cc006_inclusion_is_domain_not_satisfied() {
        // b 的 [low,high] 包含 c 的 [low,high]：b=[10000,11000] c=[10500,10800]。
        let a = Bar {
            source_index: 0,
            timestamp: 0,
            open: 10000,
            high: 10000,
            low: 10000,
            close: 10000,
            volume: 1.0,
            untradable: false,
        };
        let b = Bar {
            source_index: 1,
            timestamp: 1,
            open: 10000,
            high: 11000,
            low: 10000,
            close: 11000,
            volume: 1.0,
            untradable: false,
        };
        let c = Bar {
            source_index: 2,
            timestamp: 2,
            open: 10500,
            high: 10800,
            low: 10500,
            close: 10800,
            volume: 1.0,
            untradable: false,
        };
        assert!(matches!(
            classify_local_shape(&a, &b, &c),
            Cc006LocalShape::DomainNotSatisfied {
                reason: Cc006DomainViolation::AdjacentInclusion
            }
        ));
    }

    /// 滑窗：不足三根无实例；窗口分类 ≠ 后续成笔结果（只产窗口叶）。
    #[test]
    fn cc006_sliding_window_count() {
        let merged: Vec<Bar> = [10000, 10500, 11000, 10500]
            .iter()
            .enumerate()
            .map(|(i, &p)| tick(i, p))
            .collect();
        let wins = classify_local_shape_sliding(&merged);
        assert_eq!(wins.len(), 2);
        match &wins[0].1 {
            Cc006LocalShape::Classified { branch, .. } => assert_eq!(*branch, Cc006Branch::Rising),
            other => panic!("窗口0期望 Classified，得到 {other:?}"),
        }
        match &wins[1].1 {
            Cc006LocalShape::Classified { branch, .. } => assert_eq!(*branch, Cc006Branch::Top),
            other => panic!("窗口1期望 Classified，得到 {other:?}"),
        }
        assert!(classify_local_shape_sliding(&merged[..2]).is_empty());
    }
}
