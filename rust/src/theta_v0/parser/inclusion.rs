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

// ============================================================================
// 增量包含合并 API（#93 per-bar substrate O(n²) 根因解，aed4d5f5 缺口）。
//
// ## 缺口锚点（前序工位实证）
//
// `incremental.rs:10`：`parse_layer(&bars[..=i])` 的 `merged_bars` 每 bar 严格增长 ⟹
// ParseLayer-equality gate 命中率 0%（缓存路径死路）。真 O(n) 须 **parser 内部增量 API**
// （`incremental.rs:23`、`runner.rs:280`）。本 API 是该缺口的 parser 侧解。
//
// ## bit-exact 不变量（铁律）
//
// 对任意 bar 序列 `bars` 与任意 `i`：
//   `process_inclusion_append_n(prev_state_for_bars[..i], &bars[i])` 的输出 `merged`
//   == `process_inclusion(&bars[..=i]).merged`，逐字段精确。
//
// 结构性保证：本增量算法与全量 `process_inclusion` 共享同一个左折叠步进语义
// （`fold_step`），仅状态表示不同——对拍测试（下方 `bit_exact_per_bar_inclusion`）
// 作为运行时硬断言。
//
// ## 算法（镜像全量两步，状态化）
//
// 全量算法两步：(1) 开头方向扫描（找第一对非包含严格单调对定 `start_dir`）；
// (2) 有方向左折叠。增量状态机分两相，精确对应：
//
// - **相 A（only_open_tail，开头方向未定）**：`raw_pending` 缓存全部原始 bar。每追加 1 bar，
//   增量重跑开头方向扫描的**增量步**——只在 `raw_pending` 末尾对新追加 bar 与前一 bar
//   做一次非包含严格对判定（旧前缀已扫过，结果不变）。若找到方向 → 一次性从 `raw_pending`
//   全量左折叠进入相 B（O(n) 但全程仅发生一次）；否则维持相 A。
// - **相 B（有方向 left-fold）**：`merged_prefix`（已定稿，confirmed 前缀不动）+ `acc`
//   （当前未定稿段）+ `dir`。每追加 1 bar 调 `fold_step`（O(1)）：包含则合并进 acc，
//   非包含则 acc 定稿入 `merged_prefix`、更新 dir、新 bar 成新 acc。
//
// ## 复杂度
//
// 稳态（相 B）：append 1 bar = O(1)。相 A → 相 B 的一次性迁移 = O(n)（全程仅一次）。
// 故对 n bar 序列的逐 bar 增量总计 O(n)（vs 全量逐 bar O(n²)）。开头方向迟迟不定时
// 多 bar 停留在相 A，每 bar O(1)（追加 + 尾部一对判定）；最终迁移仍是一次 O(n)。
// ============================================================================

/// 增量包含合并状态机（bit-exact 对齐 `process_inclusion`）。
///
/// 表示全量算法的中间态，支持 append-1-bar 增量推进。构造后不可变（函数式推进，
/// 每次 `append` 返回新状态——coding-style immutability）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrInclusion {
    /// 相 A：开头方向未定时缓存原始 bar（方向确定后清空）。
    raw_pending: Vec<Bar>,
    /// 相 B：已定稿的合并段（confirmed 前缀，追加 bar 不改其内容）。
    merged_prefix: Vec<Bar>,
    /// 相 B：当前未定稿合并段（可能被后续 bar 吸收）。
    /// 相 A 下为 `None`（acc 概念不存在）。
    acc: Option<Bar>,
    /// 相 B：当前折叠方向。
    dir: MergeDir,
    /// 开头方向（None = 相 A 仍 only_open_tail；Some = 相 B 已定方向）。
    start_dir: Option<MergeDir>,
}

impl IncrInclusion {
    /// 空状态（零 bar）。
    pub fn empty() -> Self {
        IncrInclusion {
            raw_pending: Vec::new(),
            merged_prefix: Vec::new(),
            acc: None,
            dir: Direction::Up, // 占位（相 A 不用）；定方向时覆盖。
            start_dir: None,
        }
    }

    /// 从已有 `InclusionResult`（全量版输出）恢复增量状态。
    ///
    /// 用途：从全量基线起继续增量追加（如断点续算）。**仅当 `prev.only_open_tail=false`
    /// 时可精确恢复 left-fold 状态**——此时需从 `prev.merged` 重建 `(merged_prefix, acc, dir)`。
    /// `dir` 从 `merged` 末两根的严格高低变化推断（若末尾不足定方向，保持前序）。
    ///
    /// 边界条件：
    /// - `prev.only_open_tail=true` ⟹ 回退到相 A，`raw_pending = prev.merged.clone()`，
    ///   `start_dir=None`（与全量从头扫描语义一致）。
    /// - `prev.merged` 为空 ⟹ `empty()`。
    pub fn from_result(prev: &InclusionResult) -> Self {
        if prev.merged.is_empty() {
            return IncrInclusion::empty();
        }
        if prev.only_open_tail {
            // 全程无方向：原始 bar 序列 = merged（未合并）。回到相 A 等待方向出现。
            return IncrInclusion {
                raw_pending: prev.merged.clone(),
                merged_prefix: Vec::new(),
                acc: None,
                dir: Direction::Up,
                start_dir: None,
            };
        }
        // 有方向相 B：末根为 acc，前缀定稿。dir 从末两根严格高低变化推断（与全量
        // 左折叠中"acc 与新 bar 的 strict_dir 更新"语义一致——取末两根若有严格对）。
        let (prefix, last) = prev.merged.split_at(prev.merged.len() - 1);
        let acc = last[0];
        let dir = match (prefix.last(), Some(&acc)) {
            (Some(p, ), Some(a)) => strict_dir(p, a).unwrap_or(Direction::Up),
            _ => Direction::Up, // 仅一根 merged（合并成一根），无前对照——占位 Up。
        };
        IncrInclusion {
            raw_pending: Vec::new(),
            merged_prefix: prefix.to_vec(),
            acc: Some(acc),
            dir,
            start_dir: Some(dir),
        }
    }

    /// 追加 1 bar，返回新状态（immutability）。
    ///
    /// bit-exact：本 bar 后调 `to_result()` 的输出 == `process_inclusion(全部已追加 bar)`。
    pub fn append(&self, new_bar: Bar) -> IncrInclusion {
        match self.start_dir {
            // 相 B：稳态 O(1) left-fold 步进。
            Some(_) => self.append_folded(new_bar),
            // 相 A：方向未定，追加到 raw_pending，增量扫描开头方向。
            None => self.append_pending(new_bar),
        }
    }

    /// 当前状态快照为 `InclusionResult`（与全量输出 bit-exact）。
    pub fn to_result(&self) -> InclusionResult {
        match self.start_dir {
            None => InclusionResult {
                merged: self.raw_pending.clone(),
                only_open_tail: true,
            },
            Some(_) => {
                let mut merged = self.merged_prefix.clone();
                if let Some(acc) = self.acc {
                    merged.push(acc);
                }
                InclusionResult {
                    merged,
                    only_open_tail: false,
                }
            }
        }
    }

    /// 相 A 推进：追加到 raw_pending，增量判定开头方向是否出现。
    ///
    /// 增量扫描性质：全量开头扫描查"第一对非包含严格单调对"。旧前缀已扫无非包含严格对
    /// （否则已进入相 B），故只需判 `raw_pending` 末根 与 `new_bar` 这一对。若严格 →
    /// 进入相 B，一次性全量左折叠 `raw_pending ++ [new_bar]`。
    fn append_pending(&self, new_bar: Bar) -> IncrInclusion {
        let mut raw = self.raw_pending.clone();
        raw.push(new_bar);
        // 增量判定：旧前缀（追加前 raw_pending）已无非包含严格对（否则已进相 B）。
        // 故全量开头扫描的"第一对非包含严格对"只可能在新增的末尾对 (last, new_bar) 出现。
        let n = raw.len();
        let start_dir = if n >= 2 {
            let prev = &raw[n - 2];
            if !contains(prev, &new_bar) {
                strict_dir(prev, &new_bar)
            } else {
                None
            }
        } else {
            None
        };
        match start_dir {
            None => {
                // 仍 only_open_tail。
                IncrInclusion {
                    raw_pending: raw,
                    merged_prefix: Vec::new(),
                    acc: None,
                    dir: Direction::Up,
                    start_dir: None,
                }
            }
            Some(dir0) => {
                // 进入相 B：从 raw 全量左折叠（一次性 O(n)，全程仅一次）。
                let (merged_prefix, acc, dir) = fold_all(&raw, dir0);
                IncrInclusion {
                    raw_pending: Vec::new(),
                    merged_prefix,
                    acc: Some(acc),
                    dir,
                    start_dir: Some(dir0),
                }
            }
        }
    }

    /// 相 B 推进：O(1) left-fold 步进（镜像全量算法 inclusion.rs:137-148 循环体）。
    fn append_folded(&self, new_bar: Bar) -> IncrInclusion {
        let acc = self.acc.expect("相 B 下 acc 必非空（不变量）");
        let mut merged_prefix = self.merged_prefix.clone();
        let (new_acc, new_dir) = fold_step(&acc, new_bar, self.dir, &mut merged_prefix);
        IncrInclusion {
            raw_pending: Vec::new(),
            merged_prefix,
            acc: Some(new_acc),
            dir: new_dir,
            start_dir: self.start_dir,
        }
    }
}

/// 左折叠单步（镜像全量 `process_inclusion` 循环体，inclusion.rs:137-148）。
///
/// 给定当前 acc + 新 bar + 方向，推进：包含则合并进 acc（返回新 acc）；非包含则 acc
/// 定稿 push 到 `merged_out`、新 bar 成新 acc、按 strict_dir 更新 dir。返回 `(新 acc, 新 dir)`。
fn fold_step(acc: &Bar, b: Bar, dir: MergeDir, merged_out: &mut Vec<Bar>) -> (Bar, MergeDir) {
    if contains(acc, &b) {
        (merge(acc, &b, dir), dir)
    } else {
        let mut new_dir = dir;
        if let Some(d) = strict_dir(acc, &b) {
            new_dir = d;
        }
        merged_out.push(*acc);
        (b, new_dir)
    }
}

/// 全量左折叠（相 A → 相 B 迁移时一次性调用，镜像全量 inclusion.rs:134-149）。
///
/// 给定原始 bar 序列 + 开头方向，返回 `(merged_prefix, acc, final_dir)`。
/// `merged_prefix` 不含最终 acc（acc 留作增量态的未定稿段）。
fn fold_all(bars: &[Bar], dir0: MergeDir) -> (Vec<Bar>, Bar, MergeDir) {
    debug_assert!(
        !bars.is_empty(),
        "fold_all 仅在相 A 累积 ≥1 bar 后调用"
    );
    let mut merged: Vec<Bar> = Vec::with_capacity(bars.len());
    let mut acc = bars[0];
    let mut dir = dir0;
    for b in &bars[1..] {
        let (na, nd) = fold_step(&acc, *b, dir, &mut merged);
        acc = na;
        dir = nd;
    }
    (merged, acc, dir)
}

/// 增量包含合并便利函数：`prev` 状态 + 1 bar → 新 `InclusionResult`。
///
/// 等价 `IncrInclusion::from_state(prev).append(new_bar).to_result()`，便于逐 bar 调用。
pub fn process_inclusion_append(prev: &IncrInclusion, new_bar: &Bar) -> (IncrInclusion, InclusionResult) {
    let next = prev.append(*new_bar);
    let result = next.to_result();
    (next, result)
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

    // -------- 增量 API 测试（bit-exact 对拍，#93） --------

    /// 合成数据逐 bar 对拍：增量 `IncrInclusion::append` 链 == 全量 `process_inclusion`。
    ///
    /// 镜像 incremental.rs:132 `bit_exact_per_bar` 模式——每追加 1 bar 后，增量态的
    /// `to_result()` 必须等于全量 `process_inclusion(&bars[..=i])`，逐字段精确。
    fn bit_exact_inclusion_chain(bars: &[Bar]) {
        let mut incr = IncrInclusion::empty();
        for (i, b) in bars.iter().enumerate() {
            incr = incr.append(*b);
            let incr_result = incr.to_result();
            let full_result = process_inclusion(&bars[..=i]);
            assert_eq!(
                incr_result, full_result,
                "bar {i}: 增量 inclusion != 全量（bit-exact 破裂）\n\
                 incr: {:?}\nfull: {:?}",
                incr_result, full_result
            );
        }
    }

    #[test]
    fn incr_empty_chain_matches_full() {
        // 空 + 单根 + 两根无方向 → 全程相 A。
        bit_exact_inclusion_chain(&[]);
        bit_exact_inclusion_chain(&[bar(0, 10, 5)]);
        bit_exact_inclusion_chain(&[bar(0, 10, 5), bar(1, 8, 6)]); // 互含
    }

    #[test]
    fn incr_simple_up_then_merge_matches_full() {
        // golden_simple_up_then_inclusion_merge 的增量版（A→B 定方向 + 尾部合并）。
        let bars = vec![bar(0, 10, 5), bar(1, 12, 7), bar(2, 11, 8)];
        bit_exact_inclusion_chain(&bars);
    }

    #[test]
    fn incr_all_contained_matches_full() {
        // 全程包含（相 A 始终）：每根都含于第一根，only_open_tail=true。
        let bars = vec![bar(0, 20, 1), bar(1, 10, 5), bar(2, 8, 6), bar(3, 15, 3)];
        bit_exact_inclusion_chain(&bars);
    }

    #[test]
    fn incr_direction_flip_after_delay_matches_full() {
        // ★bit-exact 关键边界：相 A 累积多 bar 后，末尾才出现首个严格对 → 相 A→B 迁移。
        // 前两根互含（相 A），第三根与第二根严格上升 → 翻转进入相 B 全量左折叠。
        let bars = vec![bar(0, 20, 1), bar(1, 18, 3), bar(2, 25, 10), bar(3, 22, 12)];
        bit_exact_inclusion_chain(&bars);
    }

    #[test]
    fn incr_alternating_dirs_matches_full() {
        // 多次方向切换（上→下→上）+ 中途包含合并，覆盖 fold_step 的 dir 更新与 acc 定稿。
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 15, 8),  // 严格上
            bar(2, 12, 9),  // 含于 1 → 向上合并
            bar(3, 8, 4),   // 严格下（非包含）→ acc 定稿，dir 翻下
            bar(4, 9, 5),   // 含于 3 → 向下合并
            bar(5, 14, 10), // 严格上 → acc 定稿，dir 翻上
            bar(6, 13, 11), // 含于 5 → 向上合并
        ];
        bit_exact_inclusion_chain(&bars);
    }

    #[test]
    fn incr_long_synthetic_matches_full() {
        // 长合成序列（混合包含 / 严格 / 方向翻转）逐 bar 对拍。
        let bars: Vec<Bar> = (0..500usize)
            .map(|i| {
                let base = 100i64 + (i as i64) * 3;
                let cycle = ((i as f64) / 17.0).sin() as i64 * 40;
                let close = base + cycle;
                bar(i, close + 6, close - 6)
            })
            .collect();
        bit_exact_inclusion_chain(&bars);
    }

    #[test]
    fn incr_from_result_resumes() {
        // 从全量结果恢复增量态，继续追加 bit-exact。
        let bars0 = vec![bar(0, 10, 5), bar(1, 15, 8), bar(2, 12, 9)];
        let full0 = process_inclusion(&bars0);
        let mut incr = IncrInclusion::from_result(&full0);
        // 追加若干 bar，与全量对照。
        let extra = vec![bar(3, 8, 4), bar(4, 9, 5), bar(5, 14, 10)];
        for (k, b) in extra.iter().enumerate() {
            incr = incr.append(*b);
            let total: Vec<Bar> = bars0.iter().chain(extra[..=k].iter()).copied().collect();
            let full = process_inclusion(&total);
            assert_eq!(incr.to_result(), full, "from_result 恢复后追加 bar {} 不匹配", k);
        }
    }
}
