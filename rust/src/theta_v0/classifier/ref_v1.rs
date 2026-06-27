//! reference v1 引擎中枢语义（`~/Downloads/newchanlun-engine-formal-audit/reference_chanlun.py`
//! 逐字直译）—— 第Ⅱ类异源 canonical「可执行 reference 语义」的 rust 实装。
//!
//! ## 契约锚（Lean `Origin.CenterConstruct` § 5.5 + reference_chanlun.py，bit-exact）
//!
//! - **核心区间（frozen v1，全三段）** ↔ `Origin.CenterConstruct.refV1Interval`
//!   / `reference_chanlun.py:119-120`：`zd = max3(c1.low,c2.low,c3.low)`、
//!   `zg = min3(c1.high,c2.high,c3.high)`。**注意**：这与 `center.rs` 的 chan99 §6.4 派生
//!   口径 `compute_zd/compute_zg`（**前两段** `max(d₁,d₂)/min(g₁,g₂)`）是**不同口径**——
//!   两口径在第三段贯穿核心时重合，第三段更窄时分叉（见谱系 中枢核心区间口径分离）。
//! - **弱接触延伸** ↔ `Origin.CenterConstruct.weakOverlap` / `reference:78-85`：
//!   `component.high >= zd && component.low <= zg`（边界相切算延伸，frozen 测试口径）。
//! - **生命周期字段（gg/dd/settled/break）** ↔ `Origin.CenterConstruct.RefZhongshu` /
//!   `reference:40-52`：外缘 gg/dd 随延伸聚合；`settled = j < n`（末中枢强制 unsettled）；
//!   break_index/break_direction。
//! - **结算后重叠回退** ↔ `reference:150-151`：`i = max(j-2, end)`。
//!
//! ## v0/v1 反例裁决（已 native_decide 裁 v0 错，`legacy_v0_not_correct_for_v1_reference`）
//!
//! fixture `s0=[10,20],s1=[12,15],s2=[11,18]` 上 v1=[12,15]、legacy_v0=[11,18]
//! （v0 用首尾段 `max(l0,l2)/min(h0,h2)`）。本模块实装 **v1 正确语义**（全三段）。
//!
//! ## 认识论（formalization-validity-domain 231号）
//!
//! 全部 **L0**：纯整数 max3/min3 + 滑窗 + 弱接触延伸，不依赖经验数据。bit-exact 等级 = L0
//! （rust 实装忠实于 Lean 形式化 / reference Python，非 L2 行情有效断言）。

use super::super::types::{Segment, Tick};

/// reference `RefZhongshu`（`reference_chanlun.py:40-52` 逐字镜像）—— v1 中枢生命周期。
///
/// 契约锚 `Origin.CenterConstruct.RefZhongshu`（CenterConstruct.lean § 5.5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefZhongshu {
    /// 固定核心下沿 ZD（全三段 `max3(lows)`，frozen v1）。
    pub zd: Tick,
    /// 固定核心上沿 ZG（全三段 `min3(highs)`，frozen v1）。
    pub zg: Tick,
    /// 构成段起始下标（completed 序列内）。
    pub start: usize,
    /// 构成段终止下标（随弱接触延伸增长）。
    pub end: usize,
    /// 构成段数 `end - start + 1`。
    pub count: usize,
    /// 是否已结算（`j < n`——延伸后仍有段离开核心则结算）。
    pub settled: bool,
    /// 结算时离开核心的段下标（未结算 = -1）。
    pub break_index: i64,
    /// 离开方向 true=up / false=down（未结算 = true，对齐 reference 默认 ""→防御 up）。
    pub break_up: bool,
    /// 外缘上界 GG（初始三段 + 延伸段聚合 max）。
    pub gg: Tick,
    /// 外缘下界 DD（初始三段 + 延伸段聚合 min）。
    pub dd: Tick,
}

/// 三元 max（reference `max(a,b,c)`，全三段核心 ZD）。契约锚 `Origin.CenterConstruct.max3`。
fn max3(a: Tick, b: Tick, c: Tick) -> Tick {
    a.max(b).max(c)
}

/// 三元 min（reference `min(a,b,c)`，全三段核心 ZG）。契约锚 `Origin.CenterConstruct.min3`。
fn min3(a: Tick, b: Tick, c: Tick) -> Tick {
    a.min(b).min(c)
}

/// reference 弱接触延伸判据（`reference_chanlun.py:78-85` 逐字）。
///
/// `component.high >= zd && component.low <= zg`（弱相交，边界相切算延伸）。
/// 契约锚 `Origin.CenterConstruct.weakOverlap`。
fn weak_overlap(comp_high: Tick, comp_low: Tick, zd: Tick, zg: Tick) -> bool {
    comp_high >= zd && comp_low <= zg
}

/// reference 离开方向（`reference_chanlun.py:88-93` 逐字）—— true=up。
///
/// `low > zg → up`；`high < zd → down`；否则 `high > zg ? up : down`（防御分支）。
/// 契约锚 `Origin.CenterConstruct.breakDirectionUp`。
fn break_direction_up(comp_high: Tick, comp_low: Tick, zd: Tick, zg: Tick) -> bool {
    if comp_low > zg {
        true
    } else if comp_high < zd {
        false
    } else {
        comp_high > zg
    }
}

/// 段 → (high, low) 投影（reference ComponentLike）。
///
/// 向上段 high=end_price/low=start_price；向下段 high=start_price/low=end_price
/// （对齐 `center.rs` segment_to_unit / `CenterConstruction.segHigh/segLow`）。
fn seg_hl(s: &Segment) -> (Tick, Tick) {
    if s.start_price >= s.end_price {
        (s.start_price, s.end_price)
    } else {
        (s.end_price, s.start_price)
    }
}

/// **★reference v1 中枢构造（`zhongshus_from_components_ref`，`reference_chanlun.py:96-155` 逐字）**
/// —— 完成态段序列 → v1 中枢序列。
///
/// 契约锚 `Origin.CenterConstruct.refZhongshusFromComponents`（CenterConstruct.lean § 5.5，bit-exact）。
///
/// 输入 `segments` 已是完成态段（`completed_mask` 过滤在调用点，reference:158-160）。
/// `n < 3 ⟹ []`。否则滑窗：前三段定全三段核心；核心空（`zg<=zd`）滑窗前进1；否则弱接触延伸 +
/// 生命周期字段，结算后重叠回退 `i=max(j-2,end)`，末中枢强制 unsettled 后停止。
///
/// 边界条件（结论翻转）：核心口径用全三段 max3/min3（非前两段——见模块级口径分离说明）；
/// 延伸用弱相交（边界相切算延伸）；`settled` 由 `j<n` 决定（末中枢必 unsettled）。
pub fn ref_zhongshus_from_components(segments: &[Segment]) -> Vec<RefZhongshu> {
    let comps: Vec<(Tick, Tick)> = segments.iter().map(seg_hl).collect();
    let n = comps.len();
    if n < 3 {
        return Vec::new();
    }

    let mut result: Vec<RefZhongshu> = Vec::new();
    let mut i = 0usize;
    while i + 2 < n {
        let (h1, l1) = comps[i];
        let (h2, l2) = comps[i + 1];
        let (h3, l3) = comps[i + 2];
        let zd = max3(l1, l2, l3);
        let zg = min3(h1, h2, h3);

        if zg <= zd {
            // 核心空：滑窗前进 1（reference:122-124）。
            i += 1;
            continue;
        }

        let mut gg = max3(h1, h2, h3);
        let mut dd = min3(l1, l2, l3);
        let mut end = i + 2;
        // 弱接触延伸（reference:129-134）：从 j=i+3 起逐段吸收。
        let mut j = i + 3;
        while j < n && weak_overlap(comps[j].0, comps[j].1, zd, zg) {
            end = j;
            gg = gg.max(comps[j].0);
            dd = dd.min(comps[j].1);
            j += 1;
        }

        let settled = j < n;
        let (break_index, break_up) = if settled {
            (j as i64, break_direction_up(comps[j].0, comps[j].1, zd, zg))
        } else {
            (-1, true)
        };
        result.push(RefZhongshu {
            zd,
            zg,
            start: i,
            end,
            count: end - i + 1,
            settled,
            break_index,
            break_up,
            gg,
            dd,
        });

        if settled {
            // 重叠回退（reference:150-151）：i = max(j-2, end)。
            i = (j.saturating_sub(2)).max(end);
        } else {
            // 末中枢未结算：停止（reference:152-153 break）。
            break;
        }
    }

    result
}

/// reference v1 核心区间（全三段，`reference:119-120`）—— 契约锚 `Origin.CenterConstruct.refV1Interval`。
pub fn ref_v1_interval(
    l0: Tick,
    h0: Tick,
    l1: Tick,
    h1: Tick,
    l2: Tick,
    h2: Tick,
) -> (Tick, Tick) {
    (max3(l0, l1, l2), min3(h0, h1, h2))
}

/// legacy v0 核心区间（首尾段，`NewChanlunEngineAudit.lean v0Interval`）—— 已裁决错。
///
/// 契约锚 `Origin.CenterConstruct.legacyV0Interval`。`(max(l0,l2), min(h0,h2))`。
pub fn legacy_v0_interval(
    l0: Tick,
    h0: Tick,
    _l1: Tick,
    _h1: Tick,
    l2: Tick,
    h2: Tick,
) -> (Tick, Tick) {
    (l0.max(l2), h0.min(h2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Direction;

    /// 构造段（high/low 由 start/end price 编码：up 段 end>start，down 段 start>end）。
    fn seg(si: usize, ei: usize, dir: Direction, sp: Tick, ep: Tick) -> Segment {
        Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    // reference fixture 三段（v1 反例 s0=[10,20],s1=[12,15],s2=[11,18]）。
    fn ref_seg0() -> Segment {
        seg(0, 1, Direction::Up, 10, 20)
    } // [low=10, high=20]
    fn ref_seg1() -> Segment {
        seg(1, 2, Direction::Down, 15, 12)
    } // [low=12, high=15]
    fn ref_seg2() -> Segment {
        seg(2, 3, Direction::Up, 11, 18)
    } // [low=11, high=18]

    // ──────────────────────────────────────────────────────────────────────
    //  v0/v1 反例裁决（对齐 Origin.CenterConstruct + NewChanlunEngineAudit.lean）
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn v1_interval_fixture_value() {
        // 全三段：zd=max(10,12,11)=12, zg=min(20,15,18)=15。
        assert_eq!(ref_v1_interval(10, 20, 12, 15, 11, 18), (12, 15));
    }

    #[test]
    fn v0_interval_fixture_value() {
        // 首尾段：zd=max(10,11)=11, zg=min(20,18)=18。
        assert_eq!(legacy_v0_interval(10, 20, 12, 15, 11, 18), (11, 18));
    }

    #[test]
    fn v0_ne_v1_fixture() {
        // 同一三段两口径核心区间不同（裁决 v0 错的反例）。
        assert_ne!(
            legacy_v0_interval(10, 20, 12, 15, 11, 18),
            ref_v1_interval(10, 20, 12, 15, 11, 18)
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  v1 中枢构造 witness（bit-exact 对齐 Origin.CenterConstruct witness_ref_*）
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn ref_zhongshu_one_unsettled() {
        // 三段 ⟹ 恰一个中枢，无后续段 ⟹ jOut=3=n ⟹ settled=false（末中枢强制 unsettled）。
        let zs = ref_zhongshus_from_components(&[ref_seg0(), ref_seg1(), ref_seg2()]);
        assert_eq!(zs.len(), 1);
        assert_eq!(zs[0].zd, 12);
        assert_eq!(zs[0].zg, 15);
        assert_eq!(zs[0].gg, 20); // max3(20,15,18)
        assert_eq!(zs[0].dd, 10); // min3(10,12,11)
        assert!(!zs[0].settled);
        assert_eq!(zs[0].break_index, -1);
    }

    #[test]
    fn ref_under_three_empty() {
        // n<3 ⟹ []（reference:112-113）。
        assert!(ref_zhongshus_from_components(&[ref_seg0(), ref_seg1()]).is_empty());
    }

    #[test]
    fn ref_weak_extends_to_four() {
        // 第四段 [13,16] 弱接触核心 [12,15]（16>=12 ∧ 13<=15）⟹ end 延伸，count=4，末 unsettled。
        let seg3 = seg(3, 4, Direction::Down, 16, 13); // [low=13, high=16]
        let zs = ref_zhongshus_from_components(&[ref_seg0(), ref_seg1(), ref_seg2(), seg3]);
        assert_eq!(zs.len(), 1);
        assert_eq!(zs[0].count, 4);
        assert_eq!(zs[0].end, 3);
        assert!(!zs[0].settled);
    }

    #[test]
    fn ref_settled_break_up() {
        // 第四段延伸（count→4），第五段 [22,25] low=22>zg=15 离开 ⟹ jOut=4<n=5 settled，break up。
        let seg3 = seg(3, 4, Direction::Down, 16, 13);
        let seg4 = seg(4, 5, Direction::Up, 22, 25); // [low=22, high=25]
        let zs =
            ref_zhongshus_from_components(&[ref_seg0(), ref_seg1(), ref_seg2(), seg3, seg4]);
        assert!(!zs.is_empty());
        assert!(zs[0].settled);
        assert!(zs[0].break_up);
        assert_eq!(zs[0].break_index, 4);
    }

    #[test]
    fn ref_core_empty_slides() {
        // 前三段无公共三段重叠（核心空）⟹ 滑窗前进，找不到中枢。
        // s0=[0,4], s1=[10,14], s2=[20,24]：zd=max(0,10,20)=20 > zg=min(4,14,24)=4 ⟹ 空。
        let a = seg(0, 1, Direction::Up, 0, 4);
        let b = seg(1, 2, Direction::Down, 14, 10);
        let c = seg(2, 3, Direction::Up, 20, 24);
        assert!(ref_zhongshus_from_components(&[a, b, c]).is_empty());
    }
}
