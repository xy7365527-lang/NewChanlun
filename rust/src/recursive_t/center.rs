//! T 步骤 a)：重叠检测 → 中枢。
//!
//! 第17课中枢递归定义：被至少三个连续次级别走势类型所重叠的部分。
//! 设计文档 §1.3 步骤a：
//! - 核心区间 `[ZD, ZG]` 由前三段定（第18课公式：`ZD = max(d1,d2,d3)`，
//!   `ZG = min(g1,g2,g3)`，v3 nucleus `zhongshu_from_components` 口径），成立条件 `ZG > ZD`；
//! - 外缘区间 `[DD, GG]` 由全部参与段定。
//!
//! c 段裁决（`docs/c_segment_attribution_reasoning.md`，编排者 2026-06-20 指示移植修复A）：
//! 中枢终结后**跳过突破段（离开段/连接段）**再扫下一中枢（`i = last+2`，见 [`find_centers`]），
//! 使连接段成为中枢间间隙 = 本走势 c 段，修复 c 段 79% 缺失（趋势背驰几乎从不完美）。
//!
//! 严格性标注（谱系 536）：重叠 = 区间交集 ∩，是**格运算**（lattice meet），不是
//! 群运算。中枢是 k 级走势重叠涌现的**新对象**，不能用 σ-等变描述。

use super::types::{Unit, Zhongshu};

/// 单元区间 `[low, high]` 是否与中枢核心区间 `[zd, zg]` 有交集（**闭区间**，含端点）。
///
/// ★口径（#290 裁定 A，2026-07-26 用户裁决；#314 落 Python 侧，#322 落本处）：含端点
/// `<=` / `>=`——相切（`unit.low == zg` 或 `unit.high == zd`）**算重叠 ⟹ 中枢延伸**。
///
/// 原文锚：中心定理一（`docs/chanlun/text/blog/020-第20课.md:56`）「走势中枢的延伸等价于
/// 任意区间[dn，gn]与[ZD，ZG]有重叠。换言之，若有Zn，使得dn>ZG或gn<ZD，则必然产生高级别的
/// 走势中枢或趋势及延续。」——脱离条件用**严格**不等 ⟹ 端点相等不构成脱离 ⟹ 仍属有重叠。
///
/// 对齐对象：Lean `Origin.CenterStates.CenterExtension`（弱）/`CenterBroken`（严格）、
/// v1 `a_zhongshu_v1._extend_zhongshu`（`:104`）、v0 `a_center_v0._has_overlap`（#314 已弱）、
/// `a_level_fsm_newchan.overlap`（#314 已弱）、#246「相切=重合」全域口径。本处是第四实现
/// （#322 票面「第四实现分歧」），改弱后四实现同口径。
///
/// 调研：`chanlun/review-results/center-tangency-doctrine-20260726.md` §2.1；爆炸半径
/// `.chanlun/review-results/center-tangency-blast-radius-20260726.md` §79 行。
///
/// **不改的两处严格**（同属定理一，方向相反）：`detect_type3` 的离开判据
/// （`leave.low > c.high`）与 `same_direction_step` 的外缘分离判据（`next.dd > prev.gg`）
/// ——脱离/分离在原文即严格不等，相切在这两处应判「未脱离/未分离」，现状已符合。
#[inline]
fn overlaps(unit: &Unit, zd: f64, zg: f64) -> bool {
    unit.low <= zg && unit.high >= zd
}

/// 步骤 a)：从级别 k 的单元序列识别全部中枢。
///
/// 算法（顺扫 + 跳过连接段，设计文档 §1.3 + c 段裁决 §5）：
/// 1. 在位置 i 用**前三段** `units[i..=i+2]` 定核心区间 `[ZD, ZG]`（第18课公式）；
/// 2. 若 `ZG > ZD` → 中枢成立（**严格**：`ZG == ZD` 单点核心不成立，#321 裁定 /
///    #323 三实现统一从严；与步骤 3 的弱延伸口径并存是裁定结果，非遗漏）；
/// 3. 向后延伸：后续单元只要与核心区间重叠（**闭区间**，相切算重叠 ⟹ 延伸，#290 裁定 A /
///    #322，见 [`overlaps`]）就纳入该中枢，终结于第一个不再重叠的单元 `last`；
/// 4. **跳过突破段** `units[last+1]`（= 离开/连接段），从 `i = last+2` 扫描下一中枢。
///
/// 返回的 `Zhongshu.units` 索引相对于传入的 `units` 切片。
///
/// **为何跳过突破段**（c 段裁决 `docs/c_segment_attribution_reasoning.md` §5，编排者
/// 2026-06-20 指示）：旧实现 `i = last+1` 把"离开前一中枢的连接段"吸收为下一中枢首单元，
/// 使 (a) 相邻中枢无间隙 → 走势离开段 c 恒空（趋势背驰几乎从不完美，79% 缺失）；
/// (b) 连接段拖低下一中枢外缘 `DD` → 同向延续被 `same_direction_step` 误判为盘整延伸
/// （实证4：BTC 牛市被折叠成一个大盘整）。`i = last+2` 让突破段成为中枢间间隙：c 段恢复
/// 至 100%、外缘 `DD` 不再被污染（同向延续在外缘判据下正确合并）。第18课定理一原文：
/// 「连接两个同级别中枢的必然是次级别以下级别的走势类型」——连接段不属任一中枢。
///
/// L2 实证（8 标的 structural，c 段 21%→100%、type1 250→1927/塔）：清仓频率是 regime
/// 函数——震荡标的改善（CL −16%→+120% 反超 BH），最强牛市退化（BTC +1155%→+68% 踏空，
/// ES −48pp）。结构正确独立于交易有效域（裁决 §8，formalization-validity-domain）。
pub fn find_centers(units: &[Unit], level: usize) -> Vec<Zhongshu> {
    let mut centers = Vec::new();
    let n = units.len();
    if n < 3 {
        return centers;
    }

    let mut i = 0usize;
    while i + 2 < n {
        // 核心区间由前三段定（第18课公式 ZD=max(d1,d2,d3)/ZG=min(g1,g2,g3)，v3 口径）。
        let zd = units[i].low.max(units[i + 1].low).max(units[i + 2].low);
        let zg = units[i].high.min(units[i + 1].high).min(units[i + 2].high);

        if zg > zd {
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
            // 跳过突破段（= 离开段/连接段，第18课定理一：连接两中枢的次级别以下走势，
            // 不属任一中枢）→ 它成为中枢间的间隙 = 本走势 c 段；下一中枢从其后扫描，
            // 故不污染下一中枢外缘 DD（同向延续在外缘判据下正确合并）。
            i = last + 2;
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

    /// #322 裁定 A（相切=重叠=延伸）：上沿相切段应被吸收进中枢。
    #[test]
    fn 上沿相切段延伸中枢() {
        // 前三段核心 ZD=12, ZG=20；第四段 low == ZG == 20（仅端点相接）。
        // 旧严格口径：`low < zg` → 20 < 20 假 → 不延伸（中枢在此终结）。
        // 新弱口径（定理一 020:56 脱离用严格不等）：相切算重叠 → 延伸吸收。
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(20.0, 30.0, 3, 4, Direction::Up),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 1);
        assert_eq!(centers[0].units, vec![0, 1, 2, 3]);
        assert_eq!(centers[0].gg, 30.0); // 外缘随吸收段抬高
    }

    /// #322 裁定 A：下沿相切段同样延伸（对称面）。
    #[test]
    fn 下沿相切段延伸中枢() {
        // 前三段核心 ZD=12, ZG=20；第四段 high == ZD == 12。
        // 旧严格口径：`high > zd` → 12 > 12 假 → 不延伸。新口径：延伸。
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(2.0, 12.0, 3, 4, Direction::Down),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 1);
        assert_eq!(centers[0].units, vec![0, 1, 2, 3]);
        assert_eq!(centers[0].dd, 2.0); // 外缘随吸收段压低
    }

    /// #322：脱离仍为严格口径（定理一「dn>ZG 或 gn<ZD」）——真脱离段不被吸收。
    #[test]
    fn 真脱离段不延伸中枢() {
        // 核心 ZD=12, ZG=20；第四段 low=20.1 > ZG → 脱离，中枢终结。
        let units = vec![
            bi(10.0, 20.0, 0, 1, Direction::Up),
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(20.1, 30.0, 3, 4, Direction::Up),
        ];
        let centers = find_centers(&units, 1);
        assert_eq!(centers.len(), 1);
        assert_eq!(centers[0].units, vec![0, 1, 2]);
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

    // ================================================================
    // #322 靶向验证探针（切片对照，**不重放全塔**）
    // ================================================================

    /// 旧严格口径的 `find_centers` 复刻（仅探针用；与生产版唯一差别 = 延伸谓词严格）。
    ///
    /// 保留在此是为了让「改弱的实际影响」可随时复算，而不是靠推测——口径若再变，
    /// 本函数与生产版的 diff 就是变更本体。
    fn find_centers_strict_legacy(units: &[Unit], level: usize) -> Vec<Zhongshu> {
        #[inline]
        fn overlaps_strict(unit: &Unit, zd: f64, zg: f64) -> bool {
            unit.low < zg && unit.high > zd
        }
        let mut centers = Vec::new();
        let n = units.len();
        if n < 3 {
            return centers;
        }
        let mut i = 0usize;
        while i + 2 < n {
            let zd = units[i].low.max(units[i + 1].low).max(units[i + 2].low);
            let zg = units[i].high.min(units[i + 1].high).min(units[i + 2].high);
            if zg > zd {
                let mut last = i + 2;
                while last + 1 < n && overlaps_strict(&units[last + 1], zd, zg) {
                    last += 1;
                }
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
                centers.push(Zhongshu { high: zg, low: zd, gg, dd, units: idxs, level });
                i = last + 2;
            } else {
                i += 1;
            }
        }
        centers
    }

    /// **靶向验证（#322）**：真实数据上逐级别对照新旧延伸口径，量化相切的实际影响。
    ///
    /// 方法（切片、非重放）：跑一次生产 `iterate` 得到全塔，取每级别的**实际输入单元序列**
    /// （level 0 = a₀，level k = level k−1 的 `next_units`），在同一序列上分别跑弱口径
    /// （生产）与严格口径（`find_centers_strict_legacy`）的 `find_centers`，比较中枢数与
    /// 逐中枢字段。不做双塔重放（上层输入本身会随口径漂移，那是全量重跑的范畴）。
    ///
    /// 跑法：`BT_SYMBOLS=OKLO cargo test --release recursive_t::center::tests::靶向_相切延伸口径逐级对照 -- --ignored --nocapture`
    #[test]
    #[ignore = "靶向验证(#322): 需 analysis/data_cache/*.json；BT_SYMBOLS 选标的"]
    fn 靶向_相切延伸口径逐级对照() {
        use crate::orchestrator::RecursiveOrchestrator;
        use crate::recursive_t::backtest::build_a0_from_segments;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use crate::recursive_t::{iterate, types::PerfectionMode};
        use std::path::PathBuf;

        let sym = std::env::var("BT_SYMBOLS").unwrap_or_else(|_| "OKLO".into());
        let file = SYMBOLS
            .iter()
            .find(|(s, _)| s.eq_ignore_ascii_case(&sym))
            .map(|(_, f)| *f)
            .unwrap_or_else(|| panic!("未知标的 {sym}"));
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache")
            .join(file);
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();
        eprintln!("[{sym}] bars={n} 跑 orchestrator…");
        let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
        for i in 0..n {
            orch.process_bar(o[i], h[i], l[i], c[i]);
        }
        let a0 = build_a0_from_segments(orch.segments(), orch.merged_to_raw(), &c);
        eprintln!("[{sym}] a0={} 单元，iterate(structural)…", a0.len());
        let tree = iterate(a0.clone(), PerfectionMode::Structural);

        // 逐级别输入序列：level 0 = a0；level k = level k-1 的 next_units。
        let mut inputs: Vec<Vec<Unit>> = vec![a0];
        for lv in tree.levels.iter() {
            inputs.push(lv.next_units.clone());
        }

        let mut tot_tangent = 0usize;
        let mut tot_diff_centers = 0usize;
        for (k, units) in inputs.iter().enumerate() {
            if units.len() < 3 {
                continue;
            }
            let weak = find_centers(units, k);
            let strict = find_centers_strict_legacy(units, k);

            // 相切命中：弱口径判重叠、严格口径判不重叠的**单元级**判定次数。
            let mut tangent = 0usize;
            for cen in weak.iter() {
                let (zd, zg) = (cen.low, cen.high);
                for &j in cen.units.iter().skip(3) {
                    let u = &units[j];
                    if !(u.low < zg && u.high > zd) {
                        tangent += 1;
                    }
                }
            }
            let diff = weak.len() != strict.len()
                || weak.iter().zip(strict.iter()).any(|(a, b)| {
                    a.low != b.low || a.high != b.high || a.dd != b.dd || a.gg != b.gg
                        || a.units != b.units
                });
            tot_tangent += tangent;
            if diff {
                tot_diff_centers += 1;
            }
            eprintln!(
                "  L{k}: units={:5} 中枢 弱={:4} 严={:4} 相切吸收判定={tangent} 集合差异={diff}",
                units.len(),
                weak.len(),
                strict.len()
            );
        }
        eprintln!(
            "[{sym}] 合计：相切吸收判定={tot_tangent} 次，级别集合差异={tot_diff_centers} 层"
        );
        assert!(!tree.levels.is_empty(), "至少涌现一层");
    }
}
