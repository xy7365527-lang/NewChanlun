//! 算子 T：单级别迭代一次（组合步骤 a→b→c→d）。
//!
//! 设计文档 §1.3。`apply_t` 是纯函数：输入级别 k 的单元序列，输出该级别的中枢、
//! 走势类型、买卖点，以及封装产出的级别 k+1 单元序列 `S_{k+1}`。无副作用、无状态突变。

use super::center::find_centers;
use super::divergence::judge_divergence;
use super::trend::segment_into_trends;
use super::types::{BSPKind, Direction, TLevelOutput, TrendType, Unit, Zhongshu, BSP};

/// 步骤 d)：把一个终完美的走势类型封装为级别 k+1 的一根单元。
///
/// 递归恒等式（谱系 540）：`Move(k) ≡ Level-(k+1) 的笔`。封装携带：
/// - 区间 = 走势全部单元的 [min low, max high]；
/// - 方向 = 走势方向（压缩视图）；
/// - `inner_zhongshu_count` = 走势的中枢数（嵌套深度，供上级背驰判定，§6.4）。
fn encapsulate(t: &TrendType, next_level: usize) -> Unit {
    let high = t.units.iter().map(|u| u.high).fold(f64::MIN, f64::max);
    let low = t.units.iter().map(|u| u.low).fold(f64::MAX, f64::min);
    Unit {
        high,
        low,
        start_bar: t.units.first().unwrap().start_bar,
        end_bar: t.units.last().unwrap().end_bar,
        direction: t.direction,
        level: next_level,
        inner_zhongshu_count: t.zhongshus.len(),
    }
}

/// type3 派生（步骤a 的伴随）：中枢离开 + 回抽不破 ZG/ZD。
///
/// 第三类买点：次级别走势向上离开中枢，回抽低点不跌破 ZG → Type3Buy。
/// 第三类卖点：向下离开中枢，回抽高点不升破 ZD → Type3Sell。
///
/// `centers` 的 `units` 索引相对于 `units`（绝对坐标，来自 `find_centers`）。
fn detect_type3(units: &[Unit], centers: &[Zhongshu], level: usize) -> Vec<BSP> {
    let mut res = Vec::new();
    for c in centers {
        let after = *c.units.last().unwrap() + 1;
        if after + 1 >= units.len() {
            continue;
        }
        let leave = &units[after];
        let pull = &units[after + 1];

        // 向上离开（离开段低点高于 ZG）+ 回抽不破 ZG。
        if leave.low > c.high && pull.low > c.high {
            res.push(BSP {
                kind: BSPKind::Type3Buy,
                bar: pull.end_bar,
                price: pull.low,
                level,
            });
        }
        // 向下离开（离开段高点低于 ZD）+ 回抽不破 ZD。
        if leave.high < c.low && pull.high < c.low {
            res.push(BSP {
                kind: BSPKind::Type3Sell,
                bar: pull.end_bar,
                price: pull.high,
                level,
            });
        }
    }
    res
}

/// 算子 T：对级别 k 的单元序列迭代一次。
///
/// 1. 步骤a：`find_centers` → 中枢（绝对坐标）；
/// 2. 步骤b：`segment_into_trends` → 走势类型实例（自包含切片，索引已重基）；
/// 3. 步骤c：`judge_divergence` → type1 买卖点 + 终完美标记；
///    （被反向走势终结的中间走势也标记 completed，但无显式 type1 BSP）；
/// 4. 步骤d：`encapsulate` 终完美走势 → `S_{k+1}`；
/// 5. 伴随：`detect_type3` → type3 买卖点。
pub fn apply_t(units: &[Unit], level: usize) -> TLevelOutput {
    // 步骤 a
    let centers = find_centers(units, level);
    // 步骤 b
    let mut trends = segment_into_trends(units, &centers, level);

    // 步骤 c：背驰判定 + 完成标记
    let n_groups = trends.len();
    for (gi, t) in trends.iter_mut().enumerate() {
        if let Some(bsp) = judge_divergence(t) {
            t.bsp = Some(bsp);
            t.completed = true;
        } else if gi + 1 < n_groups {
            // 中间走势被后一个反向走势终结：完成（append-only 冻结），无显式背驰。
            t.completed = true;
        }
    }

    // 步骤 d：封装终完美走势为上级单元
    let next_units: Vec<Unit> = trends
        .iter()
        .filter(|t| t.completed)
        .map(|t| encapsulate(t, level + 1))
        .collect();

    // 买卖点汇总：type1（步骤c）+ type3（伴随）
    let mut bsps: Vec<BSP> = trends.iter().filter_map(|t| t.bsp.clone()).collect();
    bsps.extend(detect_type3(units, &centers, level));
    bsps.sort_by_key(|b| b.bar);

    TLevelOutput {
        level,
        centers,
        trends,
        bsps,
        next_units,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::types::TrendKind;

    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    /// 8 笔上涨趋势（2 中枢 + 背驰末段），验证 apply_t 一次迭代的完整产出。
    fn 上涨趋势八笔() -> Vec<Unit> {
        vec![
            bi(8.0, 22.0, 0, 1, Direction::Up),   // a 进入段，振幅大
            bi(12.0, 18.0, 1, 2, Direction::Down),
            bi(10.0, 16.0, 2, 3, Direction::Up),  // 中枢1 = 0,1,2 核心[12,18]
            bi(23.0, 35.0, 3, 4, Direction::Up),  // 离开向上
            bi(32.0, 40.0, 4, 5, Direction::Down),
            bi(33.0, 42.0, 5, 6, Direction::Up),  // 中枢2 = 3,4,5,6 核心[32,35]
            bi(31.0, 39.0, 6, 7, Direction::Down),
            bi(40.0, 45.0, 7, 8, Direction::Up),  // c 段创新高 45 但振幅弱
        ]
    }

    #[test]
    fn apply_t_识别两中枢一趋势一卖点() {
        let units = 上涨趋势八笔();
        let out = apply_t(&units, 0);
        assert_eq!(out.centers.len(), 2, "应识别 2 个中枢");
        assert_eq!(out.trends.len(), 1, "应识别 1 个走势类型");
        assert_eq!(out.trends[0].kind, TrendKind::UpTrend);

        let sells: Vec<_> = out
            .bsps
            .iter()
            .filter(|b| b.kind == BSPKind::Type1Sell)
            .collect();
        assert_eq!(sells.len(), 1, "应产生 1 个一类卖点");
        assert_eq!(sells[0].price, 45.0);
        assert_eq!(sells[0].level, 0);
    }

    #[test]
    fn apply_t_封装产出上级单元() {
        let units = 上涨趋势八笔();
        let out = apply_t(&units, 0);
        assert_eq!(out.next_units.len(), 1, "终完美趋势封装为 1 根上级单元");
        let up = &out.next_units[0];
        assert_eq!(up.level, 1);
        assert_eq!(up.direction, Direction::Up);
        assert_eq!(up.inner_zhongshu_count, 2, "携带 2 个中枢的嵌套深度");
        assert_eq!(up.low, 8.0); // 全段最低
        assert_eq!(up.high, 45.0); // 全段最高
        assert_eq!(up.start_bar, 0);
        assert_eq!(up.end_bar, 8);
    }

    #[test]
    fn apply_t_是纯函数_不改输入() {
        let units = 上涨趋势八笔();
        let snapshot = units.clone();
        let _ = apply_t(&units, 0);
        assert_eq!(units, snapshot, "apply_t 不得突变输入");
    }

    #[test]
    fn apply_t_泛型_同一套代码作用于level1单元() {
        // 验证「所有级别用同一套代码」：把 level-1 单元（带 inner_zhongshu_count）
        // 喂给同一个 apply_t，走嵌套深度档背驰，产出 level-2 结构。
        let mk = |low, high, s, e, d, nest| Unit {
            high,
            low,
            start_bar: s,
            end_bar: e,
            direction: d,
            level: 1,
            inner_zhongshu_count: nest,
        };
        let units = vec![
            mk(8.0, 22.0, 0, 1, Direction::Up, 3),
            mk(12.0, 18.0, 1, 2, Direction::Down, 2),
            mk(10.0, 16.0, 2, 3, Direction::Up, 2),
            mk(23.0, 35.0, 3, 4, Direction::Up, 2),
            mk(32.0, 40.0, 4, 5, Direction::Down, 2),
            mk(33.0, 42.0, 5, 6, Direction::Up, 2),
            mk(31.0, 39.0, 6, 7, Direction::Down, 2),
            mk(40.0, 60.0, 7, 8, Direction::Up, 2), // c 创新高，含 2 次级别中枢，弱于 a 段
        ];
        let out = apply_t(&units, 1);
        assert_eq!(out.level, 1);
        assert_eq!(out.centers.len(), 2);
        let sells: Vec<_> = out.bsps.iter().filter(|b| b.kind == BSPKind::Type1Sell).collect();
        assert_eq!(sells.len(), 1);
        assert_eq!(out.next_units[0].level, 2, "封装到 level 2");
    }
}
