//! 第七步：未完成尾部 tail（reference-theta-v0.md:25，[设计选择,默认值]）。
//!
//! ## 契约重锚（legacy Strict/OpenTail + Strict/Parse §6 → `Origin.ChanlunElements.OpenTail`）
//!
//! Origin `ElementPipeline.parse` 把解析输出切为 **confirmed（ParseStruct 主体，已闭合走势，交易
//! 可用）** + **`tail : OpenTail`（未完成尾部，仅预警）**。`Origin.ChanlunElements.OpenTail`
//! （inductive 五构造子）：`none / pendingFractal (bars) / pendingStroke (fractals) /
//! pendingSegment (strokes) / pendingMove (segments)`。本文件实装该 active 侧——把 parser 流水线
//! **各阶段**未确认的延伸结构显式保存为 `PendingTail`，**不**输出为 confirmed（对齐 `tailOf : ... -> OpenTail`）。
//!
//! 核心裁定：未完成走势只能唯一分类为**当下状态**（对应 `OpenTail` 的 pending* 构造子），**不能**
//! 强行分类为最终结果。`PendingTail` 的每个变体携带「方向/起点/当前极值」= 当下状态内容，**不**
//! 携带「最终终结结果」——未来延伸落入 reversal/continuation 哪个分支由后续真实数据决定（L2/L3），
//! 不由本文件预判。本文件只保存当下状态，不预测分支。
//!
//! ## 流水线阶段轴 vs 级别递归轴（两个正交的 tail 概念，非矛盾）
//!
//! ★诚实标注（formalization-validity-domain，澄清 spec 间张力）：
//! - `Origin.ChanlunElements.OpenTail` 是**级别递归轴**的尾部（一个解析级别最多一个正在延伸的
//!   走势尾部，单个 `OpenTail` 值的 pending* 构造子）。
//! - 本文件的 `tail : Vec<PendingTail>` 是**单层流水线阶段轴**的尾部（K线→分型→笔→线段
//!   各阶段各自的未完成结构，reference:25 逐字列举 `PendingFractal|PendingStroke|
//!   PendingSegment|AliveCenter|PendingMove`，覆盖 `OpenTail` 五构造子的多阶段并存版本）。
//!
//! 两者正交：流水线阶段轴在「一个级别内部」的多个识别阶段上各保存一个 pending（栈式，
//! 从粗到细：段未完成 → 该段内最后笔未完成 → 该笔内最后分型未完成）；级别递归轴在「跨级别」
//! 上保存。reference:25 明确多变体，故 `Vec<PendingTail>` 是 frozen spec 要求，非声明膨胀。
//!
//! ## reference-theta-v0.md:25 逐字
//!
//! 「显式保存 `PendingFractal|PendingStroke|PendingSegment|AliveCenter|PendingMove`
//! （含方向/起点/当前极值/确认条件）；不输出为 confirmed。」
//!
//! ★范围（no-patch-mentality 诚实）：parser 单层流水线产出 **PendingFractal / PendingStroke
//! / PendingSegment** 三层（K线/分型/笔/线段四步对应的未完成结构）。`AliveCenter` 是中枢层
//! （步骤5，移交 classifier）、`PendingMove` 是级别递归层（classifier 递归级别）——二者不在
//! parser 单层流水线职责内，由 classifier 产出（不在本文件冒充）。

use super::super::types::{Bar, Direction, Fractal, FractalKind, PendingTail, Segment, Stroke, Tick};

// ============================================================================
// 增量 tail O(n²) 修复（#93 incr_total exp 1.89 残余：tail 全量重算 O(merged_i)/bar）。
//
// ## 根因（aa8ab531 标注）
//
// `extreme_after` + `pending_fractal` 每 bar 遍历整个 merged 序列：
// - `extreme_after`：`merged.iter().filter().collect()` + `max/min` = O(merged_i)/bar。
// - `pending_fractal`：`merged.iter().position()` = O(merged_i)/bar。
//
// ## 修复（ponytail：二分查找替代线性扫描）
//
// merged 的 source_index 严格单调递增（inclusion 左折叠只前进，合并段保留起点
// source_index）。故可用 `partition_point`（二分 O(log n)）定位锚点，然后只扫尾部
// 延伸段（稳态 O(1)——延伸段 = 新增 bar，非整个 merged）。
// ============================================================================

/// 从未确认线段的剩余笔序列读出 `PendingSegment`（reference:25，OpenTail 当下状态）。
///
/// `divide_segments_with_tail` 在 SecondKind/无分型/不足成段时停于 `pending_start`，
/// 剩余笔 `strokes[pending_start..]` 是一个正在延伸的未确认线段。当下状态：
/// - `direction`：剩余笔首笔方向（线段方向 = 首笔方向，第67课）。
/// - `start_index`:剩余笔首笔起点原始 K 序号。
/// - `current_extreme`:剩余笔在线段方向上的当前极值（向上=最高 end/start，向下=最低）。
///
/// 边界条件：剩余笔为空 ⟹ `None`（无未完成线段）。
fn pending_segment(strokes: &[Stroke], pending_start: usize) -> Option<PendingTail> {
    let rest = strokes.get(pending_start..)?;
    let first = rest.first()?;
    let direction = first.direction;
    // 当下极值：线段方向上剩余笔触及的最远价（向上取最高、向下取最低）——当下状态内容，
    // 非最终终结价（OpenTail：未完成走势只给当下状态）。
    let current_extreme = rest.iter().fold(
        match direction {
            Direction::Up => first.start_price.max(first.end_price),
            Direction::Down => first.start_price.min(first.end_price),
        },
        |acc, s| {
            let stroke_extreme = match direction {
                Direction::Up => s.start_price.max(s.end_price),
                Direction::Down => s.start_price.min(s.end_price),
            };
            match direction {
                Direction::Up => acc.max(stroke_extreme),
                Direction::Down => acc.min(stroke_extreme),
            }
        },
    );
    Some(PendingTail::PendingSegment {
        direction,
        start_index: first.start_index,
        current_extreme,
    })
}

/// 从已确认笔之后悬挂的延伸分型读出 `PendingStroke`（reference:25）。
///
/// 笔划分后，最后一个确认笔尾之后若 merged 仍在延伸但未形成下一个确认笔端（无满足间隔的
/// 异类分型配对），则有一个正在延伸的笔。当下状态：
/// - `direction`:下一笔方向（最后确认笔尾分型 = 下一笔起点，方向由其反向推断；最后笔尾
///   是顶 ⟹ 下笔向下，是底 ⟹ 向上）。无已确认笔时由首个悬挂分型类型推断。
/// - `start_index`:延伸笔起点（最后确认笔尾 / 首个悬挂分型）原始 K 序号。
/// - `current_extreme`:延伸方向上 merged 触及的当前极值。
///
/// ★诚实范围：本函数只在「有已确认笔且其后有未配对延伸分型」或「有分型但一根笔都未确认」
/// 时产生 PendingStroke。判据严格基于确认结构（最后笔 + 悬挂分型），不猜测。
///
/// 边界条件：分型 < 1 或无悬挂延伸 ⟹ `None`。
fn pending_stroke(
    strokes: &[Stroke],
    fractals: &[Fractal],
    merged: &[Bar],
) -> Option<PendingTail> {
    let last_fractal = fractals.last()?;
    // 已确认笔的最后笔尾原始 index（无笔时用首个分型 index 作起点）。
    let (anchor_index, anchor_kind) = match strokes.last() {
        Some(last_stroke) => {
            // 已确认笔尾之后无更晚的悬挂分型 ⟹ 无延伸笔（笔尾恰是最后分型）。
            if last_fractal.source_index <= last_stroke.end_index {
                return None;
            }
            // 笔尾分型类型 = 笔方向决定的端点类型（Up 笔尾=顶，Down 笔尾=底）。
            let kind = match last_stroke.direction {
                Direction::Up => FractalKind::Top,
                Direction::Down => FractalKind::Bottom,
            };
            (last_stroke.end_index, kind)
        }
        // 一根确认笔都没有：以首个分型作延伸笔起点。
        None => {
            let first = fractals.first()?;
            (first.source_index, first.kind)
        }
    };
    // 下一笔方向：从锚点分型反向（顶后向下、底后向上）。
    let direction = match anchor_kind {
        FractalKind::Top => Direction::Down,
        FractalKind::Bottom => Direction::Up,
    };
    // 延伸方向上 merged 中锚点之后触及的当前极值。
    let current_extreme = extreme_after(merged, anchor_index, direction)?;
    Some(PendingTail::PendingStroke {
        direction,
        start_index: anchor_index,
        current_extreme,
    })
}

/// 从最后确认分型之后的延伸 merged K 读出 `PendingFractal`（reference:25）。
///
/// 分型需第三根 K 收盘确认（reference:20）。最后一个确认分型之后，merged 序列若仍在延伸
/// 但尾部不足三根构成下一确认分型，则尾部存在一个未确认（pending）分型形态。当下状态：
/// - `kind`:延伸方向决定的临时分型类型（最后分型是顶 ⟹ 反向延伸找底，反之找顶）。
/// - `source_index`:延伸段当前极值所在原始 K 序号。
/// - `extreme`:延伸段当前极值价（顶=high 侧、底=low 侧）。
///
/// ★诚实范围：只在「最后确认分型之后 merged 仍有 ≥1 根延伸 K」时产生。延伸不足三根
/// （未确认）⟹ pending；已达三根则会被 `detect_fractals` 确认（不在 tail）。
///
/// 边界条件：无分型 / 最后分型已是 merged 末尾（无延伸）⟹ `None`。
fn pending_fractal(fractals: &[Fractal], merged: &[Bar]) -> Option<PendingTail> {
    let last = fractals.last()?;
    // 最后确认分型在 merged 中的位置（按 source_index 定位）。
    // ponytail: 二分查找替代线性 position（O(log n) vs O(n)）。merged.source_index 严格单调递增。
    let last_pos = merged
        .partition_point(|b| b.source_index < last.source_index);
    if last_pos >= merged.len() || merged[last_pos].source_index != last.source_index {
        return None;
    }
    // 最后分型之后的延伸 K（已确认分型之后仍在延伸的部分）。
    let tail_bars = merged.get(last_pos + 1..)?;
    if tail_bars.is_empty() {
        return None;
    }
    // 临时分型方向：最后分型反向（顶后找底、底后找顶——交替分型，reference:21）。
    let pending_kind = match last.kind {
        FractalKind::Top => FractalKind::Bottom,
        FractalKind::Bottom => FractalKind::Top,
    };
    // 延伸段当前极值（底分型方向找最低 low，顶找最高 high）。
    // ★平局裁决（reference:16）：多个相同极值时取**最早** source_index。`min_by_key`
    // 平局返回首个（最早，正确）；`max_by_key` 平局返回**末个**（违反「最早」），故顶分型
    // 用「最大 high 的首个 bar」显式选——遍历取严格更高者更新（`>` 非 `>=`，平局保最早）。
    let (extreme_bar, extreme) = match pending_kind {
        FractalKind::Bottom => {
            // min_by_key 平局取首个（最早 source_index），符合 :16。
            let b = tail_bars.iter().min_by_key(|b| b.low)?;
            (b, b.low)
        }
        FractalKind::Top => {
            // 严格更高才更新（`>`），平局保留更早的（先扫描到的）bar——符合 :16。
            let b = tail_bars
                .iter()
                .reduce(|acc, b| if b.high > acc.high { b } else { acc })?;
            (b, b.high)
        }
    };
    Some(PendingTail::PendingFractal {
        kind: pending_kind,
        source_index: extreme_bar.source_index,
        extreme,
    })
}

/// merged 序列中锚点原始 index 之后、给定方向上的当前极值（向上取最高 high、向下最低 low）。
///
/// 边界条件：锚点之后无延伸 K ⟹ `None`。
///
/// ponytail: 二分查找定位锚点（O(log n)），只扫尾部延伸段（稳态 O(1)/bar）。
/// merged.source_index 严格单调递增（inclusion 左折叠不变量）。
fn extreme_after(merged: &[Bar], anchor_index: usize, direction: Direction) -> Option<Tick> {
    // 二分定位首个 source_index > anchor_index 的位置。
    let start = merged
        .partition_point(|b| b.source_index <= anchor_index);
    let after = merged.get(start..)?;
    if after.is_empty() {
        return None;
    }
    let ext = match direction {
        Direction::Up => after.iter().map(|b| b.high).max()?,
        Direction::Down => after.iter().map(|b| b.low).min()?,
    };
    Some(ext)
}

/// 组装未完成尾部 tail（reference-theta-v0.md:25，步骤7）——三层 Pending* 栈式保存。
///
/// 从 parser 流水线各阶段的确认结构 + 延伸读出三层未完成尾部（从粗到细）：
/// 1. **PendingSegment**：未确认线段（`divide_segments_with_tail` 停点后的剩余笔）。
/// 2. **PendingStroke**：未确认笔（最后确认笔尾后的悬挂延伸分型）。
/// 3. **PendingFractal**：未确认分型（最后确认分型后的延伸 merged 尾部）。
///
/// 顺序：从粗（段）到细（分型），对齐流水线阶段轴的栈语义（外层结构未完成 → 其内层
/// 最后元素也未完成）。每层独立判定，某层无未完成则跳过（不强行造 pending，no-patch）。
///
/// **段划分复用**（性能，#93）：调用方已跑过 `divide_segments_with_tail`（步骤4），本函数
/// 接收其 `(segments, pending_start)` 输出，**不重跑**段划分——产物路径段划分只跑一次。
/// 语义 bit-exact：`pending_start` 来自同一 `divide_segments_with_tail` 调用，与旧版内部自调等价。
///
/// 契约锚 `Origin.ChanlunElements.OpenTail`（active 尾部 pending* 构造子，当下状态，不预判分支）。
///
/// 边界条件：所有阶段都恰好闭合无延伸 ⟹ 空 tail。
pub fn build_tail(
    merged: &[Bar],
    fractals: &[Fractal],
    strokes: &[Stroke],
    _segments: &[Segment],
    pending_start: Option<usize>,
) -> Vec<PendingTail> {
    let mut tail = Vec::new();

    // 1. 未确认线段（段层）：从线段划分停点读出（复用调用方结果，不重跑）。
    if let Some(start) = pending_start {
        if let Some(pt) = pending_segment(strokes, start) {
            tail.push(pt);
        }
    }

    // 2. 未确认笔（笔层）：最后确认笔尾后的悬挂延伸分型。
    if let Some(pt) = pending_stroke(strokes, fractals, merged) {
        tail.push(pt);
    }

    // 3. 未确认分型（分型层）：最后确认分型后的延伸 merged 尾部。
    if let Some(pt) = pending_fractal(fractals, merged) {
        tail.push(pt);
    }

    tail
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Timestamp;

    fn bar(i: usize, high: Tick, low: Tick) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as Timestamp,
            open: low,
            high,
            low,
            close: high,
            volume: 1,
            untradable: false,
        }
    }
    fn frac(kind: FractalKind, idx: usize, price: Tick) -> Fractal {
        Fractal {
            kind,
            source_index: idx,
            timestamp: idx as Timestamp,
            price,
        }
    }
    fn stroke(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Stroke {
        Stroke {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    #[test]
    fn pending_segment_from_leftover_strokes() {
        // 剩余笔 [Up 0..4 (0→10), Down 4..8 (10→5)] 从 start=0 → 未确认线段 Up，极值 10。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 10),
            stroke(Direction::Down, 4, 8, 10, 5),
        ];
        let pt = pending_segment(&strokes, 0).unwrap();
        match pt {
            PendingTail::PendingSegment {
                direction,
                start_index,
                current_extreme,
            } => {
                assert_eq!(direction, Direction::Up);
                assert_eq!(start_index, 0);
                assert_eq!(current_extreme, 10); // Up 线段当前最高
            }
            _ => panic!("应为 PendingSegment"),
        }
    }

    #[test]
    fn pending_segment_empty_rest_none() {
        let strokes = vec![stroke(Direction::Up, 0, 4, 0, 10)];
        assert!(pending_segment(&strokes, 5).is_none()); // 越界 start → None
    }

    #[test]
    fn pending_stroke_after_last_confirmed() {
        // 最后笔 Up 0..4，最后分型在 idx 8（> 笔尾 4）→ 悬挂延伸笔。
        // 笔尾 Up=顶 → 下一笔向下。merged 在 idx>4 的延伸低点。
        let strokes = vec![stroke(Direction::Up, 0, 4, 0, 10)];
        let fractals = vec![
            frac(FractalKind::Bottom, 0, 0),
            frac(FractalKind::Top, 4, 10),
            frac(FractalKind::Bottom, 8, 2),
        ];
        let merged = vec![bar(0, 1, 0), bar(4, 10, 9), bar(6, 8, 5), bar(8, 4, 2)];
        let pt = pending_stroke(&strokes, &fractals, &merged).unwrap();
        match pt {
            PendingTail::PendingStroke {
                direction,
                start_index,
                current_extreme,
            } => {
                assert_eq!(direction, Direction::Down); // 顶后向下
                assert_eq!(start_index, 4); // 最后笔尾
                assert_eq!(current_extreme, 2); // idx>4 的最低 low
            }
            _ => panic!("应为 PendingStroke"),
        }
    }

    #[test]
    fn pending_stroke_no_hanging_fractal_none() {
        // 最后分型 idx=4 = 最后笔尾 → 无悬挂延伸 → None。
        let strokes = vec![stroke(Direction::Up, 0, 4, 0, 10)];
        let fractals = vec![frac(FractalKind::Bottom, 0, 0), frac(FractalKind::Top, 4, 10)];
        let merged = vec![bar(0, 1, 0), bar(4, 10, 9)];
        assert!(pending_stroke(&strokes, &fractals, &merged).is_none());
    }

    #[test]
    fn pending_fractal_after_last_confirmed() {
        // 最后确认分型 Top idx=4，之后 merged 延伸到 idx 6,8（找底，最低 low=2 在 idx8）。
        let fractals = vec![frac(FractalKind::Bottom, 0, 0), frac(FractalKind::Top, 4, 10)];
        let merged = vec![bar(0, 1, 0), bar(4, 10, 9), bar(6, 8, 5), bar(8, 4, 2)];
        let pt = pending_fractal(&fractals, &merged).unwrap();
        match pt {
            PendingTail::PendingFractal {
                kind,
                source_index,
                extreme,
            } => {
                assert_eq!(kind, FractalKind::Bottom); // 顶后找底
                assert_eq!(source_index, 8); // 最低 low 所在
                assert_eq!(extreme, 2);
            }
            _ => panic!("应为 PendingFractal"),
        }
    }

    #[test]
    fn pending_fractal_no_extension_none() {
        // 最后分型已是 merged 末尾（无延伸）→ None。
        let fractals = vec![frac(FractalKind::Top, 4, 10)];
        let merged = vec![bar(0, 1, 0), bar(4, 10, 9)];
        assert!(pending_fractal(&fractals, &merged).is_none());
    }

    #[test]
    fn build_tail_empty_when_all_closed() {
        // 无分型无笔无延伸 → 空 tail。
        let merged = vec![bar(0, 10, 5)];
        assert!(build_tail(&merged, &[], &[], &[], None).is_empty());
    }

    #[test]
    fn build_tail_collects_pending_layers() {
        // 有剩余笔（不足成段）+ 悬挂分型 + 延伸分型 → 多层 pending。
        // 两笔不足成段（< 3）→ pending_start=Some(0) → PendingSegment。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 10),
            stroke(Direction::Down, 4, 8, 10, 3),
        ];
        let fractals = vec![
            frac(FractalKind::Bottom, 0, 0),
            frac(FractalKind::Top, 4, 10),
            frac(FractalKind::Bottom, 8, 3),
            frac(FractalKind::Top, 12, 14), // 悬挂（> 最后笔尾 8）
        ];
        let merged = vec![
            bar(0, 1, 0),
            bar(4, 10, 9),
            bar(8, 4, 3),
            bar(12, 14, 13),
        ];
        let tail = build_tail(&merged, &fractals, &strokes, &[], Some(0));
        // 至少含 PendingSegment（剩余笔不足成段）。
        assert!(tail
            .iter()
            .any(|pt| matches!(pt, PendingTail::PendingSegment { .. })));
    }

    /// property：build_tail 产出的 Pending* 都不携带"最终终结"——仅当下状态字段
    /// （方向/起点/当前极值）。这是 `Origin.ChanlunElements.OpenTail`「未完成只给当下状态」的结构见证。
    #[test]
    fn property_tail_carries_only_current_state() {
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 0, 10),
            stroke(Direction::Down, 4, 8, 10, 3),
        ];
        let merged = vec![bar(0, 1, 0), bar(4, 10, 9), bar(8, 4, 3)];
        let tail = build_tail(&merged, &[], &strokes, &[], Some(0));
        // 所有变体只可能是 parser 三层（不含 AliveCenter/PendingMove——那是 classifier 层）。
        for pt in &tail {
            assert!(matches!(
                pt,
                PendingTail::PendingSegment { .. }
                    | PendingTail::PendingStroke { .. }
                    | PendingTail::PendingFractal { .. }
            ));
        }
    }
}
