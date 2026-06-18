//! T 步骤 a)：重叠检测 → 中枢。
//!
//! 第17课中枢递归定义：被至少三个连续次级别走势类型所重叠的部分。
//! 设计文档 §1.3 步骤a：
//! - 核心区间 `[ZD, ZG]` 由前两段定（编纂版口径）：`ZD = max(d_i, d_{i+1})`，
//!   `ZG = min(g_i, g_{i+1})`，成立条件 `ZG > ZD`；
//! - 第三段必须与核心区间重叠（满足「≥3 连续单元重叠」）；
//! - 外缘区间 `[DD, GG]` 由全部参与段定。
//!
//! 严格性标注（谱系 536）：重叠 = 区间交集 ∩，是**格运算**（lattice meet），不是
//! 群运算。中枢是 k 级走势重叠涌现的**新对象**，不能用 σ-等变描述。

use super::types::{Unit, Zhongshu};

/// 单元区间 `[low, high]` 是否与区间 `[zd, zg]` 有非空交集（严格重叠）。
#[inline]
fn overlaps(unit: &Unit, zd: f64, zg: f64) -> bool {
    unit.low < zg && unit.high > zd
}

/// 步骤 a)：从级别 k 的单元序列识别全部中枢。
///
/// 算法（贪心顺扫，设计文档 §1.3）：
/// 1. 在位置 i 用前两段 `units[i], units[i+1]` 定核心区间 `[ZD, ZG]`；
/// 2. 若 `ZG > ZD` 且第三段 `units[i+2]` 与核心重叠 → 中枢成立；
/// 3. 向后延伸：后续单元只要与核心区间重叠就纳入该中枢；
/// 4. 中枢终结于第一个不再重叠的单元，从该单元继续扫描下一个中枢。
///
/// 返回的 `Zhongshu.units` 索引相对于传入的 `units` 切片。
///
/// 已知结构简化（standalone 版特性，A/B 对比项）：贪心扫描会把"离开前一中枢的
/// 连接段"吸收为下一中枢的首单元，从而把下一中枢外缘 `DD` 拖低。因此只有当两中枢
/// **真正分离**（`后中枢 DD > 前中枢 GG`）时才会被 `same_direction_step` 判为趋势的
/// 一步；外缘重叠的相邻中枢按缠论视为盘整延伸（非趋势）。复用 nucleus 版
/// （`crate::recursive_t`）经由已验证的 `moves_from_level_zhongshus` 处理连接段，
/// 二者对连接段的处理差异正是两版对比的观察点之一。
pub fn find_centers(units: &[Unit], level: usize) -> Vec<Zhongshu> {
    let mut centers = Vec::new();
    let n = units.len();
    if n < 3 {
        return centers;
    }

    let mut i = 0usize;
    while i + 2 < n {
        // 核心区间由前两段定（编纂版口径）。
        let zd = units[i].low.max(units[i + 1].low);
        let zg = units[i].high.min(units[i + 1].high);
        let third_overlaps = overlaps(&units[i + 2], zd, zg);

        if zg > zd && third_overlaps {
            // 中枢成立。向后延伸纳入后续重叠单元。
            let mut last = i + 2;
            while last + 1 < n && overlaps(&units[last + 1], zd, zg) {
                last += 1;
            }

            // 外缘区间由全部参与段定。
            let mut gg = f64::MIN;
            let mut dd = f64::MAX;
            let mut idxs = Vec::with_capacity(last - i + 1);
            for j in i..=last {
                if units[j].high > gg {
                    gg = units[j].high;
                }
                if units[j].low < dd {
                    dd = units[j].low;
                }
                idxs.push(j);
            }

            centers.push(Zhongshu {
                high: zg,
                low: zd,
                gg,
                dd,
                units: idxs,
                level,
            });
            i = last + 1;
        } else {
            i += 1;
        }
    }

    centers
}

/// 判断两个相邻中枢是否「依次同向」构成趋势的一步（第17课 + 中心定理二）。
///
/// 用外缘区间 `[DD, GG]` 判定（设计文档 §1.3 步骤b 同向判据）：
/// - 后中枢 `DD > 前中枢 GG` → 向上一步；
/// - 后中枢 `GG < 前中枢 DD` → 向下一步；
/// - 两中枢外缘有重叠 → `None`（不是趋势的一步，是盘整延伸/方向断裂）。
pub fn same_direction_step(prev: &Zhongshu, next: &Zhongshu) -> Option<super::types::Direction> {
    use super::types::Direction;
    if next.dd > prev.gg {
        Some(Direction::Up)
    } else if next.gg < prev.dd {
        Some(Direction::Down)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::types::Direction;

    /// 构造一根测试笔。
    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    #[test]
    fn 三笔重叠成中枢() {
        // 三笔区间 [10,20] [12,22]... 前两段核心 ZD=max(10,12)=12, ZG=min(20,22)=20
        // 第三段 [11,21] 与 [12,20] 重叠 → 中枢成立。
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 1);
        let c = &centers[0];
        assert_eq!(c.low, 12.0); // ZD
        assert_eq!(c.high, 20.0); // ZG
        assert_eq!(c.dd, 10.0); // DD 全部段最低
        assert_eq!(c.gg, 22.0); // GG 全部段最高
        assert_eq!(c.units, vec![0, 1, 2]);
    }

    #[test]
    fn 不重叠不成中枢() {
        // 三笔彼此不重叠（逐级抬高），无公共区间。
        let units = vec![
            bi(10.0, 15.0, 0, 1, Direction::Up),
            bi(16.0, 20.0, 1, 2, Direction::Up),
            bi(21.0, 25.0, 2, 3, Direction::Up),
        ];
        let centers = find_centers(&units, 1);
        assert!(centers.is_empty());
    }

    #[test]
    fn 中枢向后延伸吸收重叠单元() {
        // 四笔都重叠在 [12,20] 上 → 一个中枢吸收全部四段。
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(13.0, 19.0, 3, 4, Direction::Down),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 1);
        assert_eq!(centers[0].units, vec![0, 1, 2, 3]);
    }

    #[test]
    fn 两个不重叠中枢_上涨同向() {
        // 中枢1 核心 [12,18] 外缘 [8,22]；连接段 low=23 > 中枢1 GG=22（真正分离）；
        // 中枢2 核心 [32,35] 外缘 DD=23 > 中枢1 GG=22 → 同向上趋势的一步。
        let units = vec![
            bi(8.0, 22.0, 0, 1, Direction::Up),
            bi(12.0, 18.0, 1, 2, Direction::Down),
            bi(10.0, 16.0, 2, 3, Direction::Up),
            // 离开向上（low=23 > 中枢1 GG=22）
            bi(23.0, 35.0, 3, 4, Direction::Up),
            // 中枢2
            bi(32.0, 40.0, 4, 5, Direction::Down),
            bi(33.0, 42.0, 5, 6, Direction::Up),
            bi(31.0, 39.0, 6, 7, Direction::Down),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 2);
        let step = same_direction_step(&centers[0], &centers[1]);
        assert_eq!(step, Some(Direction::Up));
    }
}
