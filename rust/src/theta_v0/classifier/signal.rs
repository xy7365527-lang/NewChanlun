//! 信号端点提取（reference-theta-v0.md:34-36）——confirmed 走势结构 → 买卖点。
//!
//! ## 契约重锚（legacy Formal/BSPLabels + Claim9 → `Origin.BspClassification` + `Origin.CenterStates`）
//!
//! 把 confirmed 中枢 + 线段序列提取为买卖点端点（`BspPoint`）。每个端点的语义状态（对齐
//! `Origin.BspClassification.BspEndpoint` 字段）由走势结构相对中枢的位置（`Origin.CenterStates.
//! classifyPosition`）bit-exact 计算（非臆造）。
//!
//! ## 诚实范围（formalization-validity-domain，★关键边界标注）
//!
//! 本工位 v0 **只提取第三类买卖点**——它是 confirmed 结构上**严格可判定**的买卖点：
//! reference:36「上离中枢后次级别回试低点 `≥ZG`=3买；下离后回抽高点 `≤ZD`=3卖」是
//! **点相对中枢核心区间的位置判据**（`Origin.BspClassification.IsType3Buy/IsType3Sell` +
//! `Origin.CenterStates.classifyPosition`，已 bit-exact 形式化），只需 confirmed 中枢
//! + 后续线段端点价，零经验时序依赖。
//!
//! 第一/二类买卖点**未在此提取**，原因是诚实的结构边界（非补丁/未实装）：
//! - 1 类需**趋势末段背驰确认**（趋势 ≥2 同向中枢 + 末段相对前同向段 MACD 面积严格变小，
//!   reference:34）——背驰判据 `divergence::segments_diverge` 已实装，但「定位趋势的末段
//!   与前同向段的 bar 区间」需完整趋势确认时序（多级别走势的段边界），超出当前**单中枢
//!   局部**可严格判定的范围。
//! - 2 类需 1 类后的回调时序状态（reference:35），依赖 1 类已确认。
//!
//! ⟹ v0 提取第三类（端到端打通 L2：classify 返回非空买卖点）。1/2 类在趋势确认时序工位
//! 就绪后零改本判据扩展（endpoint_to_bsp 已支持全三类 bit）。这是**增量覆盖**——第三类
//! 是 confirmed 结构正确产出，不是错误逻辑加垫片。

use super::super::types::{Center, Direction, Segment, Tick};
use super::bsp::{endpoint_to_bsp, EndpointSituation};
use super::super::types::BspBits;

/// 买卖点条目（带结构止损价，single source，见 `bsp::BspPoint`）。
pub use super::bsp::BspPoint;

/// 线段端点投影（信号候选点：每条线段的终止端点 = 一个潜在买卖点）。
struct SegEnd {
    source_index: usize,
    /// 端点价（线段终止价 = 该端点的极值）。
    price: Tick,
    /// 该线段方向（向上线段端点 = 高点候选，向下 = 低点候选）。
    dir: Direction,
}

fn seg_end(s: &Segment) -> SegEnd {
    SegEnd {
        source_index: s.end_index,
        price: s.end_price,
        dir: s.direction,
    }
}

/// 第三类买卖点提取（契约锚 `Origin.BspClassification.IsType3Buy/IsType3Sell` 点位判据）。
///
/// 对每个 confirmed 中枢 `c`，扫描其后的线段端点序列，按 reference:36 判第三类：
/// - **3买**：一条**向上线段**离开中枢上方（端点 `> zg`），紧随的**向下线段**回试低点
///   `>= zg`（不重新跌破进入中枢）⟹ 该回试低点端点 = 3 买。
/// - **3卖**：一条**向下线段**离开中枢下方（端点 `< zd`），紧随的**向上线段**回抽高点
///   `<= zd`（不重新升破进入中枢）⟹ 该回抽高点端点 = 3 卖。
///
/// `segs_after` 是中枢 `end_index` 之后的线段序列（按时间序）。逐相邻对 (leave, retest)
/// 判定。bit-exact：边界用 reference:36 等号口径（`>= zg` / `<= zd` 含等号）。
fn extract_third_for_center(c: &Center, segs_after: &[Segment]) -> Vec<BspPoint> {
    let mut points = Vec::new();
    // 逐相邻线段对：前者离开中枢，后者回试。
    for pair in segs_after.windows(2) {
        let leave = seg_end(&pair[0]);
        let retest = seg_end(&pair[1]);
        match (leave.dir, retest.dir) {
            // 3 买：向上离开（leave 端点 > zg）+ 向下回试（retest 低点 >= zg，不入中枢）。
            (Direction::Up, Direction::Down) => {
                if leave.price > c.zg && retest.price >= c.zg {
                    let situ = EndpointSituation {
                        after_first_buy: false,
                        is_pullback_end: false,
                        left_center: true,        // 离开中枢（leave.price > zg）
                        retrace_not_reenter: true, // 回试不入（retest >= zg）
                        below_last_center: false,
                        is_sell_side: false,
                    };
                    let bits = endpoint_to_bsp(&situ);
                    points.push(make_point(retest.source_index, bits, retest.price, c));
                }
            }
            // 3 卖：向下离开（leave 端点 < zd）+ 向上回抽（retest 高点 <= zd，不入中枢）。
            (Direction::Down, Direction::Up) => {
                if leave.price < c.zd && retest.price <= c.zd {
                    let situ = EndpointSituation {
                        after_first_buy: false,
                        is_pullback_end: false,
                        left_center: true,
                        retrace_not_reenter: true,
                        below_last_center: false,
                        is_sell_side: true,
                    };
                    let bits = endpoint_to_bsp(&situ);
                    points.push(make_point(retest.source_index, bits, retest.price, c));
                }
            }
            // 同向相邻（无回试）/其他：非第三类结构，跳过。
            _ => {}
        }
    }
    points
}

/// 构造 BspPoint（结构止损价 single source）。
///
/// 3 类止损取 `center.zg`(买)/`zd`(卖)，故 center 必 `Some`（不变量：含 3 类 bit ⟹ center
/// 有值）。pivot_low/pivot_high 取回试端点价（买点回试低点 = pivot_low，卖点 = pivot_high）。
fn make_point(source_index: usize, bits: BspBits, retest_price: Tick, c: &Center) -> BspPoint {
    BspPoint {
        source_index,
        bits,
        // 买点回试低点 → pivot_low；卖点回抽高点 → pivot_high。按 bit 方向填，另一侧 0。
        pivot_low: if bits.buy3 { retest_price } else { 0 },
        pivot_high: if bits.sell3 { retest_price } else { 0 },
        center: Some(*c),
    }
}

/// 从 confirmed 中枢序列 + 线段序列提取该级别全部买卖点（reference:34-36）。
///
/// 对每个中枢，取其 `end_index` 之后的线段子序列，提取第三类买卖点。买卖点按 source_index
/// 升序返回（reference:16 平局裁决——已确认结构不回写，时间序天然升序）。
///
/// ★诚实范围（见模块头）：v0 只提取第三类（confirmed 结构严格可判定）。
pub fn extract_signals(centers: &[Center], segments: &[Segment]) -> Vec<BspPoint> {
    let mut points = Vec::new();
    for c in centers {
        // 中枢之后的线段（end_index 严格大于中枢 end_index 的线段——离开+回试发生在中枢后）。
        let segs_after: Vec<Segment> = segments
            .iter()
            .filter(|s| s.start_index >= c.end_index)
            .copied()
            .collect();
        points.extend(extract_third_for_center(c, &segs_after));
    }
    // 按 source_index 升序（reference:16 平局裁决键的时间序分量）。
    points.sort_by_key(|p| p.source_index);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(zd: Tick, zg: Tick, end_index: usize) -> Center {
        Center { zd, zg, dd: zd - 5, gg: zg + 5, start_index: 0, end_index }
    }

    fn seg(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    #[test]
    fn third_buy_extracted_bit_exact() {
        // 中枢核心 [100,200] end_index=12。后续：向上线段离开（端点 250 > zg=200），
        // 向下回试低点 210 >= zg=200（不入中枢）⟹ 3 买。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),   // 离开：端点 250 > 200
            seg(Direction::Down, 16, 20, 250, 210), // 回试：低点 210 >= 200 → 3 买
        ];
        let points = extract_signals(&[c], &segs);
        assert_eq!(points.len(), 1);
        assert!(points[0].bits.buy3);
        assert_eq!(points[0].source_index, 20); // 回试端点
        assert_eq!(points[0].pivot_low, 210);   // 回试低点 = pivot_low
        assert_eq!(points[0].center.map(|c| c.zg), Some(200)); // 3 买止损 = zg
    }

    #[test]
    fn third_buy_rejected_when_retest_reenters() {
        // 回试低点 190 < zg=200（重新跌破进入中枢）⟹ 不是 3 买。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 190), // 回试 190 < 200 → 入中枢，非 3 买
        ];
        let points = extract_signals(&[c], &segs);
        assert!(points.is_empty());
    }

    #[test]
    fn third_buy_boundary_retest_eq_zg_is_third() {
        // 回试低点 = zg=200（reference:36 「>=ZG」等号允许）⟹ 3 买成立。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 200), // 回试 = zg → 3 买（等号允许）
        ];
        let points = extract_signals(&[c], &segs);
        assert_eq!(points.len(), 1);
        assert!(points[0].bits.buy3);
    }

    #[test]
    fn third_sell_extracted_bit_exact() {
        // 镜像：向下离开（端点 50 < zd=100），向上回抽高点 90 <= zd=100 ⟹ 3 卖。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Down, 12, 16, 150, 50),  // 离开：端点 50 < 100
            seg(Direction::Up, 16, 20, 50, 90),     // 回抽：高点 90 <= 100 → 3 卖
        ];
        let points = extract_signals(&[c], &segs);
        assert_eq!(points.len(), 1);
        assert!(points[0].bits.sell3);
        assert_eq!(points[0].pivot_high, 90);   // 回抽高点 = pivot_high
        assert_eq!(points[0].center.map(|c| c.zd), Some(100)); // 3 卖止损 = zd
    }

    #[test]
    fn no_leave_no_signal() {
        // 线段未离开中枢（端点 180 < zg=200）⟹ 无第三类。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 180),   // 未离开（180 < 200）
            seg(Direction::Down, 16, 20, 180, 160),
        ];
        let points = extract_signals(&[c], &segs);
        assert!(points.is_empty());
    }

    #[test]
    fn third_buy_has_center_invariant() {
        // 不变量：含 3 类 bit ⟹ center 必 Some（strategy 3 类止损 unwrap 安全）。
        let c = center(100, 200, 12);
        let segs = vec![
            seg(Direction::Up, 12, 16, 150, 250),
            seg(Direction::Down, 16, 20, 250, 210),
        ];
        let points = extract_signals(&[c], &segs);
        for p in &points {
            if p.bits.buy3 || p.bits.sell3 {
                assert!(p.center.is_some(), "含 3 类 bit ⟹ center 必 Some");
            }
        }
    }

    #[test]
    fn empty_centers_no_signal() {
        let points = extract_signals(&[], &[seg(Direction::Up, 0, 4, 0, 10)]);
        assert!(points.is_empty());
    }
}
