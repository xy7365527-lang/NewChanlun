//! SecondKind 第二特征序列动态确认（reference-theta-v0.md:22 第二种情况，第67课）。
//!
//! ## 认识论等级（formalization-validity-domain，Lead 裁定 #84 点2 强制标注）
//!
//! **本文件 = L1（对 frozen 定义忠实 + 对 Python 参考交叉验证），非对 Lean bit-exact。**
//!
//! 关键诚实标注：SecondKind 第二特征序列**动态确认状态机**在 Origin canonical **无 spec**——
//! Origin `SegmentConstruction`/`SegmentFeatureComplete` 形式化了静态层，但动态划分状态机不在其
//! 形式化范围（设计选择，留给参考实现 `a_segment_v1.py`，非遗漏——与 legacy
//! Phase2/Claim10_SegmentV1:334-346 同样的有意非形式化边界）。故本文件**不**对 Origin Lean
//! bit-exact，只对：
//! - **frozen 定义**（reference-theta-v0.md:22 + 第67课博文原文）**忠实**；
//! - **Python `a_segment_v1.py`**（`_second_seq_has_fractal`，37 测试）**交叉验证**（同输入对比
//!   线段确认结果，不要求逐位等价内部中间态）。
//!
//! 对照：静态判据（`classify_termination`/特征序列抽取/包含处理）保持对
//! `Origin.SegmentFeatureSeq`/`SegmentFeatureComplete` 对齐（那是 Origin 已形式化的静态层）。
//! theta_v0「对 Origin canonical 对齐」的**有效域 = 静态判据**，动态确认部分超出该有效域（Lead #84 点2）。
//!
//! ## 第67课博文权威定义（动态确认语义，CLAUDE.md 三级权威链：博文 = 一级权威）
//!
//! 067-第67课.md 原文（第二种情况）：
//! > 特征序列的顶分型中，第一和第二元素间**存在缺口**，**如果从该分型最高点开始的向下一笔
//! > 开始的序列的特征序列出现底分型**，那么该线段在该顶分型的高点处结束；底分型镜像。
//! > 强调：第二个序列中的分型，**不分第一二种情况，只要有分型就可以**。
//!
//! ## 方向语义（第67课博文 + Python 交叉验证，Lead #84 点3 追溯依据）
//!
//! 向上线段（seg_dir=Up）顶分型有缺口 → 从分型最高点之后的笔序列构造**第二特征序列**。
//! 第二特征序列 = 新（反向=向下）段的特征序列 = 取**向上笔**（新向下段的反向笔 = seg_dir
//! 同向笔）。出现**任意分型**（顶或底，"只要有分型"）即确认原线段终结。
//!
//! ★这与 Python `_second_seq_has_fractal` 一致（取 `sk.direction == seg_dir` 同向笔构第二
//! 特征序列）——交叉验证锚点。方向语义以 67课博文为准（Lead #84 点3）：博文"向下一笔开始
//! 的序列的特征序列出现底分型"，该新向下段的特征序列取其反向（向上=seg_dir 同向）笔。
//!
//! ## 不复制 Python 性能启发式（Lead #84 点1 强制）
//!
//! Python 含 `TAIL_WINDOW=7` / `MAX_SECOND_SEQ_SCAN=50`（O(n²) 性能妥协，**非缠论可导**）。
//! 本文件**不复制**这些常数。扫描窗口若需作 config 字段 `parse.second_seq_scan_window`
//! （default 0 = 无限全扫，bit-exact 优先于性能），显式标 `[设计选择,默认值]`，不进缠论核心。

use super::super::types::{Direction, Stroke, Tick};
use super::feature_seq::second_seq_has_fractal;
use super::segment::{stroke_interval, Interval};

/// 在标准特征序列上检测是否存在**任意**分型（顶或底，第67课"只要有分型就可以"）。
///
/// 第二特征序列分型不分第一二种情况（第67课:46 + 博文"不分第一二种情况，只要有分型"），
/// 故顶分型 OR 底分型都算。bit-exact 对齐 Python `_has_any_fractal`（同极值条件）。
///
/// 顶分型：中元素 hi 严格高于左右 hi；底分型：中元素 lo 严格低于左右 lo（严格不等，第67课）。
///
/// 边界条件：标准特征序列 < 3 元素 ⟹ 无分型（无完整三元组）。
///
/// （BUG-08 修复后生产路径经 `feature_seq::second_seq_has_fractal` 的尾三元组提前终止判定，
/// 本函数仅测试断言保留——语义锚不变。）
#[cfg(test)]
fn has_any_feature_fractal(std_feat: &[Interval]) -> bool {
    if std_feat.len() < 3 {
        return false;
    }
    for i in 1..std_feat.len() - 1 {
        let (l, m, r) = (&std_feat[i - 1], &std_feat[i], &std_feat[i + 1]);
        let is_top = m.hi > l.hi && m.hi > r.hi;
        let is_bottom = m.lo < l.lo && m.lo < r.lo;
        if is_top || is_bottom {
            return true;
        }
    }
    false
}

/// 第二特征序列动态确认（reference-theta-v0.md:22 第二种情况，第67课博文）。
///
/// SecondKind（首特征序列分型有缺口）下，从分型极值点之后的笔序列构造**第二特征序列**，
/// 检测是否出现分型——出现即确认原线段终结。
///
/// 参数：
/// - `strokes`：从线段起点开始的笔序列（`strokes[0].direction` = 线段方向 seg_dir）。
/// - `apex_stroke_offset`：首特征序列分型极值点对应的笔在 `strokes` 中的偏移（分型最高/低点
///   所在的那根 seg_dir 反向笔——线段方向的反向笔承载分型极值）。
///
/// 算法（第67课博文逐字 + Python `_second_seq_has_fractal` 交叉验证）：
/// 1. 从 `apex_stroke_offset` 之后的笔中，取 **seg_dir 同向笔**（= 新反向段的特征序列元素，
///    第67课"从分型极值点开始的反向一笔开始的序列的特征序列"）。
/// 2. 对这些笔的区间做**方向性**包含处理 → 标准第二特征序列。
/// 3. 检测任意分型（第67课"只要有分型就可以"）。
///
/// ★BUG-08 修复：包含处理复用 `feature_seq::second_seq_has_fractal`（bit-exact 对齐 Python
/// `_apply_inclusion` 的方向性合并：向上取 max/max、向下取 min/min，初始 dir_state 同
/// Python `seg_dir==up ⟹ DOWN` 语义）。原实现复用 `segment::process_feature_inclusion`
/// 的中性外包络 `[min(lo),max(hi)]`——那是 Origin 静态层的配套实现，与本文件宣称的
/// Python bit-exact 交叉验证不是同一算法（`[10,20]` 含 `[12,18]` 向上应得 `[12,20]`，
/// 外包络仍得 `[10,20]`，会增删后续分型）。
///
/// 返回：第二特征序列出现分型 ⟹ `true`（确认终结）；否则 `false`（未确认，留 tail）。
///
/// ★诚实（Lead #84 点1）：默认全扫描（无 TAIL_WINDOW/MAX_SCAN 截断）——bit-exact 优先于
/// 性能。窗口截断若需作 config（不硬编码 Python 的 50）。
///
/// 边界条件：`apex_stroke_offset` 之后无 seg_dir 同向笔 / 不足成第二特征序列 ⟹ `false`。
pub fn second_kind_confirmed(strokes: &[Stroke], apex_stroke_offset: usize) -> bool {
    let Some(first) = strokes.first() else {
        return false;
    };
    let seg_dir = first.direction;
    // 第二特征序列 = apex 之后的 seg_dir 同向笔 + 方向性包含处理 + 任意分型检测——
    // 与 feature_seq::second_seq_has_fractal 完全同构（scan_window=0 = 无限全扫，
    // 对齐本模块"默认全扫描，bit-exact 优先"承诺）。
    second_seq_has_fractal(strokes, seg_dir, apex_stroke_offset, 0)
}

/// 在线段方向反向笔序列里定位首特征序列分型极值点的笔偏移（SecondKind 确认的起点锚）。
///
/// 首特征序列由 seg_dir **反向**笔构成。分型极值点（向上线段=顶分型最高点，向下=底分型最低点）
/// 落在某根反向笔上。返回该笔在 `strokes` 中的偏移——第二特征序列从此笔之后构造。
///
/// `apex`：首特征序列分型中心元素的区间（顶分型=hi 最高的元素，底分型=lo 最低的元素）。
/// 通过区间重叠定位贡献该极值元素的反向笔（与 segment.rs `analyze_termination` FirstKind
/// 定位段端笔同构）。
///
/// 边界条件：无匹配反向笔 ⟹ `None`（无法定位极值锚点，调用方按未确认处理）。
pub fn locate_apex_stroke(strokes: &[Stroke], apex: &Interval) -> Option<usize> {
    let first = strokes.first()?;
    let rev = first.direction.flip();
    strokes
        .iter()
        .position(|s| s.direction == rev && stroke_interval(s).overlaps(apex))
}

/// 第二特征序列动态确认结果（供 segment.rs 状态机消费）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondKindResult {
    /// 第二特征序列出现分型 → 原线段终结确认，段端 = 首分型极值笔偏移。
    Confirmed { end_offset: usize },
    /// 第二特征序列未出现分型 → 未确认（留 tail，不强断，不放松产假线段）。
    Pending,
}

/// SecondKind 动态确认全流程（segment.rs 状态机的 SecondKind 分支入口）。
///
/// 给定首特征序列分型中心元素 `apex`（顶/底分型中心），定位极值笔锚点 → 构造第二特征序列 →
/// 检测分型。出现 ⟹ `Confirmed{ end_offset }`（段端 = 极值笔偏移）；否则 `Pending`（留 tail）。
///
/// ★严格性（Lead #84 硬约束 + no-patch）：仅第二特征序列**实际出现分型**才 `Confirmed`。
/// 未出现 ⟹ `Pending`（古怪线段/笔破坏未发展成线段破坏，对齐 Origin.SegmentFeatureComplete 未确认态）——
/// 不为"多产线段"放松标准。
pub fn resolve_second_kind(strokes: &[Stroke], apex: &Interval) -> SecondKindResult {
    let Some(apex_offset) = locate_apex_stroke(strokes, apex) else {
        return SecondKindResult::Pending;
    };
    if second_kind_confirmed(strokes, apex_offset) {
        SecondKindResult::Confirmed { end_offset: apex_offset }
    } else {
        SecondKindResult::Pending
    }
}

/// 第二特征序列方向极值（向上线段确认看新向下段、向下线段看新向上段的极值）。
/// 供调用方读出确认段端价（保留供 segment.rs 端点计算复用，避免重复极值逻辑）。
pub fn apex_extreme(strokes: &[Stroke], apex_offset: usize, seg_dir: Direction) -> Option<Tick> {
    let s = strokes.get(apex_offset)?;
    let iv = stroke_interval(s);
    Some(match seg_dir {
        Direction::Up => iv.hi,   // 向上线段顶分型极值 = 高点
        Direction::Down => iv.lo, // 向下线段底分型极值 = 低点
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Stroke {
        Stroke { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    #[test]
    fn has_any_fractal_detects_top() {
        // 三元素中元素 hi 最高 → 顶分型。
        let feat = vec![
            Interval { lo: 5, hi: 10 },
            Interval { lo: 8, hi: 15 },
            Interval { lo: 6, hi: 12 },
        ];
        assert!(has_any_feature_fractal(&feat));
    }

    #[test]
    fn has_any_fractal_detects_bottom() {
        // 中元素 lo 最低 → 底分型（第二特征序列不分顶底，都算）。
        let feat = vec![
            Interval { lo: 8, hi: 20 },
            Interval { lo: 3, hi: 12 },
            Interval { lo: 7, hi: 15 },
        ];
        assert!(has_any_feature_fractal(&feat));
    }

    #[test]
    fn has_any_fractal_none_when_monotone() {
        // 单调递增 → 无分型。
        let feat = vec![
            Interval { lo: 1, hi: 5 },
            Interval { lo: 3, hi: 8 },
            Interval { lo: 6, hi: 12 },
        ];
        assert!(!has_any_feature_fractal(&feat));
    }

    #[test]
    fn has_any_fractal_fewer_than_three_none() {
        assert!(!has_any_feature_fractal(&[Interval { lo: 1, hi: 5 }, Interval { lo: 2, hi: 6 }]));
    }

    /// golden：向上线段 SecondKind，第二特征序列（向上笔）出现分型 → 确认。
    /// 67课博文：顶分型有缺口，从极值点后向上笔序列的特征序列出现分型则确认终结。
    #[test]
    fn golden_second_kind_confirmed_when_second_seq_has_fractal() {
        // 向上线段：strokes[0]=Up（seg_dir=Up）。apex_offset 后的向上笔构第二特征序列。
        // 第二特征序列三元素须**互不包含**（否则包含处理坍缩）+ 中间 hi 最高成顶分型。
        // 取 Up 笔区间 [5,12] / [10,20] / [8,16]：两两互不包含，中间 hi=20 最高 → 顶分型。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 25),    // seg_dir=Up
            stroke(Direction::Down, 4, 8, 25, 5),  // apex 反向笔（offset 1）
            stroke(Direction::Up, 8, 12, 5, 12),   // 第二特征序列元素1 [5,12]
            stroke(Direction::Down, 12, 16, 12, 4),
            stroke(Direction::Up, 16, 20, 10, 20), // 元素2 [10,20] hi=20 最高
            stroke(Direction::Down, 20, 24, 20, 6),
            stroke(Direction::Up, 24, 28, 8, 16),  // 元素3 [8,16] → 顶分型（中间最高）
        ];
        // apex_offset=1。第二特征序列 = offset>1 的向上笔 [5,12]/[10,20]/[8,16]（互不含）→ 顶分型。
        assert!(second_kind_confirmed(&strokes, 1));
    }

    /// golden：第二特征序列无分型（互不含 + 单调）→ 未确认（Pending，严格不强断）。
    #[test]
    fn golden_second_kind_pending_when_no_fractal() {
        // apex 后向上笔 [5,8]/[9,12]/[13,16]：互不包含（不坍缩）+ 单调递增 → 无分型 → 未确认。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 20),
            stroke(Direction::Down, 4, 8, 20, 5),
            stroke(Direction::Up, 8, 12, 5, 8),    // [5,8]
            stroke(Direction::Down, 12, 16, 8, 4),
            stroke(Direction::Up, 16, 20, 9, 12),  // [9,12]
            stroke(Direction::Down, 20, 24, 12, 6),
            stroke(Direction::Up, 24, 28, 13, 16), // [13,16] 单调递增（互不含）→ 无分型
        ];
        assert!(!second_kind_confirmed(&strokes, 1));
    }

    #[test]
    fn second_kind_empty_strokes_false() {
        assert!(!second_kind_confirmed(&[], 0));
    }

    #[test]
    fn resolve_pending_when_apex_not_locatable() {
        // apex 区间与任何反向笔都不重叠 → 无法定位锚点 → Pending（不强断）。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 20),
            stroke(Direction::Down, 4, 8, 20, 5),
            stroke(Direction::Up, 8, 12, 5, 25),
        ];
        let apex = Interval { lo: 100, hi: 200 }; // 不重叠任何笔
        assert_eq!(resolve_second_kind(&strokes, &apex), SecondKindResult::Pending);
    }

    /// property：second_kind_confirmed 严格——无分型时恒不确认（Lead 硬约束：不产假线段）。
    #[test]
    fn property_no_fractal_never_confirms() {
        // 任何单调第二特征序列都不确认。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 20),
            stroke(Direction::Down, 4, 8, 20, 5),
            stroke(Direction::Up, 8, 12, 5, 7),
            stroke(Direction::Down, 12, 16, 7, 4),
            stroke(Direction::Up, 16, 20, 4, 9),
        ];
        // 仅 2 根向上笔（offset>1），< 3 不足成分型 → 不确认。
        assert!(!second_kind_confirmed(&strokes, 1));
    }

    #[test]
    fn apex_extreme_reads_directional_extreme() {
        let strokes = vec![stroke(Direction::Up, 0, 4, 5, 20)];
        // 向上线段 apex 极值 = 区间 hi = 20。
        assert_eq!(apex_extreme(&strokes, 0, Direction::Up), Some(20));
        // 向下线段 apex 极值 = 区间 lo = 5。
        assert_eq!(apex_extreme(&strokes, 0, Direction::Down), Some(5));
    }
}
