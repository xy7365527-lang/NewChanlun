//! T 步骤 b)：走势类型识别（盘整 / 趋势）+ 方向。
//!
//! 第17课：
//! - 盘整：某完成的走势类型只包含一个走势中枢；
//! - 趋势：至少包含两个以上依次同向的走势中枢。
//!
//! 设计文档 §1.3 步骤b。本步骤把级别 k 的单元序列按「依次同向的中枢分组」切分为
//! 多个走势类型实例，每个实例后续封装为一个级别 k+1 单元（步骤d）。

use super::center::same_direction_step;
use super::types::{Direction, TrendKind, TrendType, Unit, Zhongshu};

/// 把中枢序列按「依次同向」分组：每组中枢构成一个走势类型实例。
///
/// 分组规则：
/// - 单个中枢自成一组（盘整），除非它与下一个中枢同向且能续上趋势；
/// - 连续同向的中枢归入同一组（趋势）；
/// - 方向断裂（两中枢外缘重叠或反向）→ 收尾当前组、新开一组。
fn group_centers_by_direction(centers: &[Zhongshu]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    if centers.is_empty() {
        return groups;
    }

    let mut cur: Vec<usize> = vec![0];
    let mut cur_dir: Option<Direction> = None;

    for ci in 1..centers.len() {
        let prev_idx = *cur.last().unwrap();
        let step = same_direction_step(&centers[prev_idx], &centers[ci]);
        match (cur_dir, step) {
            // 当前组尚无方向，本步定下方向。
            (None, Some(d)) => {
                cur_dir = Some(d);
                cur.push(ci);
            }
            // 与当前组方向一致：趋势延伸。
            (Some(cd), Some(d)) if cd == d => {
                cur.push(ci);
            }
            // 方向断裂或反向：收尾，新开组。
            _ => {
                groups.push(std::mem::take(&mut cur));
                cur = vec![ci];
                cur_dir = None;
            }
        }
    }
    groups.push(cur);
    groups
}

/// 计算一段单元的净方向（用于盘整：盘整无趋势方向，取首尾中点的净位移）。
fn net_direction(units: &[Unit]) -> Direction {
    debug_assert!(!units.is_empty());
    let first = &units[0];
    let last = &units[units.len() - 1];
    let first_mid = (first.high + first.low) / 2.0;
    let last_mid = (last.high + last.low) / 2.0;
    if last_mid >= first_mid {
        Direction::Up
    } else {
        Direction::Down
    }
}

/// 步骤 b)：把级别 k 单元序列切分并分类为走势类型实例。
///
/// 单元到走势的归属：按中枢分组边界把 `units` 连续切分。
/// - 第 0 组从 `units[0]` 开始（含进入段 a）；
/// - 第 i 组从「上一组结束的下一个单元」开始；
/// - 边界 = 下一组首个中枢的首个参与单元（绝对索引）。
///
/// 返回的每个 `TrendType.units` 是自包含的克隆切片，其内部 `zhongshus.units`
/// 索引已**重基**到该切片（减去切片起点）。
pub fn segment_into_trends(units: &[Unit], centers: &[Zhongshu], level: usize) -> Vec<TrendType> {
    let groups = group_centers_by_direction(centers);
    if groups.is_empty() {
        return Vec::new();
    }

    let n = units.len();
    let n_groups = groups.len();

    // 每组的起始绝对单元索引：第 0 组=0；第 i 组=该组首中枢首单元。
    let mut starts: Vec<usize> = Vec::with_capacity(n_groups);
    for (gi, g) in groups.iter().enumerate() {
        if gi == 0 {
            starts.push(0);
        } else {
            let first_center = &centers[g[0]];
            starts.push(*first_center.units.first().unwrap());
        }
    }

    let mut trends = Vec::with_capacity(n_groups);
    for gi in 0..n_groups {
        let us = starts[gi];
        let ue = if gi + 1 < n_groups {
            starts[gi + 1] // 独占到下一组起点之前
        } else {
            n
        };
        if us >= ue {
            continue; // 退化空段保护
        }

        // 克隆单元切片，自包含。
        let slice: Vec<Unit> = units[us..ue].to_vec();

        // 重基本组中枢的单元索引到切片坐标。
        let group_centers: Vec<Zhongshu> = groups[gi]
            .iter()
            .map(|&ci| {
                let c = &centers[ci];
                Zhongshu {
                    high: c.high,
                    low: c.low,
                    gg: c.gg,
                    dd: c.dd,
                    units: c.units.iter().map(|&u| u - us).collect(),
                    level: c.level,
                }
            })
            .collect();

        // 分类 + 方向。
        let (kind, direction) = classify(&group_centers, &slice);

        trends.push(TrendType {
            kind,
            zhongshus: group_centers,
            units: slice,
            level,
            direction,
            completed: false,
            bsp: None,
        });
    }

    trends
}

/// 走势分类与方向判定（第17课 + 中心定理二）。
fn classify(centers: &[Zhongshu], units: &[Unit]) -> (TrendKind, Direction) {
    if centers.len() >= 2 {
        // 趋势：方向由依次同向的中枢决定。用首尾中枢外缘判定。
        let first = &centers[0];
        let last = &centers[centers.len() - 1];
        if last.dd > first.gg {
            (TrendKind::UpTrend, Direction::Up)
        } else if last.gg < first.dd {
            (TrendKind::DownTrend, Direction::Down)
        } else {
            // 理论上分组已保证同向；防御性退化为盘整。
            (TrendKind::Consolidation, net_direction(units))
        }
    } else {
        // 盘整：单中枢，方向取净位移。
        (TrendKind::Consolidation, net_direction(units))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::center::find_centers;

    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    #[test]
    fn 单中枢识别为盘整() {
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
        ];
        let centers = find_centers(&units, 1);
        let trends = segment_into_trends(&units, &centers, 1);
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].kind, TrendKind::Consolidation);
    }

    /// 真正分离的两中枢上涨：连接段 low=23 > 中枢1 GG=22（见 center.rs 连接段说明）。
    fn 上涨趋势两中枢七笔() -> Vec<Unit> {
        vec![
            bi(8.0, 22.0, 0, 1, Direction::Up),
            bi(12.0, 18.0, 1, 2, Direction::Down),
            bi(10.0, 16.0, 2, 3, Direction::Up),
            bi(23.0, 35.0, 3, 4, Direction::Up),
            bi(32.0, 40.0, 4, 5, Direction::Down),
            bi(33.0, 42.0, 5, 6, Direction::Up),
            bi(31.0, 39.0, 6, 7, Direction::Down),
        ]
    }

    #[test]
    fn 两同向中枢识别为上涨趋势() {
        let units = 上涨趋势两中枢七笔();
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 2);
        let trends = segment_into_trends(&units, &centers, 1);
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].kind, TrendKind::UpTrend);
        assert_eq!(trends[0].direction, Direction::Up);
        // 两个中枢都归入这一个趋势。
        assert_eq!(trends[0].zhongshus.len(), 2);
    }

    #[test]
    fn 中枢索引重基到切片坐标() {
        // 趋势的单元从 0 开始，重基后首中枢首单元索引应为 0。
        let units = 上涨趋势两中枢七笔();
        let centers = find_centers(&units, 1);
        let trends = segment_into_trends(&units, &centers, 1);
        assert_eq!(trends[0].zhongshus[0].units[0], 0);
    }
}
