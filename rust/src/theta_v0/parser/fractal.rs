//! 第二步：分型识别（reference-theta-v0.md:20，[缠论可导,62课]）。
//!
//! ## 规则（reference-theta-v0.md:20 逐字）
//!
//! - **输入**：包含处理后的 K 序列（inclusion.rs 输出的 merged bars）。
//! - **顶分型**：中 K 的 high 和 low 都**严格**高于左右相邻 K（`等价不成立`）。
//! - **底分型**：中 K 的 high 和 low 都**严格**低于左右相邻 K。
//! - **确认**：第三根 K 收盘确认（即三根 K 构成分型，分型在第三根确认）。
//!
//! ## bit-exact 注意点
//!
//! - **严格不等号**（`>` / `<`，非 `>=`/`<=`）：reference-theta-v0.md:20 明确「等价不成立」。
//!   这是 `[缠论可导,62课；严格等号处理设计选择]`——严格性是规则核心，不可松为非严格。
//! - 顶分型要求中 K high>左右 high **且** low>左右 low（两个极值都严格高）；底镜像。
//! - 包含处理后相邻 K 两两非包含（inclusion 不动点），故中 K 与左右的 high/low 严格序
//!   关系良定义（不会同时 high 高而 low 低）。
//! - 分型的 `source_index`/`timestamp`/极值价取**中 K**（分型顶点所在 K）。

use super::super::types::{Fractal, FractalKind};
use super::super::types::Bar;
use std::rc::Rc;

/// 在包含处理后的 merged K 序列上识别分型（reference-theta-v0.md:20）。
///
/// 扫描每个内部三元组 `(left, mid, right)`：
/// - 顶分型：`mid.high > left.high && mid.high > right.high && mid.low > left.low
///   && mid.low > right.low`（严格，等价不成立）。
/// - 底分型：四个严格 `<` 镜像。
///
/// 第三根 K 确认：分型在 `right`（第三根）出现时确认，故输出顺序按 mid 的位置升序。
///
/// 边界条件：
/// - `merged.len() < 3` ⟹ 无分型（无完整三元组）。
/// - 中 K 既非严格全高也非严格全低 ⟹ 非分型（跳过；含包含处理残留的非严格情形）。
pub fn detect_fractals(merged: &[Bar]) -> Vec<Fractal> {
    let mut out = Vec::new();
    if merged.len() < 3 {
        return out;
    }
    for i in 1..merged.len() - 1 {
        let left = &merged[i - 1];
        let mid = &merged[i];
        let right = &merged[i + 1];

        let is_top = mid.high > left.high
            && mid.high > right.high
            && mid.low > left.low
            && mid.low > right.low;
        let is_bottom = mid.high < left.high
            && mid.high < right.high
            && mid.low < left.low
            && mid.low < right.low;

        if is_top {
            out.push(Fractal {
                kind: FractalKind::Top,
                source_index: mid.source_index,
                timestamp: mid.timestamp,
                price: mid.high,
            });
        } else if is_bottom {
            out.push(Fractal {
                kind: FractalKind::Bottom,
                source_index: mid.source_index,
                timestamp: mid.timestamp,
                price: mid.low,
            });
        }
    }
    out
}

/// 分型供给口（T1 键域重锚 #170）：源序号 → 该处 confirmed 分型（分型管单一来源）。
///
/// `fractals` 按 `source_index` 升序（合并序列顺序三联扫描产出）⟹ partition_point 二分
/// O(log n)。返回分型本体（`Fractal` 为 Copy）——未命中 = 该源序号无 confirmed 分型
/// （诚实 None，消费侧禁降级第二查法）。
pub fn fractal_at_source(fractals: &[Fractal], source_index: usize) -> Option<Fractal> {
    let i = fractals.partition_point(|f| f.source_index < source_index);
    fractals.get(i).filter(|f| f.source_index == source_index).copied()
}

// ============================================================================
// 增量 fractal（#93 H1 主根因：parse_layer 下游 per-bar O(n²) → 增量化）。
//
// ## 前缀不变性（bit-exact 基础）
//
// 分型在 mid 位置确认，需 right = merged[mid+1]。inclusion 增量只修改 acc = merged 末元素
// （merged_prefix 不可变）。故 mid ≤ n-4 的分型不可变（right ≤ merged[n-3] ∈ prefix）。
// 仅 mid ∈ {n-3, n-2} 的分型可能受新 bar 影响（right 可能是 acc）。
//
// 增量策略：保留 confirmed 前缀（mid ≤ n-4 的分型），重算尾部（mid ≥ n-3）。
// 严格性：内部存 (Fractal, mid_idx) 对，按 mid_idx 精确过滤——不依赖 source_index 代理。
// ============================================================================

/// 增量 fractal 状态（bit-exact 对齐 `detect_fractals`）。
///
/// 保留 confirmed 前缀分型 + 对应 mid 下标 + 当前 merged 长度。
/// append 新 merged 序列时只重算尾部 2 个三元组（mid ≥ confirmed_mid_bound）。
///
/// Rc 共享——`to_result_rc()` 返回 `Rc::clone` O(1)，替代旧 `to_result()` 的 O(n) map+collect。
/// `fractals_rc` + `mid_indices` 双 Vec 同步 truncate（ponytail: 复用缓冲消除 O(n)/bar clone）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncrFractals {
    /// confirmed 前缀分型（mid < confirmed_mid_bound，不可变）。
    /// Rc 共享——`to_result_rc()` O(1) clone 给 `ParseLayer.fractals`。
    fractals_rc: Rc<Vec<Fractal>>,
    /// 对应 mid 下标（与 fractals_rc 同步 truncate/push）。
    mid_indices: Vec<usize>,
    /// 上次快照时的 merged 长度（用于确定重算起点）。
    merged_len: usize,
}

impl IncrFractals {
    /// 空状态。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 从全量 `detect_fractals` 结果 + merged 长度恢复增量状态（断点续算）。
    ///
    /// 重建 mid_idx：对每个分型，mid = 其在 merged 中的位置（source_index 单调性不足以
    /// 精确反推 mid 下标，故要求调用方提供 merged 用于重建——或接受全量重算）。
    /// 严格起见：`from_full` 仅在 merged 与 fractals 一致时使用，内部全量重扫建 mid_idx。
    pub fn from_full(fractals: &[Fractal], merged: &[Bar]) -> Self {
        if merged.len() < 3 || fractals.is_empty() {
            return IncrFractals {
                fractals_rc: Rc::new(Vec::new()),
                mid_indices: Vec::new(),
                merged_len: merged.len(),
            };
        }
        // 全量重扫建 mid_idx——from_full 是断点续算入口，一次性 O(n) 可接受。
        let mut mid_indices = Vec::with_capacity(fractals.len());
        let mut fi = 0usize;
        for i in 1..merged.len() - 1 {
            if fi < fractals.len() {
                let mid = &merged[i];
                let expected_src = fractals[fi].source_index;
                if mid.source_index == expected_src {
                    mid_indices.push(i);
                    fi += 1;
                }
            }
        }
        IncrFractals {
            fractals_rc: Rc::new(fractals.to_vec()),
            mid_indices,
            merged_len: merged.len(),
        }
    }

    /// 增量追加 merged 序列，消费 self 返回新状态（by-value 缓冲复用，消除 O(n)/bar clone）。
    ///
    /// bit-exact：结果分型序列 == `detect_fractals(merged)`。
    ///
    /// 保留 mid < confirmed_mid_bound 的前缀分型（不可变），重算 mid ≥ confirmed_mid_bound 的尾部。
    /// confirmed_mid_bound = min(old_len, n).saturating_sub(2)（mid+1 < 此值的 right 不可变）。
    /// ponytail: truncate 复用 Vec 缓冲 + Rc::make_mut 突变 fractals_rc，替代旧
    /// `take_while().collect()` + `to_result()` 的 O(n)/bar map+collect。
    pub fn append(self, merged: &[Bar]) -> IncrFractals {
        let n = merged.len();
        if n < 3 {
            return IncrFractals {
                fractals_rc: Rc::new(Vec::new()),
                mid_indices: Vec::new(),
                merged_len: n,
            };
        }
        // 移出 self 字段（by-value 消费）。
        let IncrFractals {
            mut fractals_rc,
            mut mid_indices,
            merged_len: old_len,
        } = self;

        // mid 不可变 ⟺ right = mid+1 在不可变前缀内 ⟺ mid+1 < min(old_len, n) - 1。
        // inclusion 增量只改末元素（acc），故 merged[0..n-1] 不可变（n == old_len 时 acc=merged[n-1] 可变）。
        // old_len < n 时 merged[0..old_len] 完全不可变（旧 acc 已定稿）。
        // confirmed_mid_bound = mid < 此值的分型不可变。
        let confirmed_mid_bound = old_len.saturating_sub(2).min(n.saturating_sub(2));

        // ponytail: partition_point O(log n) 定位 truncate 位置，替代 take_while O(n)。
        let keep = mid_indices
            .partition_point(|&mid_idx| mid_idx < confirmed_mid_bound);

        // Rc::make_mut 突变 fractals_rc——strong_count==1（self 被消费，旧 ParseLayer 已 drop）
        // 时 O(1) in-place；strong_count>1 时 O(n) deep copy（调用方须丢弃上一轮 ParseLayer 保 O(1)）。
        let new_fractals = Rc::make_mut(&mut fractals_rc);
        new_fractals.truncate(keep);
        mid_indices.truncate(keep);

        // 重算尾部：mid ∈ [confirmed_mid_bound.saturating_sub(1) .. n-1)。
        // 从 confirmed_mid_bound-1 起扫（多算 1 个保边界——但该 mid 的分型若已在新_prefix 则不重复）。
        // 严格：只扫 mid ≥ confirmed_mid_bound（前缀已覆盖 mid < confirmed_mid_bound）。
        let scan_start = confirmed_mid_bound.max(1);
        for i in scan_start..n - 1 {
            if i == 0 {
                continue;
            }
            let left = &merged[i - 1];
            let mid = &merged[i];
            let right = &merged[i + 1];

            let is_top = mid.high > left.high
                && mid.high > right.high
                && mid.low > left.low
                && mid.low > right.low;
            let is_bottom = mid.high < left.high
                && mid.high < right.high
                && mid.low < left.low
                && mid.low < right.low;

            if is_top {
                new_fractals.push(Fractal {
                    kind: FractalKind::Top,
                    source_index: mid.source_index,
                    timestamp: mid.timestamp,
                    price: mid.high,
                });
                mid_indices.push(i);
            } else if is_bottom {
                new_fractals.push(Fractal {
                    kind: FractalKind::Bottom,
                    source_index: mid.source_index,
                    timestamp: mid.timestamp,
                    price: mid.low,
                });
                mid_indices.push(i);
            }
        }

        IncrFractals {
            fractals_rc,
            mid_indices,
            merged_len: n,
        }
    }

    /// 当前快照分型序列（与 `detect_fractals` bit-exact）。
    pub fn to_result(&self) -> Vec<Fractal> {
        self.fractals_rc.to_vec()
    }

    /// 当前快照分型序列的 Rc 共享句柄（O(1) refcount bump）。
    ///
    /// ponytail: Rc 共享替代旧 `to_result()` 的 O(n) map+collect——`ParseLayerIncr::append`
    /// 用此填 `ParseLayer.fractals`，消除每 bar Vec clone。
    pub fn to_result_rc(&self) -> Rc<Vec<Fractal>> {
        Rc::clone(&self.fractals_rc)
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

    // -------- T1 (#170) 分型供给口测试（先红后绿） --------

    #[test]
    fn fractal_at_source_hit_miss_and_order() {
        let fs = vec![
            Fractal { kind: FractalKind::Top, source_index: 3, timestamp: 3, price: 150 },
            Fractal { kind: FractalKind::Bottom, source_index: 8, timestamp: 8, price: 90 },
            Fractal { kind: FractalKind::Top, source_index: 21, timestamp: 21, price: 170 },
        ];
        assert_eq!(
            fractal_at_source(&fs, 8).map(|f| (f.kind, f.price)),
            Some((FractalKind::Bottom, 90)),
            "命中返回分型本体（方向 + 极值价）"
        );
        assert!(fractal_at_source(&fs, 9).is_none(), "非分型中K位置不得命中");
        assert!(fractal_at_source(&fs, 0).is_none(), "首元素之前不得命中");
        assert!(fractal_at_source(&[], 0).is_none(), "空供给不得命中");
    }

    #[test]
    fn detects_top_fractal_strict() {
        // 中 K [15,12] 严格高于左 [10,5] 右 [11,6] 的 high 和 low → 顶分型。
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 11, 6)];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FractalKind::Top);
        assert_eq!(f[0].source_index, 1);
        assert_eq!(f[0].price, 15);
    }

    #[test]
    fn detects_bottom_fractal_strict() {
        // 中 K [6,2] 严格低于左 [12,8] 右 [11,7] → 底分型，price=low=2。
        let merged = vec![bar(0, 12, 8), bar(1, 6, 2), bar(2, 11, 7)];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FractalKind::Bottom);
        assert_eq!(f[0].price, 2);
    }

    #[test]
    fn equality_does_not_form_fractal() {
        // 中 K high 等于右 K high（非严格）→ 不成立（reference-theta-v0.md:20 等价不成立）。
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 15, 6)];
        let f = detect_fractals(&merged);
        assert!(f.is_empty(), "非严格高低不应形成分型");
    }

    #[test]
    fn fewer_than_three_bars_no_fractal() {
        assert!(detect_fractals(&[bar(0, 10, 5), bar(1, 15, 12)]).is_empty());
        assert!(detect_fractals(&[]).is_empty());
    }

    #[test]
    fn golden_alternating_top_then_bottom() {
        // 五根：底→顶→底交替（每个内部三元组各产一个分型）。
        // [10,5] [15,12] [11,6] [4,1] [8,3]
        // i=1: mid[15,12] > 左[10,5] 右[11,6] → 顶
        // i=2: mid[11,6]：左[15,12] 右[4,1]，high 11<15 非全高；11>4 但 low 6>1，非全低 → 非分型
        // i=3: mid[4,1] < 左[11,6] 右[8,3] → 底
        let merged = vec![
            bar(0, 10, 5),
            bar(1, 15, 12),
            bar(2, 11, 6),
            bar(3, 4, 1),
            bar(4, 8, 3),
        ];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind, FractalKind::Top);
        assert_eq!(f[1].kind, FractalKind::Bottom);
    }

    /// property：分型的极值价就是中 K 的对应极值（顶=high，底=low）。
    #[test]
    fn property_fractal_price_is_mid_extreme() {
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 11, 6)];
        let f = detect_fractals(&merged);
        assert_eq!(f[0].price, merged[1].high); // 顶 → 中 K high
    }

    /// ★bit-exact：增量 IncrFractals::append 链 == 全量 detect_fractals（逐 merged 长度）。
    #[test]
    fn bit_exact_incr_fractals_per_bar() {
        // 合成 merged 序列（交替顶底 + 平坦段，触发各类分型 + 尾部边界）。
        let merged: Vec<Bar> = (0..200usize)
            .map(|i| {
                let (h, l) = match i % 7 {
                    0 => (10, 5),
                    1 => (20, 15),
                    2 => (12, 8),
                    3 => (25, 18),
                    4 => (8, 3),
                    5 => (22, 14),
                    _ => (15, 10),
                };
                bar(i, h + (i as i64), l + (i as i64))
            })
            .collect();

        let mut incr = IncrFractals::empty();
        for end in 1..=merged.len() {
            incr = incr.append(&merged[..end]);
            let full = detect_fractals(&merged[..end]);
            assert_eq!(
                incr.to_result(),
                full,
                "merged len {end}: 增量 fractal != 全量（bit-exact 破裂）"
            );
        }
    }
}
