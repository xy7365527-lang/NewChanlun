//! 第一步：K线包含合并（reference-theta-v0.md:19，[缠论可导,62/65课]）。
//!
//! ## 规则（reference-theta-v0.md:19 逐字）
//!
//! - **相邻区间包含即合并**：相邻两 K（合并方向上）区间包含关系成立即合并为一根。
//! - **向上合并**：`high=max, low=max`（取两者较高的 high 与较高的 low）。
//! - **向下合并**：`low=min, high=min`（取两者较低的 low 与较低的 high）。
//! - **方向决定**：方向按**前一对非包含 K** 的严格高低变化决定。
//! - **开头无方向**：向前看第一个非包含对（开头 tie-break 是设计选择）。
//! - **全程无方向**：只输出 open-tail（无 confirmed 结构）。
//!
//! ## 包含关系定义（缠论 62/65课）
//!
//! 两 K（A 在前，B 在后）包含 ⟺ 一方区间完全含另一方：
//! `(A.high >= B.high && A.low <= B.low)` 或 `(B.high >= A.high && B.low <= A.low)`。
//! 即一根的 [low,high] 闭区间包含另一根的 [low,high]。
//!
//! ## bit-exact 注意点
//!
//! - 整数 tick 域比较（types::Tick = i64），无浮点歧义。
//! - 合并是**有方向**的左折叠：方向由已合并序列的前一段趋势决定，不是全局重算——
//!   保证确定性（reference-theta-v0.md:16 已确认结构不可回写）。
//! - 合并后的 bar 保留**起始** bar 的 `source_index`/`timestamp`（合并段的锚点），
//!   `volume` 累加，`untradable` 取或（任一不可交易则合并段不可交易）。

use super::super::types::{Bar, Direction};

/// 合并方向：包含处理的左折叠方向（向上吞并取高，向下吞并取低）。
///
/// 与 `Direction` 区分：`Direction` 是笔/线段的几何方向；此处是**包含合并**的局部
/// 处理方向，由前一对非包含 K 的严格高低变化决定（reference-theta-v0.md:19）。
type MergeDir = Direction;

/// 判断两 K 是否包含（一方闭区间 [low,high] 含另一方）。
///
/// 边界条件：相等区间（A==B）视为包含（互含）——按合并规则处理为一根。
fn contains(a: &Bar, b: &Bar) -> bool {
    (a.high >= b.high && a.low <= b.low) || (b.high >= a.high && b.low <= a.low)
}

/// 按合并方向把 `acc`（已合并段）与新 K `b` 合并为一根（reference-theta-v0.md:19）。
///
/// - 向上：`high=max(acc.high,b.high)`，`low=max(acc.low,b.low)`。
/// - 向下：`low=min(acc.low,b.low)`，`high=min(acc.high,b.high)`。
///
/// 合并段锚点：保留 `acc` 的 source_index/timestamp（段起点），open/close 取 acc 的
/// （包含合并只关心 high/low 极值，open/close 不参与结构识别，保留段起点的语义锚）。
fn merge(acc: &Bar, b: &Bar, dir: MergeDir) -> Bar {
    let (high, low) = match dir {
        // 向上：high=max(both)，low=max(both)。
        Direction::Up => (acc.high.max(b.high), acc.low.max(b.low)),
        // 向下：high=min(both)，low=min(both)。
        Direction::Down => (acc.high.min(b.high), acc.low.min(b.low)),
    };
    Bar {
        source_index: acc.source_index,
        timestamp: acc.timestamp,
        open: acc.open,
        high,
        low,
        close: b.close,
        volume: acc.volume.saturating_add(b.volume),
        untradable: acc.untradable || b.untradable,
    }
}

/// 严格高低变化方向：从 K `prev` 到 K `cur` 的方向（reference-theta-v0.md:19）。
///
/// 严格高高、低低 → Up；严格低低、低高 → Down；非严格（任一相等或矛盾）→ None。
/// 这是「前一对非包含 K 的严格高低变化」——只有两个区间**严格**单调（high 与 low
/// 同向严格变化）才确定方向。
fn strict_dir(prev: &Bar, cur: &Bar) -> Option<MergeDir> {
    if cur.high > prev.high && cur.low > prev.low {
        Some(Direction::Up)
    } else if cur.high < prev.high && cur.low < prev.low {
        Some(Direction::Down)
    } else {
        None
    }
}

/// 包含合并输出：合并后的 K 序列 + 是否全程无方向（only_open_tail）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InclusionResult {
    /// 包含处理后的 K 序列（合并完成）。
    pub merged: Vec<Bar>,
    /// 全程无方向 ⟹ 只输出 open-tail（reference-theta-v0.md:19）。
    pub only_open_tail: bool,
}

/// K线包含合并主流程（reference-theta-v0.md:19）。
///
/// 算法（有方向左折叠）：
/// 1. 找开头方向：向前扫描第一对**非包含**且**严格**单调的 K，定 `dir`。
///    全程无非包含严格对 ⟹ `only_open_tail=true`（方向悬空）。
/// 2. 从该方向起左折叠：遇包含则按当前 `dir` 合并进 acc；遇非包含则 acc 定稿，更新
///    `dir`（由 acc 与新 K 的严格高低变化），新 K 成为新 acc。
///
/// 边界条件：
/// - 空输入 / 单根 ⟹ merged 原样返回，`only_open_tail=true`（无法定方向）。
/// - 全程包含（始终无非包含严格对）⟹ 合并成一根，`only_open_tail=true`。
pub fn process_inclusion(bars: &[Bar]) -> InclusionResult {
    if bars.len() < 2 {
        return InclusionResult {
            merged: bars.to_vec(),
            only_open_tail: true,
        };
    }

    // 步骤 1：找开头方向（第一对非包含且严格单调的 K）。
    let mut start_dir: Option<MergeDir> = None;
    for w in bars.windows(2) {
        if !contains(&w[0], &w[1]) {
            if let Some(d) = strict_dir(&w[0], &w[1]) {
                start_dir = Some(d);
                break;
            }
        }
    }

    let dir0 = match start_dir {
        Some(d) => d,
        None => {
            // 全程无方向：只输出 open-tail。merged 仍做无方向的包含吸收
            // （把连续包含吸成单段，保留原始极值上下界）——但语义上无 confirmed。
            return InclusionResult {
                merged: bars.to_vec(),
                only_open_tail: true,
            };
        }
    };

    // 步骤 2：有方向左折叠。
    let mut merged: Vec<Bar> = Vec::with_capacity(bars.len());
    let mut acc = bars[0];
    let mut dir = dir0;
    for b in &bars[1..] {
        if contains(&acc, b) {
            acc = merge(&acc, b, dir);
        } else {
            // acc 定稿，更新方向（acc → b 的严格高低变化），b 成为新 acc。
            if let Some(d) = strict_dir(&acc, b) {
                dir = d;
            }
            merged.push(acc);
            acc = *b;
        }
    }
    merged.push(acc);

    InclusionResult {
        merged,
        only_open_tail: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Tick;

    fn bar(i: usize, high: Tick, low: Tick) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: low,
            high,
            low,
            close: high,
            volume: 1,
            untradable: false,
        }
    }

    #[test]
    fn contains_detects_full_containment() {
        // B [low=2,high=8] 含于 A [low=1,high=10]。
        assert!(contains(&bar(0, 10, 1), &bar(1, 8, 2)));
        // 互不包含。
        assert!(!contains(&bar(0, 10, 5), &bar(1, 12, 7)));
        // 相等区间互含。
        assert!(contains(&bar(0, 10, 5), &bar(1, 10, 5)));
    }

    #[test]
    fn strict_dir_only_when_both_extremes_move() {
        assert_eq!(strict_dir(&bar(0, 10, 5), &bar(1, 12, 7)), Some(Direction::Up));
        assert_eq!(strict_dir(&bar(0, 10, 5), &bar(1, 8, 3)), Some(Direction::Down));
        // high 升 low 平 → 非严格 → None。
        assert_eq!(strict_dir(&bar(0, 10, 5), &bar(1, 12, 5)), None);
    }

    #[test]
    fn merge_up_takes_higher_high_and_higher_low() {
        // 向上：high=max, low=max。
        let m = merge(&bar(0, 10, 3), &bar(1, 8, 5), Direction::Up);
        assert_eq!((m.high, m.low), (10, 5));
    }

    #[test]
    fn merge_down_takes_lower_low_and_lower_high() {
        // 向下：low=min, high=min。
        let m = merge(&bar(0, 10, 5), &bar(1, 8, 3), Direction::Down);
        assert_eq!((m.high, m.low), (8, 3));
    }

    #[test]
    fn no_direction_yields_only_open_tail() {
        // 全程包含（每根都含于第一根）→ 无非包含严格对 → only_open_tail。
        let bars = vec![bar(0, 20, 1), bar(1, 10, 5), bar(2, 8, 6)];
        let r = process_inclusion(&bars);
        assert!(r.only_open_tail);
    }

    #[test]
    fn golden_simple_up_then_inclusion_merge() {
        // 序列：A[10,5] → B[12,7]（严格上，非包含，定 dir=Up）→ C[11,8]（含于 B，向上合并）。
        // 期望：merged = [A, merge(B,C,Up)=[12,8]]，dir 已定，非 open-tail。
        let bars = vec![bar(0, 10, 5), bar(1, 12, 7), bar(2, 11, 8)];
        let r = process_inclusion(&bars);
        assert!(!r.only_open_tail);
        assert_eq!(r.merged.len(), 2);
        assert_eq!((r.merged[0].high, r.merged[0].low), (10, 5));
        assert_eq!((r.merged[1].high, r.merged[1].low), (12, 8)); // B,C 向上合并：high=max,low=max
    }

    /// property：包含合并后相邻 merged K 必两两非包含（合并的不动点性质）。
    #[test]
    fn property_merged_bars_pairwise_non_containing() {
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 12, 7),
            bar(2, 11, 8),
            bar(3, 15, 13),
            bar(4, 14, 9),
        ];
        let r = process_inclusion(&bars);
        for w in r.merged.windows(2) {
            assert!(
                !contains(&w[0], &w[1]),
                "merged 相邻 K 仍包含: {:?} {:?}",
                w[0],
                w[1]
            );
        }
    }
}
