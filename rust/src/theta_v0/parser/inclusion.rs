//! 第一步：K线包含合并（reference-theta-v0.md:19，[缠论可导,62/65课]）。
//!
//! ## 规则（reference-theta-v0.md:19 逐字）
//!
//! - **相邻区间包含即合并**：相邻两 K（合并方向上）区间包含关系成立即合并为一根。
//! - **向上合并**：`high=max, low=max`（取两者较高的 high 与较高的 low）。
//! - **向下合并**：`low=min, high=min`（取两者较低的 low 与较低的 high）。
//! - **方向决定**：方向按**前一对非包含 K** 的严格高低变化决定。
//! - **开头无方向**：向前看第一个非包含对（开头 tie-break 是设计选择）。
//! - **全程无方向**：逐 raw 原件保持未定，正式组为空（#1342 / #1393）。
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

use super::super::types::{Bar, Direction, Tick};
use std::rc::Rc;

/// 合并方向：包含处理的左折叠方向（向上吞并取高，向下吞并取低）。
///
/// 与 `Direction` 区分：`Direction` 是笔/线段的几何方向；此处是**包含合并**的局部
/// 处理方向，由前一对非包含 K 的严格高低变化决定（reference-theta-v0.md:19）。
type MergeDir = Direction;

/// 判断两 K 是否包含（一方闭区间 [low,high] 含另一方）。
///
/// 边界条件：相等区间（A==B）视为包含（互含）——按合并规则处理为一根。
///
/// `pub(crate)`（#1370 TB-01-A）：CC-006 `local_shape` 的域前件「相邻两对均非包含」复用
/// 本谓词（与 inclusion 同一判断，不另起第二查法）。
pub(crate) fn contains(a: &Bar, b: &Bar) -> bool {
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
        volume: acc.volume + b.volume, // #919：f64 加法（原 saturating_add 是 i64 防溢出语义）
        untradable: acc.untradable || b.untradable,
    }
}

/// 严格高低变化方向：从 K `prev` 到 K `cur` 的方向（reference-theta-v0.md:19）。
///
/// 严格高高、低低 → Up；严格低低、低高 → Down；非严格（任一相等或矛盾）→ None。
/// 这是「前一对非包含 K 的严格高低变化」——只有两个区间**严格**单调（high 与 low
/// 同向严格变化）才确定方向。
///
/// `pub(crate)`（#1370 TB-01-A）：CC-006 `local_shape` 的两次方向（dir(a,b)/dir(b,c)）复用
/// 本谓词（与 inclusion 同一判断，不另起第二查法）。返回类型改为公开的 [`Direction`]
/// （原私有别名 `MergeDir` = `Direction`，语义不变）。
pub(crate) fn strict_dir(prev: &Bar, cur: &Bar) -> Option<Direction> {
    if cur.high > prev.high && cur.low > prev.low {
        Some(Direction::Up)
    } else if cur.high < prev.high && cur.low < prev.low {
        Some(Direction::Down)
    } else {
        None
    }
}

/// #1393 / #1342：W 的政策版本独立于 raw、结构切面及对象修订。
pub const INITIALIZATION_POLICY: &str = "raw-initialization-w/1";

/// 正式合并组与未定原件分开；快照保存真实折叠方向，单组也可精确恢复。
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionResult {
    pub merged: Vec<Bar>,
    pub only_open_tail: bool,
    pub pending_raw: Vec<Bar>,
    pub final_direction: Option<Direction>,
}

impl Default for InclusionResult {
    fn default() -> Self {
        Self {
            merged: vec![],
            only_open_tail: true,
            pending_raw: vec![],
            final_direction: None,
        }
    }
}

/// #1393：只检查 raw 相邻对；无证据时不发布正式组，不造 envelope。
fn first_raw_direction(bars: &[Bar]) -> Option<(usize, Direction)> {
    bars.windows(2).enumerate().find_map(|(i, pair)| {
        if contains(&pair[0], &pair[1]) {
            None
        } else {
            strict_dir(&pair[0], &pair[1]).map(|direction| (i, direction))
        }
    })
}

pub fn process_inclusion(bars: &[Bar]) -> InclusionResult {
    let Some((_, direction)) = first_raw_direction(bars) else {
        return InclusionResult {
            pending_raw: bars.to_vec(),
            ..InclusionResult::default()
        };
    };
    let (merged, final_direction) = fold_all(bars, direction);
    InclusionResult {
        merged,
        only_open_tail: false,
        pending_raw: vec![],
        final_direction: Some(final_direction),
    }
}

/// 组锚供给线（T1 键域重锚 #170）：该级包含层 → 源序号 `source_index` 所在合并组的锚。
///
/// 合并组锚 = **组内首根序号**（merged bar 保留组内首根 `source_index`，本文件 :23/:57；
/// 教义 ADR `adr-chain-confirmation-and-invariant-identity-20260722` 裁定 3：「合并组锚 =
/// 组内首根序号，非极值发生根」）。`merged` 按 `source_index` 严格升序（左折叠追加）⟹
/// 组 i 覆盖原始序号 `[src_i, src_{i+1})`（末组覆盖 `[src_last, ∞)`，尾部仍可生长），
/// 故锚 = 最后一个 ≤ `source_index` 的 `merged[i].source_index`（partition_point 二分
/// O(log n)）。
///
/// 幂等：`source_index` 本身是组锚（分型/线段端点坐标已是合并锚——`Fractal.source_index`
/// = 中K merged bar 的首根序号）时返回其自身；是组内后续根（如极值发生根）时映射回组锚。
/// 本口是组锚的**单一来源**——消费侧（gate `absorb_exts` 登记、T2 投影层索引）禁二次
/// 推导（禁第二查法）。
pub fn merged_group_anchor(merged: &[Bar], source_index: usize) -> Option<usize> {
    merged_group_index(merged, source_index).map(|g| merged[g].source_index)
}

/// 组号供给线（编号域统一 S9，SPEC #847 拆出）：原始序号 `source_index` → 所在合并组的
/// 组号（= merged 下标 = 合并位置）。
///
/// 与 [`merged_group_anchor`] **同源**（读同一份 `merged`、同一个 partition_point 二分，
/// 禁第二查法）——组号是「组锚」缺的那一步：组锚返回组的**首根原始序号**，本口返回该组
/// 的**序数**（即 merged 下标）。`merged` 按 `source_index` 严格升序 ⟹ 组号 = 最后一个
/// `merged[i].source_index <= source_index` 的 i（partition_point 二分 O(log n)）。
///
/// 空 `merged` ⟹ None（诚实无组号）。幂等：组锚自身与组内后续根映射到同一组号；
/// 末组之后的 `source_index`（尾部仍可生长）映射到末组。
pub fn merged_group_index(merged: &[Bar], source_index: usize) -> Option<usize> {
    let i = merged.partition_point(|b| b.source_index <= source_index);
    (i > 0).then(|| i - 1)
}

/// 标准化 K 线稠密标注（编号域统一载体 S9，SPEC #847 拆出；#813 S-1 裁定二回流补字段）。
///
/// 每根原始 K 一条标注（与输入 `bars` 等长、按下标一一对应，序号都在）。把「原始序号 →
/// 组号」、组标准化高低、「已确认／暂定」状态位与「组内逐根高低」全部显式化——不产出一条
/// 更短的新序列，判断逻辑一行不改。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenseBarAnnotation {
    /// 所属标准化组号；#1393 未定原件为 None，不伪造组号。
    pub group: Option<usize>,
    /// 该组标准化后的 high（组内所有成员同一值）。
    pub group_high: Option<Tick>,
    /// 该组标准化后的 low（组内所有成员同一值）。
    pub group_low: Option<Tick>,
    /// 已确认（true）／暂定（false）——尾巴不稳定明账化：新 bar 只改最后一组边界、前缀
    /// 不动（与 fractal 增量「前缀不动、只重算尾部两组」同源纪律抬到地基层）。
    pub confirmed: bool,
    /// 本根原始 K 自己的 high（组内逐根高低——#813 S-1 裁定二回流补字段，供「组内 high
    /// 最大的那根原始 K」一类判据直接读数，不丢组内逐根极值）。
    pub high: Tick,
    /// 本根原始 K 自己的 low（组内逐根高低）。
    pub low: Tick,
}

/// 从包含处理结果导出稠密标注（与 [`merged_group_anchor`]／[`merged_group_index`] 同源：
/// 读同一份 `merged`，由同一趟 partition_point 二分推导——不另起第二查法）。
///
/// `ann[i]` 对应 `bars[i]`（第 i 根原始 K，其原始序号为 `bars[i].source_index`）。`result`
/// 为 [`process_inclusion`] 的输出；组号由 `bars[i].source_index` 经 [`merged_group_index`]
/// 求得。已确认／暂定：
/// - `result.only_open_tail` ⟹ 全程无方向、无 confirmed 结构 ⟹ 全部暂定；
/// - 否则前缀组（组号 < 末组）已确认，末组（组号 = `merged.len()-1`）暂定（尾部仍可生长）。
///
/// bit-exact 验收门：由本标注重建的合并序列与 `process_inclusion(bars).merged` 逐字段相同
/// （见测试 `dense_annotation_roundtrip_bit_exact`）。
pub fn dense_annotation(bars: &[Bar], result: &InclusionResult) -> Vec<DenseBarAnnotation> {
    bars.iter()
        .map(|b| {
            let group = if result.only_open_tail {
                None
            } else {
                merged_group_index(&result.merged, b.source_index)
            };
            DenseBarAnnotation {
                group,
                group_high: group.map(|i| result.merged[i].high),
                group_low: group.map(|i| result.merged[i].low),
                confirmed: group.is_some_and(|i| i + 1 < result.merged.len()),
                high: b.high,
                low: b.low,
            }
        })
        .collect()
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
///
/// ## 存储布局（增量化 to_result，ae0118c0 残余 O(n²) 解 + Rc 共享 ceiling 突破）
///
/// - **相 A**（`start_dir=None`）：`raw_pending` 缓存原始 bar；`merged` 为空。
/// - **相 B**（`start_dir=Some`）：`merged` 是**连续**的合并序列——`merged[..len-1]`
///   为 confirmed 前缀，`merged.last()` 为当前未定稿 acc。连续布局使 `to_result_ref`
///   返回 `&self.merged`（零 clone）；旧 `merged_prefix.clone() + push(acc)` 每 bar
///   clone 整个 Vec 的 O(n²) 已消除。
///
/// `merged: Rc<Vec<Bar>>`（Rc 共享所有权，#93 incr_total ceiling 突破）：
/// `ParseLayerIncr::append` 的 O(n²) 主导 = `to_result_ref().merged.to_vec()` 每 bar
/// clone 整个 merged 填 `ParseLayer.merged_bars`。改 Rc 后 `merged_rc()` 返回
/// `Rc::clone`（O(1) refcount bump），`ParseLayer.merged_bars: Rc<Vec<Bar>>` 共享同一
/// 分配。`append_folded` 用 `Rc::make_mut` 突变——当 `strong_count==1`（prev.append 消费
/// prev，上一轮 ParseLayer 已 drop）时 O(1)；若上一轮 ParseLayer 仍存活则 O(n) deep copy
/// （调用方须丢弃上一轮 ParseLayer 以保 O(1)，profile/runner 均如此）。
///
/// // ponytail: 无剩余 ceiling（incr_total O(n²) → O(n) 已闭合）。
#[derive(Debug, Clone, PartialEq)]
pub struct IncrInclusion {
    /// 相 A：开头方向未定时缓存原始 bar（方向确定后清空）。
    raw_pending: Vec<Bar>,
    /// 相 B：连续合并序列（confirmed 前缀 + 末尾 acc）。相 A 下为空。
    /// Rc 共享——`merged_rc()` 给 `ParseLayer.merged_bars` O(1) clone，`make_mut` 突变。
    merged: Rc<Vec<Bar>>,
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
            merged: Rc::new(Vec::new()),
            dir: Direction::Up, // 占位（相 A 不用）；定方向时覆盖。
            start_dir: None,
        }
    }

    /// #1393：只使用快照保存的方向；不得从末两组猜测或为单组默认 UP。
    pub fn from_result(prev: &InclusionResult) -> Self {
        if prev.only_open_tail {
            assert!(prev.merged.is_empty() && prev.final_direction.is_none());
            return Self {
                raw_pending: prev.pending_raw.clone(),
                ..Self::empty()
            };
        }
        assert!(!prev.merged.is_empty() && prev.pending_raw.is_empty());
        let dir = prev.final_direction.expect("已定快照必须保存实际方向");
        Self {
            raw_pending: vec![],
            merged: Rc::new(prev.merged.clone()),
            dir,
            start_dir: Some(dir),
        }
    }

    /// 追加 1 bar，消费 self 返回新状态（函数式 immutability，mem::take 重用 Vec 缓冲）。
    ///
    /// bit-exact：本 bar 后调 `to_result()` / `to_result_ref()` 的输出 ==
    /// `process_inclusion(全部已追加 bar)`。
    pub fn append(self, new_bar: Bar) -> IncrInclusion {
        match self.start_dir {
            // 相 B：稳态 O(1) left-fold 步进。
            Some(_) => self.append_folded(new_bar),
            // 相 A：方向未定，追加到 raw_pending，增量扫描开头方向。
            None => self.append_pending(new_bar),
        }
    }

    /// 当前状态快照为 `InclusionResult`（与全量输出 bit-exact）。
    ///
    /// 拥有版（clone merged）——供测试 / `from_result` / 便利函数使用。热路径
    /// （`ParseLayerIncr::append`）应改用 `merged_rc()` O(1) Rc 共享。
    pub fn to_result(&self) -> InclusionResult {
        let (merged, only_open_tail) = self.merged_view();
        InclusionResult {
            merged: merged.to_vec(),
            only_open_tail,
            pending_raw: self.raw_pending.clone(),
            final_direction: self.start_dir.map(|_| self.dir),
        }
    }

    /// 当前状态快照的借用视图（零 clone，与全量输出 bit-exact）。
    ///
    /// `merged` 只承载正式组；相 A 的原件由 `pending_raw` 单独提供。
    pub fn to_result_ref(&self) -> InclusionResultRef<'_> {
        let (merged, only_open_tail) = self.merged_view();
        InclusionResultRef {
            merged,
            only_open_tail,
            pending_raw: &self.raw_pending,
            final_direction: self.start_dir.map(|_| self.dir),
        }
    }

    /// 合并序列的 Rc 共享句柄（O(1) refcount bump，#93 incr_total ceiling 突破）。
    ///
    /// 相 B 返回 `Rc::clone(&self.merged)`（与 `to_result_ref().merged` 内容 bit-exact，
    /// 共享同一分配）；相 A 返回空正式组。供热路径 `ParseLayerIncr::append` 替代
    /// `to_result_ref().merged.to_vec()` 的每 bar O(n) clone。
    ///
    /// bit-exact：返回的 Rc<Vec<Bar>> 内容 == `to_result().merged` 逐字段。
    pub fn merged_rc(&self) -> Rc<Vec<Bar>> {
        Rc::clone(&self.merged)
    }

    /// 内部：返回合并序列的借用切片 + only_open_tail 标志。
    fn merged_view(&self) -> (&[Bar], bool) {
        match self.start_dir {
            None => (&self.merged, true),
            Some(_) => (self.merged.as_slice(), false),
        }
    }

    /// W 等待期只判新增的最后一对；建立时一次性从 raw0 重折。
    fn append_pending(self, new_bar: Bar) -> IncrInclusion {
        let mut raw = self.raw_pending;
        raw.push(new_bar);
        let start_dir = first_raw_direction(&raw[raw.len().saturating_sub(2)..]);
        match start_dir {
            None => Self {
                raw_pending: raw,
                ..Self::empty()
            },
            Some((_, dir0)) => {
                let (merged, dir) = fold_all(&raw, dir0);
                Self {
                    raw_pending: vec![],
                    merged: Rc::new(merged),
                    dir,
                    start_dir: Some(dir0),
                }
            }
        }
    }

    /// 相 B 推进：O(1) left-fold 步进（镜像全量算法 inclusion.rs:137-148 循环体）。
    ///
    /// 连续布局：`merged` 末根即 acc。`Rc::make_mut` 取得可变借用（消费 self 的 Rc 所有权），
    /// pop 旧 acc（仅尾部 mutation，前缀不动），fold_step 推进，push 新 acc。当 `strong_count==1`
    /// （prev.append 消费 prev，上一轮 ParseLayer 已 drop）时全程 O(1)；`strong_count>1` 时
    /// make_mut deep copy O(n)（调用方须丢弃上一轮 ParseLayer 保 O(1)）。
    fn append_folded(mut self, new_bar: Bar) -> IncrInclusion {
        let merged = Rc::make_mut(&mut self.merged);
        // 末根即 acc（相 B 不变量：merged 非空）。
        let acc = merged
            .pop()
            .expect("相 B 下 merged 非空（acc 在末尾，不变量）");
        let (new_acc, new_dir) = fold_step(&acc, new_bar, self.dir, merged);
        merged.push(new_acc);
        IncrInclusion {
            raw_pending: Vec::new(),
            merged: self.merged,
            dir: new_dir,
            start_dir: self.start_dir,
        }
    }
}

/// 增量包含合并的借用视图（零 clone 快照，与 `InclusionResult` 内容 bit-exact）。
///
/// `merged` 借用 `IncrInclusion` 内部连续存储；`only_open_tail` 同 `InclusionResult`。
/// 供热路径调用方（`ParseLayerIncr::append`）替代 `to_result().merged` 的每 bar clone。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InclusionResultRef<'a> {
    pub merged: &'a [Bar],
    pub only_open_tail: bool,
    pub pending_raw: &'a [Bar],
    pub final_direction: Option<Direction>,
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
/// 给定原始 bar 序列 + 开头方向，返回 `(merged, final_dir)`。`merged` 是**连续**序列
/// （confirmed 前缀 + 末尾 acc），直接作为相 B 的 `merged` 存储——`to_result_ref`
/// 据此零 clone 返回 `&merged`。
fn fold_all(bars: &[Bar], dir0: MergeDir) -> (Vec<Bar>, MergeDir) {
    debug_assert!(!bars.is_empty(), "fold_all 仅在相 A 累积 ≥1 bar 后调用");
    let mut merged: Vec<Bar> = Vec::with_capacity(bars.len());
    let mut acc = bars[0];
    let mut dir = dir0;
    for b in &bars[1..] {
        let (na, nd) = fold_step(&acc, *b, dir, &mut merged);
        acc = na;
        dir = nd;
    }
    // acc 入尾——连续布局：merged[..len-1] = confirmed 前缀，merged.last() = acc。
    merged.push(acc);
    (merged, dir)
}

/// 增量包含合并便利函数：`prev` 状态 + 1 bar → 新 `InclusionResult`。
///
/// 等价 `IncrInclusion::from_state(prev).append(new_bar).to_result()`，便于逐 bar 调用。
/// `prev` 按值消费（与 `append` 一致）。
pub fn process_inclusion_append(
    prev: IncrInclusion,
    new_bar: &Bar,
) -> (IncrInclusion, InclusionResult) {
    let next = prev.append(*new_bar);
    let result = next.to_result();
    (next, result)
}

/// #1373：方向证据保存建立时的 acc，而不是以后变化的组快照。
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionDirectionEvidence {
    pub direction: Direction,
    pub previous_acc: Bar,
    pub incoming: Bar,
    pub source_coords: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InclusionGroupFact {
    pub bar: Bar,
    pub members: Vec<usize>,
    pub high_sources: Vec<usize>,
    pub low_sources: Vec<usize>,
    pub confirmed: bool,
    pub confirmation_evidence: Option<InclusionDirectionEvidence>,
    pub direction_evidence: Option<InclusionDirectionEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InclusionStepFact {
    pub action: &'static str,
    pub incoming: Bar,
    pub acc_before: Option<Bar>,
    pub acc_after: Option<Bar>,
    pub contains: Option<bool>,
    pub direction_evidence: Option<InclusionDirectionEvidence>,
    pub source_coords: Vec<usize>,
    pub high_sources: Vec<usize>,
    pub low_sources: Vec<usize>,
    pub waiting_reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InclusionFacts {
    /// #1392：同次 parser 的生产新笔与证据，包含层自身不填判定。
    pub stroke_facts: Option<super::stroke::StrokeFacts>,
    /// #1404：同次生产线段状态机的构造过程。
    pub segment_facts: Option<super::segment::SegmentFacts>,
    pub merged: Vec<Bar>,
    pub groups: Vec<InclusionGroupFact>,
    pub steps: Vec<InclusionStepFact>,
    pub initial_direction_unsettled: bool,
    pub initial_direction_evidence: Option<InclusionDirectionEvidence>,
    pub pending_raw: Vec<Bar>,
    pub unmapped_sources: Vec<usize>,
}

/// #1393：完整 facts 与生产热路径共用一个状态机；仅证据展开成本不同。
///
/// 全量入口与增量热路径是同一个 [`IncrFactsState`]：`record_steps` 只决定是否同时展开
/// 历史 `source_coords` 与 `steps`，不改变任何判据。
pub fn process_inclusion_with_facts(bars: &[Bar]) -> InclusionFacts {
    let mut state = IncrFactsState {
        record_steps: true,
        ..IncrFactsState::empty()
    };
    for bar in bars {
        state.append(*bar);
    }
    InclusionFacts {
        initial_direction_unsettled: state.unsettled(),
        unmapped_sources: state.unmapped.clone(),
        pending_raw: state.pending_raw,
        initial_direction_evidence: state.initial_evidence,
        merged: state.merged.as_ref().clone(),
        groups: state.groups,
        steps: state.full_steps,
        ..InclusionFacts::default()
    }
}

/// #1392：热路径逐 bar 的**紧凑同源包含状态**；#1393：W 初始化语义的唯一状态机。
///
/// ## #1393 W 初始化（`raw-initialization-w/1`）
///
/// 方向只由首个 raw 相邻**非包含严格**对建立。此前 raw 全部是未定原件（`pending_raw`），
/// 不发布正式组、不造 envelope；全程无方向时正式组为空。建立那一次 append 从 raw0
/// 一次性重折：seed raw0（方向证据 = 初始化证据），其后 raw[1..] 逐根走**同一条**已建方向
/// 折叠路径（[`IncrFactsState::fold`]），故批量重折与逐根增量共享证据逐位相同。没有
/// 「严格失败后退回旧初始化」的第二判据——旧「建立前遇包含即永久未定」已删除。
///
/// ## 证据表示
///
/// - 紧凑字段（`merged`/`groups`/`positions`）供生产热路径；`groups` 内的方向/封口证据在
///   紧凑模式不展开 `source_coords`。
/// - #1394 共享表示：每步、每组方向/封口证据的真实坐标集合以 [`CoordsRef`] 结构共享保存，
///   [`IncrFactsState::materialize_facts`] 可逐位还原全量入口。W 下非 waiting 步与方向证据
///   的来源集合恰为「到该刻为止全部 raw 坐标」（建立前缀整体参与初始化），故直接取到达序
///   前缀 `raw_prefix` 的快照，每根 O(log n) 共享节点、不复制历史。
/// - `record_steps`（全量入口）额外按 W 的集合定义逐步展开，供降级域（源坐标非严格递增）
///   回落使用；降级域不签发共享证据。
///
/// `positions`/`ordinal`/`raw`/`raw_prefix`/`last_source_index`/`degraded` 按到达序记录，
/// 等待期照常推进，不随重折重置。
#[derive(Debug, Clone, Default)]
pub struct IncrFactsState {
    merged: Rc<Vec<Bar>>,
    groups: Vec<InclusionGroupFact>,
    /// 当前方向证据（紧凑模式 `source_coords` 为空）；None ⟺ 方向尚未建立。
    evidence: Option<InclusionDirectionEvidence>,
    /// #1393：初始化证据（建立时的首个 raw 相邻非包含严格对）。
    initial_evidence: Option<InclusionDirectionEvidence>,
    initial_node: Option<Rc<EvidenceNode>>,
    /// #1393：方向建立前的全部原件（按到达序）。
    pending_raw: Vec<Bar>,
    /// #1393：建立方向时已到达的 raw 数（= 该刻步骤数）；None = 仍在等待期。
    established_at: Option<usize>,
    /// #1393：本次 append 从 raw0 重折——该代没有可复用的旧正式前缀。
    replayed_from_zero: bool,
    /// 全量入口：同时按 W 集合定义展开 `steps`/证据坐标。
    record_steps: bool,
    full_steps: Vec<InclusionStepFact>,
    positions: std::collections::BTreeMap<usize, usize>,
    ordinal: usize,
    /// #1394：已到达 raw（按到达序）。
    raw: super::evidence::ChunkedSeq<Bar>,
    /// #1394：已进入折叠的 raw 数（等待期为 0）。
    processed: usize,
    /// #1394：每步完整事实（不可变节点句柄）。
    steps: Vec<StepRecord>,
    /// #1394：方向证据/封口证据的真实坐标句柄（按组序对齐）。
    group_direction: Vec<Option<Rc<EvidenceNode>>>,
    group_confirmation: Vec<Option<Rc<EvidenceNode>>>,
    /// #1394：当前方向证据（= 末组 `direction_evidence` 的同一节点）。
    current_evidence: Option<Rc<EvidenceNode>>,
    /// #1394：到达序 raw 坐标前缀。
    raw_prefix: super::evidence::CoordsRef,
    /// #1394：每组「组锚之后」的成员坐标链（组内到达序，严格递增）。
    group_tail: Vec<super::evidence::CoordsRef>,
    /// #1393：未定原件坐标（= `pending_raw` 坐标）。
    unmapped: Vec<usize>,
    /// #1394：每组内容版本号（成员/极值/封口任一处变化即 +1）。
    group_versions: Vec<u64>,
    /// #1394：每组 `high_sources`/`low_sources` 的共享快照。
    group_high: Vec<Rc<Vec<usize>>>,
    group_low: Vec<Rc<Vec<usize>>>,
    last_source_index: Option<usize>,
    /// #1394：坐标非严格递增 ⟹ 共享表示失效，展开回落全量入口。
    degraded: bool,
}

/// 方向证据的完整节点（不可变、可共享）：`source_coords` 用共享坐标集表示。
#[derive(Debug, Clone)]
pub struct EvidenceNode {
    pub direction: Direction,
    pub previous_acc: Bar,
    pub incoming: Bar,
    pub coords: super::evidence::CoordsRef,
}

/// 单步完整事实的共享节点。
#[derive(Debug, Clone)]
struct StepRecord {
    action: &'static str,
    incoming: Bar,
    acc_before: Option<Bar>,
    acc_after: Option<Bar>,
    contains: Option<bool>,
    evidence: Option<Rc<EvidenceNode>>,
    coords: super::evidence::CoordsRef,
    high: Option<Rc<Vec<usize>>>,
    low: Option<Rc<Vec<usize>>>,
    waiting_reasons: Vec<&'static str>,
}

/// #1394 5.3：单步完整证据的**只读借用视图**——同核共享证据直接成值边的入口。
///
/// 集合字段借自同一 [`IncrFactsState`] 的共享句柄或切片；固定大小的 Bar 与标量按值复制。
/// **不复制**坐标集合与价格来源集合，也不把方向证据展开成
/// [`InclusionDirectionEvidence`]（后者会物化 `source_coords`）。
/// 视图不可变：不导出可变借用；取用视图不改变状态，也不改变既有 `step_fact` 的读数。
///
/// 字段与 [`InclusionStepFact`] 逐位对应：
/// - `coords` / `evidence.coords`：本步来源坐标的**共享**集合（升序去重、保留稀疏坐标）；
/// - `evidence`：方向证据共享节点（`direction` / `previous_acc` / `incoming` / `coords`）；
/// - `high` / `low`：极值实际来源集合快照（无快照时为空切片，与全量展开的空 `Vec` 同形）；
/// - 坐标集合的**物理**树形状不参与公开内容身份，下游须按自身唯一分解遍历
///   （见 [`super::evidence::CoordsRef::root_node`]）。
#[derive(Debug, Clone, Copy)]
pub struct StepEvidenceView<'a> {
    pub action: &'static str,
    pub incoming: Bar,
    pub acc_before: Option<Bar>,
    pub acc_after: Option<Bar>,
    pub contains: Option<bool>,
    pub evidence: Option<&'a Rc<EvidenceNode>>,
    pub coords: &'a super::evidence::CoordsRef,
    pub high: &'a [usize],
    pub low: &'a [usize],
    pub waiting_reasons: &'a [&'static str],
}

/// #1394 5.3：单组完整证据的**只读借用视图**——同核共享组证据直接成值边的入口
/// （CC005/006/007 的输入源集合、极值来源与方向/封口证据不必展开历史前缀）。
///
/// 与 [`StepEvidenceView`] 同契约：集合字段全部借自同一 [`IncrFactsState`] 的既有存储
/// （成员切片、组尾坐标链、极值来源集合、方向/封口证据节点句柄），固定大小的 `Bar` 与
/// 标量按值复制；**不复制**集合，也不把证据展开成 [`InclusionDirectionEvidence`]。
/// 视图不可变：不导出可变借用；取用视图不改变状态，也不改变既有
/// [`IncrFactsState::group_fact`] / [`IncrFactsState::materialize_groups`] 的读数。
///
/// 字段与 [`InclusionGroupFact`] 逐位对应：
/// - `members`：该组全部成员源坐标（升序；与 `dense_annotation` 映射同值，依据见
///   [`IncrFactsState::shared_group`]）；
/// - `member_tail`：同一序列**去掉首锚**后的共享坐标链（= `members[1..]`，组锚 =
///   `bar.source_index`）——供下游 O(1) 取尾部句柄拼接输入源集合；
/// - `high_sources` / `low_sources`：极值实际来源集合快照；
/// - `direction_evidence` / `confirmation_evidence`：共享 [`EvidenceNode`] 句柄，
///   `coords` 的物理树形状不参与公开内容身份（见
///   [`super::evidence::CoordsRef::root_node`]）。
#[derive(Debug, Clone, Copy)]
pub struct GroupEvidenceView<'a> {
    pub bar: Bar,
    pub members: &'a [usize],
    pub member_tail: &'a super::evidence::CoordsRef,
    pub high_sources: &'a [usize],
    pub low_sources: &'a [usize],
    pub confirmed: bool,
    pub direction_evidence: Option<&'a Rc<EvidenceNode>>,
    pub confirmation_evidence: Option<&'a Rc<EvidenceNode>>,
}

/// #1394 5.3：共享证据不可用——显式错误，与「索引不存在」严格分开。
///
/// 与 `Ok(None)`（该索引没有步骤）不同：本错误表示**整个共享态失效**（当前唯一成因：
/// source 坐标非严格递增，共享拼接前提被破坏）。此时不静默回落成另一套判据，也不无声
/// 返回缺对象；需要完整事实的调用方走既有 `step_fact` 的全量回落路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedEvidenceUnavailable {
    /// 失效成因（固定文本，供调用方区分错误种类）。
    pub reason: &'static str,
}

/// #1394 5.3：借用接口返回的共享坐标类型定义在 `evidence`（crate 内模块）。此再导出
/// 给出它们的公开路径，使 crate 外的消费方（如 S 生产侧 `bin`）也能命名遍历用的节点
/// 类型；不得据此把物理树形状当成规范 M 结构（见 `CoordsRef::root_node` 文档）。
pub use super::evidence::{CoordNode, CoordsRef};

/// #1488：同一次 append 的实际写入范围；不是 raw 原件或事务资格。
///
/// 前缀由本核的写入点给出，不比较旧 bars、不猜尾部长度。组的范围还包括前组封口，
/// merged 范围只含数值序列的写入；waiting 不改两者，但会追加 step/未映射来源。
/// #1393：建立方向那一次 append 从 raw0 重折，步骤、组、merged 一律如实标为从 0 起脏
/// （等待期 waiting 步被 seed/折叠步取代）。字段只读，调用者不能把自己猜的边界包装成本核见证。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InclusionAppendChanges {
    old_steps: usize,
    new_steps: usize,
    step_dirty_from: usize,
    old_groups: usize,
    new_groups: usize,
    old_merged: usize,
    new_merged: usize,
    group_dirty_from: Option<usize>,
    merged_dirty_from: Option<usize>,
    was_unsettled: bool,
    is_unsettled: bool,
    shared_available: bool,
}

impl InclusionAppendChanges {
    pub fn step_counts(&self) -> (usize, usize) {
        (self.old_steps, self.new_steps)
    }
    /// 第一个被本次 append 改写或新增的步骤下标；普通 append = 旧步数，重折 = 0。
    pub fn step_dirty_from(&self) -> usize {
        self.step_dirty_from
    }
    pub fn group_counts(&self) -> (usize, usize) {
        (self.old_groups, self.new_groups)
    }
    pub fn merged_counts(&self) -> (usize, usize) {
        (self.old_merged, self.new_merged)
    }
    pub fn group_dirty_from(&self) -> Option<usize> {
        self.group_dirty_from
    }
    pub fn merged_dirty_from(&self) -> Option<usize> {
        self.merged_dirty_from
    }
    pub fn unsettled_transition(&self) -> (bool, bool) {
        (self.was_unsettled, self.is_unsettled)
    }
    /// #1393：本次 append 是否从 raw0 重折（建立方向）。
    pub fn replayed_from_zero(&self) -> bool {
        self.was_unsettled && !self.is_unsettled
    }
    /// false 时不准消费共享证据；不是触发另一套判据的开关。
    pub fn shared_available(&self) -> bool {
        self.shared_available
    }
}

impl IncrFactsState {
    pub fn empty() -> Self {
        Self::default()
    }

    /// 追加 1 根 raw bar（丢弃变化见证）。
    pub fn append(&mut self, incoming: Bar) {
        self.append_with_changes(incoming);
    }

    /// #1488：在同一 append 核的写入点产变化见证；不增加判断、不持有历史快照。
    pub fn append_with_changes(&mut self, incoming: Bar) -> InclusionAppendChanges {
        self.replayed_from_zero = false;
        let mut changes = InclusionAppendChanges {
            old_steps: self.steps.len(),
            new_steps: self.steps.len() + 1,
            step_dirty_from: self.steps.len(),
            old_groups: self.groups.len(),
            new_groups: self.groups.len(),
            old_merged: self.merged.len(),
            new_merged: self.merged.len(),
            group_dirty_from: None,
            merged_dirty_from: None,
            was_unsettled: self.unsettled(),
            is_unsettled: self.unsettled(),
            shared_available: !self.degraded,
        };
        self.positions.insert(incoming.source_index, self.ordinal);
        self.ordinal += 1;
        // 共享拼接前提：坐标严格递增。违反即降级（展开回落全量入口），不静默近似。
        if self
            .last_source_index
            .is_some_and(|previous| incoming.source_index <= previous)
        {
            self.degraded = true;
        }
        changes.shared_available = !self.degraded;
        self.last_source_index = Some(incoming.source_index);
        self.raw.push(incoming);
        self.raw_prefix = self.raw_prefix.append_greater(incoming.source_index);
        if self.evidence.is_some() {
            self.fold(incoming, &mut changes);
            changes.new_groups = self.groups.len();
            changes.new_merged = self.merged.len();
            return changes;
        }
        // #1393 W：等待期只判新增的最后一对 raw（更早的相邻对均已判过且未建立）。
        self.pending_raw.push(incoming);
        let n = self.pending_raw.len();
        let Some((_, direction)) = first_raw_direction(&self.pending_raw[n.saturating_sub(2)..])
        else {
            self.unmapped.push(incoming.source_index);
            if self.record_steps {
                self.full_steps.push(InclusionStepFact {
                    action: "waiting",
                    incoming,
                    acc_before: None,
                    acc_after: None,
                    contains: None,
                    direction_evidence: None,
                    source_coords: vec![incoming.source_index],
                    high_sources: vec![],
                    low_sources: vec![],
                    waiting_reasons: vec!["initial_direction_unsettled"],
                });
            }
            self.steps.push(StepRecord {
                action: "waiting",
                incoming,
                acc_before: None,
                acc_after: None,
                contains: None,
                evidence: None,
                coords: super::evidence::CoordsRef::single(incoming.source_index),
                high: None,
                low: None,
                waiting_reasons: vec!["initial_direction_unsettled"],
            });
            return changes;
        };
        self.establish(direction, &mut changes);
        changes
    }

    /// #1393：建立方向——从 raw0 一次性重折；变化见证记为一次原子变化（从 0 起脏）。
    fn establish(&mut self, direction: Direction, changes: &mut InclusionAppendChanges) {
        let raw = std::mem::take(&mut self.pending_raw);
        let n = raw.len();
        debug_assert!(n >= 2 && self.groups.is_empty() && self.merged.is_empty());
        let node = Rc::new(EvidenceNode {
            direction,
            previous_acc: raw[n - 2],
            incoming: raw[n - 1],
            coords: self.raw_prefix.clone(),
        });
        let evidence = InclusionDirectionEvidence {
            direction,
            previous_acc: raw[n - 2],
            incoming: raw[n - 1],
            // 首对之前无严格对也是初始化依据；修订该前缀须改变结构身份。
            source_coords: if self.record_steps {
                raw.iter().map(|b| b.source_index).collect()
            } else {
                vec![]
            },
        };
        self.initial_node = Some(Rc::clone(&node));
        self.initial_evidence = Some(evidence.clone());
        self.evidence = Some(evidence.clone());
        self.current_evidence = Some(Rc::clone(&node));
        // 等待期只有 waiting 步与未映射坐标；折叠态本为空，此处整体换成重折结果。
        self.steps.clear();
        self.full_steps.clear();
        self.unmapped.clear();
        self.established_at = Some(n);
        self.replayed_from_zero = true;
        let seed = raw[0];
        Rc::make_mut(&mut self.merged).push(seed);
        self.push_group(seed, evidence.clone(), Rc::clone(&node));
        let snapshot = Rc::clone(self.group_high.last().expect("刚推入"));
        if self.record_steps {
            self.full_steps.push(InclusionStepFact {
                action: "seed",
                incoming: seed,
                acc_before: None,
                acc_after: Some(seed),
                contains: None,
                direction_evidence: Some(evidence.clone()),
                source_coords: evidence.source_coords.clone(),
                high_sources: vec![seed.source_index],
                low_sources: vec![seed.source_index],
                waiting_reasons: vec![],
            });
        }
        self.steps.push(StepRecord {
            action: "seed",
            incoming: seed,
            acc_before: None,
            acc_after: Some(seed),
            contains: None,
            evidence: Some(node),
            coords: self.raw_prefix.clone(),
            high: Some(Rc::clone(&snapshot)),
            low: Some(snapshot),
            waiting_reasons: vec![],
        });
        self.processed = 1;
        for bar in &raw[1..] {
            self.fold(*bar, changes);
        }
        changes.step_dirty_from = 0;
        changes.new_steps = self.steps.len();
        changes.group_dirty_from = Some(0);
        changes.merged_dirty_from = Some(0);
        changes.new_groups = self.groups.len();
        changes.new_merged = self.merged.len();
        changes.is_unsettled = false;
    }

    /// 新组：紧凑组事实 + 共享句柄成对推入。
    fn push_group(&mut self, bar: Bar, evidence: InclusionDirectionEvidence, node: Rc<EvidenceNode>) {
        self.groups.push(InclusionGroupFact {
            bar,
            members: vec![bar.source_index],
            high_sources: vec![bar.source_index],
            low_sources: vec![bar.source_index],
            confirmed: false,
            confirmation_evidence: None,
            direction_evidence: Some(evidence),
        });
        let snapshot = Rc::new(vec![bar.source_index]);
        self.group_tail.push(super::evidence::CoordsRef::empty());
        self.group_direction.push(Some(node));
        self.group_confirmation.push(None);
        self.group_high.push(Rc::clone(&snapshot));
        self.group_low.push(snapshot);
        self.group_versions.push(0);
    }

    /// 已建方向后的唯一折叠路径（逐根增量与建立时重折共用）。
    fn fold(&mut self, incoming: Bar, changes: &mut InclusionAppendChanges) {
        let acc = *self.merged.last().expect("已建方向必有 acc");
        let direction = self.evidence.as_ref().expect("已建方向").direction;
        // W 集合定义：末组成员 ∪ 当前方向证据来源 ∪ {incoming}（升序去重）。
        let full_sources = if self.record_steps {
            let mut sources = self.groups.last().expect("已建方向必有组").members.clone();
            sources.extend(&self.evidence.as_ref().expect("已建方向").source_coords);
            sources.push(incoming.source_index);
            sources.sort_unstable();
            sources.dedup();
            sources
        } else {
            vec![]
        };
        // 共享表示：非降级域里上式恰为到达序全部 raw 坐标（含建立前缀）。
        let coords = self.raw_prefix.clone();
        let previous_high = Rc::clone(self.group_high.last().expect("有 acc ⟹ 至少一组"));
        let previous_low = Rc::clone(self.group_low.last().expect("有 acc ⟹ 至少一组"));
        let merged = Rc::make_mut(&mut self.merged);
        merged.pop();
        let prefix_len = merged.len();
        let (after, new_direction) = fold_step(&acc, incoming, direction, merged);
        let included = merged.len() == prefix_len;
        merged.push(after);
        // fold_step 在非包含分支原样保留旧 acc，仅追加 after；包含分支改旧末根。
        changes.merged_dirty_from = Some(if included { prefix_len } else { prefix_len + 1 });
        changes.group_dirty_from = Some(self.groups.len() - 1);
        let (high, low) = if !included {
            let node = Rc::new(EvidenceNode {
                direction: new_direction,
                previous_acc: acc,
                incoming,
                coords: coords.clone(),
            });
            let evidence = InclusionDirectionEvidence {
                direction: new_direction,
                previous_acc: acc,
                incoming,
                source_coords: full_sources.clone(),
            };
            let previous = self.groups.last_mut().unwrap();
            previous.confirmed = true;
            previous.confirmation_evidence = Some(evidence.clone());
            *self.group_confirmation.last_mut().unwrap() = Some(Rc::clone(&node));
            // 封口改了前一组的内容（confirmed + confirmation_evidence）⟹ 版本 +1。
            *self.group_versions.last_mut().unwrap() += 1;
            self.push_group(after, evidence.clone(), Rc::clone(&node));
            self.evidence = Some(evidence);
            self.current_evidence = Some(node);
            let snapshot = Rc::clone(self.group_high.last().unwrap());
            (Rc::clone(&snapshot), snapshot)
        } else {
            let group = self.groups.last_mut().unwrap();
            group.bar = after;
            group.members.push(incoming.source_index);
            // #1343 RD-02：保留按到达序的全部同价根。
            let mut high_changed = false;
            let mut low_changed = false;
            if incoming.high == after.high {
                if acc.high == after.high {
                    group.high_sources.push(incoming.source_index);
                } else {
                    group.high_sources = vec![incoming.source_index];
                }
                high_changed = true;
            }
            if incoming.low == after.low {
                if acc.low == after.low {
                    group.low_sources.push(incoming.source_index);
                } else {
                    group.low_sources = vec![incoming.source_index];
                }
                low_changed = true;
            }
            let high = if high_changed {
                Rc::new(group.high_sources.clone())
            } else {
                previous_high
            };
            let low = if low_changed {
                Rc::new(group.low_sources.clone())
            } else {
                previous_low
            };
            *self.group_high.last_mut().unwrap() = Rc::clone(&high);
            *self.group_low.last_mut().unwrap() = Rc::clone(&low);
            *self.group_versions.last_mut().unwrap() += 1;
            let tail = self.group_tail.last_mut().unwrap();
            *tail = tail.append_greater(incoming.source_index);
            (high, low)
        };
        let equal_extreme = high.len() > 1 || low.len() > 1;
        let waiting_reasons = if equal_extreme {
            vec!["equal_extreme_identity"]
        } else {
            vec![]
        };
        let action = if included { "merge" } else { "new_group" };
        if self.record_steps {
            self.full_steps.push(InclusionStepFact {
                action,
                incoming,
                acc_before: Some(acc),
                acc_after: Some(after),
                contains: Some(included),
                direction_evidence: self.evidence.clone(),
                source_coords: full_sources,
                high_sources: high.as_ref().clone(),
                low_sources: low.as_ref().clone(),
                waiting_reasons: waiting_reasons.clone(),
            });
        }
        self.steps.push(StepRecord {
            action,
            incoming,
            acc_before: Some(acc),
            acc_after: Some(after),
            contains: Some(included),
            evidence: self.current_evidence.clone(),
            coords,
            high: Some(high),
            low: Some(low),
            waiting_reasons,
        });
        self.processed += 1;
    }

    /// #1488：按既有单步快照撤回到给定 raw rank，再由 append 重算变化后缀。
    ///
    /// 已建方向且目标不早于建立点：封口只会改前一末组，merge 只会改当前末组，acc_before
    /// 与前一步 high/low 共享快照给出全部逆操作，不扫未变 raw。
    /// #1393：目标早于建立点时，结果是只含 raw[..rank] 的等待期——这些原件之间不可能有
    /// 建立对（否则早已建立），按到达序重放只做逐对 O(1) 判定，不发生折叠。
    ///
    /// 正式 S 的有效账由坐标唯一且升序的 raw winner 索引给出，属于本口的严格递增域。
    /// 非递增的通用 parser 降级域不能取得可回退的共享资格。
    pub fn rewind_to(&mut self, steps: usize) -> Result<(), SharedEvidenceUnavailable> {
        if self.degraded || steps > self.steps.len() {
            return Err(SharedEvidenceUnavailable {
                reason: if self.degraded {
                    "non_increasing_source_coordinates"
                } else {
                    "rewind_rank_out_of_range"
                },
            });
        }
        if self.established_at.is_some_and(|at| steps < at) {
            let bars: Vec<Bar> = self.raw.iter().take(steps).copied().collect();
            *self = Self {
                record_steps: self.record_steps,
                ..Self::empty()
            };
            for bar in bars {
                self.append(bar);
            }
            debug_assert!(self.unsettled());
            return Ok(());
        }
        while self.steps.len() > steps {
            let step = self.steps.pop().expect("已核非空");
            if self.record_steps {
                self.full_steps.pop();
            }
            let incoming = self.raw.pop().expect("每步对应一条 raw");
            debug_assert_eq!(incoming.source_index, step.incoming.source_index);
            self.positions.remove(&incoming.source_index);
            self.ordinal -= 1;
            self.last_source_index = self.steps.last().map(|s| s.incoming.source_index);
            self.raw_prefix = self.raw_prefix.without_last();
            if step.action == "waiting" {
                let unmapped = self.unmapped.pop();
                debug_assert_eq!(unmapped, Some(incoming.source_index));
                let pending = self.pending_raw.pop();
                debug_assert_eq!(pending.map(|b| b.source_index), Some(incoming.source_index));
                continue;
            }
            self.processed -= 1;
            if step.action == "merge" {
                let before = step.acc_before.expect("merge 有旧 acc");
                let after = step.acc_after.expect("merge 有新 acc");
                let previous = self.steps.last().expect("merge 前有 seed");
                let high = previous.high.as_ref().expect("非 waiting 前驱有极值快照");
                let low = previous.low.as_ref().expect("非 waiting 前驱有极值快照");
                *Rc::make_mut(&mut self.merged).last_mut().unwrap() = before;
                let group = self.groups.last_mut().unwrap();
                group.bar = before;
                let member = group.members.pop();
                debug_assert_eq!(member, Some(incoming.source_index));
                if before.high == after.high {
                    group.high_sources.truncate(high.len());
                } else {
                    group.high_sources = high.as_ref().clone();
                }
                if before.low == after.low {
                    group.low_sources.truncate(low.len());
                } else {
                    group.low_sources = low.as_ref().clone();
                }
                *self.group_high.last_mut().unwrap() = Rc::clone(high);
                *self.group_low.last_mut().unwrap() = Rc::clone(low);
                let tail = self.group_tail.last_mut().unwrap();
                *tail = tail.without_last();
                *self.group_versions.last_mut().unwrap() -= 1;
                continue;
            }
            // 目标不早于建立点 ⟹ 只会撤回建立之后的 new_group（seed 与重折步都在建立点之前）。
            debug_assert_eq!(step.action, "new_group");
            Rc::make_mut(&mut self.merged).pop();
            self.groups.pop();
            self.group_tail.pop();
            self.group_direction.pop();
            self.group_confirmation.pop();
            self.group_high.pop();
            self.group_low.pop();
            self.group_versions.pop();
            let group = self.groups.last_mut().expect("seed 组不在撤回范围内");
            group.confirmed = false;
            group.confirmation_evidence = None;
            *self.group_confirmation.last_mut().unwrap() = None;
            *self.group_versions.last_mut().unwrap() -= 1;
            self.evidence = group.direction_evidence.clone();
            self.current_evidence = self.group_direction.last().unwrap().clone();
        }
        self.replayed_from_zero = self.established_at == Some(self.steps.len());
        Ok(())
    }

    pub fn merged_slice(&self) -> &[Bar] {
        &self.merged
    }

    pub fn merged_rc(&self) -> Rc<Vec<Bar>> {
        Rc::clone(&self.merged)
    }

    pub fn groups_slice(&self) -> &[InclusionGroupFact] {
        &self.groups
    }

    pub fn positions(&self) -> &std::collections::BTreeMap<usize, usize> {
        &self.positions
    }

    /// #1393 W：方向尚未建立（等待期，正式组为空）。
    pub fn unsettled(&self) -> bool {
        self.evidence.is_none()
    }

    /// #1393 W：方向建立前的未定原件（按到达序）。
    pub fn pending_raw(&self) -> &[Bar] {
        &self.pending_raw
    }

    /// #1393 W：初始化证据（紧凑模式不展开 `source_coords`）。
    pub fn initial_direction_evidence(&self) -> Option<&InclusionDirectionEvidence> {
        self.initial_evidence.as_ref()
    }

    /// #1393 W：初始化证据的共享节点（`coords` = 建立前缀全部 raw 坐标）。
    pub fn initial_direction_node(&self) -> Option<&Rc<EvidenceNode>> {
        self.initial_node.as_ref()
    }

    /// #1393 W：最近一次 append 是否从 raw0 重折。
    pub fn replayed_from_zero(&self) -> bool {
        self.replayed_from_zero
    }

    /// 跨 bar 稳定的 merged 前缀长度（实际发证）：末根 acc 仍可被下一根改写，其余**一经
    /// 定稿不再改写**（`fold_step` 只 push 已定稿的 acc）。初始只有 seed 时为 0。
    pub fn merged_confirmed_len(&self) -> usize {
        // #1393：重折这一代没有旧正式前缀可复用；下一 append 才能发证。
        if self.unsettled() || self.replayed_from_zero {
            return 0;
        }
        self.merged.len().saturating_sub(1)
    }

    // ── #1394 完整证据访问口：增量路径只取本次真正变化的步/组，不物化全体 ──

    /// 已记录的步骤数（= 全量入口 `steps.len()`）。
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// 当前完整 raw 前缀（到达序坐标）的共享句柄。#1393 W 等待期步只绑定 incoming，
    /// 末步 `coords` 不再是前缀；需要「同 cut 全部生效原始账」的调用方取本句柄。
    /// 退化态（坐标非递增）下返回 `None`，调用方回落全量入口。
    pub fn shared_raw_prefix(&self) -> Option<&CoordsRef> {
        (!self.degraded).then_some(&self.raw_prefix)
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// 组内容版本号（成员/极值/封口任一处变化即 +1）——S 侧稳定对象复用的变更探针。
    pub fn group_version(&self, index: usize) -> Option<u64> {
        self.group_versions.get(index).copied()
    }

    /// 步骤 `index` 的源坐标（= 该步 incoming 的源序位；亦是其事实键）。
    pub fn step_coord(&self, index: usize) -> Option<usize> {
        self.steps.get(index).map(|step| step.incoming.source_index)
    }

    /// 步骤 `index` 的 `waiting_reasons`（廉价读取：S 侧汇总知识状态不物化该步来源集合）。
    pub fn step_waiting_reasons(&self, index: usize) -> Option<&[&'static str]> {
        self.steps
            .get(index)
            .map(|step| step.waiting_reasons.as_slice())
    }

    /// 是否已建立方向证据（= 全量入口 `steps.iter().any(|s| s.direction_evidence.is_some())`）。
    /// 证据只在 `new_group` 分支产生且此后不再清空，故与紧凑状态的 `evidence.is_some()` 等价。
    pub fn has_direction_evidence(&self) -> bool {
        self.evidence.is_some()
    }

    /// 已进入折叠的 raw 数（= 全量入口 `processed`；waiting 分支不计入）。
    pub fn processed(&self) -> usize {
        self.processed
    }

    /// 坐标非严格递增（共享表示失效，展开回落全量入口）。
    pub fn degraded(&self) -> bool {
        self.degraded
    }

    pub fn unmapped_sources(&self) -> &[usize] {
        &self.unmapped
    }

    /// 已处理 raw（按到达序）——供成员读取；O(1) 借出迭代器。
    pub fn raw_iter(&self) -> impl Iterator<Item = &Bar> {
        self.raw.iter()
    }

    /// #1394 5.3：借出单步完整证据的只读视图（不展开旧 Value/坐标集合）。
    ///
    /// - `Ok(Some(view))`：该步存在，视图借自本状态；
    /// - `Ok(None)`：该索引没有步骤（`index >= step_count()`）；
    /// - `Err(SharedEvidenceUnavailable)`：共享态退化，**先于索引判定**——不返回任何
    ///   视图，也不静默改用全量入口的判据。
    ///
    /// 取用视图不改变本状态；既有 [`IncrFactsState::step_fact`] 的读数与回落行为不变。
    pub fn shared_step(
        &self,
        index: usize,
    ) -> Result<Option<StepEvidenceView<'_>>, SharedEvidenceUnavailable> {
        if self.degraded {
            return Err(SharedEvidenceUnavailable {
                reason: "source 坐标非严格递增：共享证据不可用",
            });
        }
        let Some(step) = self.steps.get(index) else {
            return Ok(None);
        };
        Ok(Some(StepEvidenceView {
            action: step.action,
            incoming: step.incoming,
            acc_before: step.acc_before,
            acc_after: step.acc_after,
            contains: step.contains,
            evidence: step.evidence.as_ref(),
            coords: &step.coords,
            high: step.high.as_deref().map(Vec::as_slice).unwrap_or_default(),
            low: step.low.as_deref().map(Vec::as_slice).unwrap_or_default(),
            waiting_reasons: step.waiting_reasons.as_slice(),
        }))
    }

    /// #1394 5.3：借出单组完整证据的只读视图（不展开方向/封口证据的完整来源前缀）。
    ///
    /// 契约与 [`IncrFactsState::shared_step`] 一致：
    /// - `Ok(Some(view))`：该组存在，视图借自本状态；
    /// - `Ok(None)`：该索引没有组（`index >= group_count()`）；
    /// - `Err(SharedEvidenceUnavailable)`：共享态退化，**先于索引判定**——不返回任何视图，
    ///   也不静默改用全量入口的判据。
    ///
    /// 取用视图不改变本状态；既有 [`IncrFactsState::group_fact`] /
    /// [`IncrFactsState::materialize_groups`] 的读数与回落行为不变。
    ///
    /// ## 成员字段的等价性（不另立成员判据）
    ///
    /// `members` 借自增量成员表（`seed`/`new_group` 以 incoming 起表、`merge` 追加），
    /// 与 `group_fact`/`materialize_groups` 现用的 `dense_annotation` 分组**同输入同值**：
    ///
    /// 1. `merge` 保留 `acc.source_index`，而 `merged` 按源坐标严格递增 ⟹ 组锚 = 组内首根
    ///    序号，`merged_group_index` 的「最后一个 ≤ 该坐标的锚」= 落入区间
    ///    `[merged[g].source_index, merged[g+1].source_index)` 的那一组；
    /// 2. 非退化域里每根 raw 恰折叠进一个组（组只在 `new_group` 处前进，成员只增不改），
    ///    故增量成员表与上述区间成员同值；
    /// 3. 全量入口同样只用 `bars[..processed]` 的稠密标注（方向未定前缀之后的 raw 不冒充
    ///    已分组），两边截断口径一致。
    ///
    /// 退化域（源坐标非严格递增）上述前提失效，故直接 `Err`，不用近似映射冒充。
    /// `member_tail` 是同一成员序列去掉首锚后的既有共享尾链（= `members[1..]`），
    /// 不新算成员判据；等价性由逐前缀逐组对拍锁定
    /// （测试 `shared_group_matches_full_facts_for_every_prefix`）。
    pub fn shared_group(
        &self,
        index: usize,
    ) -> Result<Option<GroupEvidenceView<'_>>, SharedEvidenceUnavailable> {
        if self.degraded {
            return Err(SharedEvidenceUnavailable {
                reason: "source 坐标非严格递增：共享证据不可用",
            });
        }
        let Some(group) = self.groups.get(index) else {
            return Ok(None);
        };
        // 组尾链与组在同一分支成对推入（`seed` / `new_group`），长度恒等。
        let member_tail = self
            .group_tail
            .get(index)
            .expect("组尾链与组按序成对维护（seed / new_group 同点推入）");
        Ok(Some(GroupEvidenceView {
            bar: group.bar,
            members: group.members.as_slice(),
            member_tail,
            high_sources: group.high_sources.as_slice(),
            low_sources: group.low_sources.as_slice(),
            confirmed: self.evidence.is_some() && index + 1 < self.groups.len(),
            direction_evidence: self.group_direction.get(index).and_then(|h| h.as_ref()),
            confirmation_evidence: self.group_confirmation.get(index).and_then(|h| h.as_ref()),
        }))
    }

    /// 展开单步完整事实（与全量入口同字段、同顺序）。
    pub fn step_fact(&self, index: usize) -> Option<InclusionStepFact> {
        if self.degraded {
            return process_inclusion_with_facts(&self.raw.to_vec())
                .steps
                .into_iter()
                .nth(index);
        }
        let step = self.steps.get(index)?;
        Some(InclusionStepFact {
            action: step.action,
            incoming: step.incoming,
            acc_before: step.acc_before,
            acc_after: step.acc_after,
            contains: step.contains,
            direction_evidence: step.evidence.as_ref().map(expand_evidence),
            source_coords: step.coords.materialize(),
            high_sources: step
                .high
                .as_ref()
                .map(|v| v.as_ref().clone())
                .unwrap_or_default(),
            low_sources: step
                .low
                .as_ref()
                .map(|v| v.as_ref().clone())
                .unwrap_or_default(),
            waiting_reasons: step.waiting_reasons.clone(),
        })
    }

    /// 展开单组完整事实：`members` 走与全量入口**同一条**稠密标注（不另立第二查法）。
    pub fn group_fact(&self, index: usize) -> Option<InclusionGroupFact> {
        if self.degraded {
            return process_inclusion_with_facts(&self.raw.to_vec())
                .groups
                .into_iter()
                .nth(index);
        }
        let base = self.groups.get(index)?.clone();
        let annotations = self.dense_annotations();
        Some(InclusionGroupFact {
            members: self
                .raw
                .iter()
                .take(self.processed)
                .zip(&annotations)
                .filter(|(_, a)| a.group == Some(index))
                .map(|(b, _)| b.source_index)
                .collect(),
            confirmed: self.evidence.is_some() && index + 1 < self.groups.len(),
            direction_evidence: self
                .group_direction
                .get(index)
                .and_then(|h| h.as_ref())
                .map(expand_evidence),
            confirmation_evidence: self
                .group_confirmation
                .get(index)
                .and_then(|h| h.as_ref())
                .map(expand_evidence),
            ..base
        })
    }

    /// 组 `index` 的 `direction_evidence.source_coords`（不展开全量 facts 的快速口）。
    pub fn group_direction_coords(&self, index: usize) -> Vec<usize> {
        self.group_direction
            .get(index)
            .and_then(|h| h.as_ref())
            .map(|e| e.coords.materialize())
            .unwrap_or_default()
    }

    /// 组 `index` 的 `confirmation_evidence.source_coords`（同上）。
    pub fn group_confirmation_coords(&self, index: usize) -> Vec<usize> {
        self.group_confirmation
            .get(index)
            .and_then(|h| h.as_ref())
            .map(|e| e.coords.materialize())
            .unwrap_or_default()
    }

    /// 完整组证据投影；成员仍由同一 dense_annotation 判定，一次按组分桶。
    /// 本入口不展开步骤历史，不能当成完整 InclusionFacts 使用。
    pub fn materialize_groups(&self) -> Vec<InclusionGroupFact> {
        if self.degraded {
            return process_inclusion_with_facts(&self.raw.to_vec()).groups;
        }
        let annotations = self.dense_annotations();
        let mut members_by_group = vec![Vec::new(); self.groups.len()];
        for (bar, annotation) in self.raw.iter().take(self.processed).zip(&annotations) {
            if let Some(group) = annotation.group {
                members_by_group[group].push(bar.source_index);
            }
        }
        let mut groups: Vec<InclusionGroupFact> = Vec::with_capacity(self.groups.len());
        for (index, base) in self.groups.iter().enumerate() {
            let members = std::mem::take(&mut members_by_group[index]);
            groups.push(InclusionGroupFact {
                bar: base.bar,
                members,
                high_sources: base.high_sources.clone(),
                low_sources: base.low_sources.clone(),
                confirmed: self.evidence.is_some() && index + 1 < self.groups.len(),
                confirmation_evidence: self
                    .group_confirmation
                    .get(index)
                    .and_then(|h| h.as_ref())
                    .map(expand_evidence),
                direction_evidence: self
                    .group_direction
                    .get(index)
                    .and_then(|h| h.as_ref())
                    .map(expand_evidence),
            });
        }
        groups
    }

    /// 展开完整 facts——与 [`process_inclusion_with_facts`] 全部字段逐位相同。
    pub fn materialize_facts(&self) -> InclusionFacts {
        if self.degraded {
            return process_inclusion_with_facts(&self.raw.to_vec());
        }
        let merged = self.merged.to_vec();
        let groups = self.materialize_groups();
        let steps = (0..self.steps.len())
            .filter_map(|index| self.step_fact(index))
            .collect();
        InclusionFacts {
            stroke_facts: None,
            segment_facts: None,
            merged,
            groups,
            steps,
            initial_direction_unsettled: self.unsettled(),
            initial_direction_evidence: self.initial_node.as_ref().map(expand_evidence),
            pending_raw: self.pending_raw.clone(),
            unmapped_sources: self.unmapped.clone(),
        }
    }

    /// 与全量入口同一条稠密标注（`bars[..processed]` × 当前 merged）。
    fn dense_annotations(&self) -> Vec<DenseBarAnnotation> {
        let prefix: Vec<Bar> = self.raw.iter().take(self.processed).cloned().collect();
        dense_annotation(
            &prefix,
            &InclusionResult {
                merged: self.merged.to_vec(),
                only_open_tail: self.evidence.is_none(),
                ..InclusionResult::default()
            },
        )
    }
}

/// 共享证据节点 → 全量 `InclusionDirectionEvidence`（`source_coords` 现场展开）。
fn expand_evidence(node: &Rc<EvidenceNode>) -> InclusionDirectionEvidence {
    InclusionDirectionEvidence {
        direction: node.direction,
        previous_acc: node.previous_acc,
        incoming: node.incoming,
        source_coords: node.coords.materialize(),
    }
}

#[cfg(test)]
#[path = "inclusion_tb02u_tests.rs"]
mod tb02u_tests;

#[cfg(test)]
#[path = "inclusion_wreview_tests.rs"]
mod wreview_tests;

#[cfg(test)]
mod tests {
    use super::super::super::types::Tick;
    use super::*;

    fn bar(i: usize, high: Tick, low: Tick) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: low,
            high,
            low,
            close: high,
            volume: 1.0,
            untradable: false,
        }
    }

    fn assert_delta_prefixes(bars: &[Bar]) {
        let mut state = IncrFactsState::empty();
        for (index, incoming) in bars.iter().enumerate() {
            let before = state.materialize_facts();
            let changes = state.append_with_changes(*incoming);
            let after = state.materialize_facts();
            // 独立旧全量核，不经 append_with_changes 计算 expected。
            assert_eq!(after, process_inclusion_with_facts(&bars[..=index]));
            assert_eq!(changes.step_counts(), (index, index + 1));
            assert_eq!(
                changes.group_counts(),
                (before.groups.len(), after.groups.len())
            );
            assert_eq!(
                changes.merged_counts(),
                (before.merged.len(), after.merged.len())
            );
            let groups = changes.group_dirty_from().unwrap_or(before.groups.len());
            let merged = changes.merged_dirty_from().unwrap_or(before.merged.len());
            assert_eq!(&before.groups[..groups], &after.groups[..groups]);
            assert_eq!(&before.merged[..merged], &after.merged[..merged]);
            // #1393：建立方向那一代从 raw0 重折，步骤如实从 0 起脏；其余只追加一步。
            let steps = changes.step_dirty_from();
            assert_eq!(steps, if changes.replayed_from_zero() { 0 } else { index });
            assert_eq!(&before.steps[..steps], &after.steps[..steps]);
            if changes.group_dirty_from().is_none() {
                assert_eq!(before.groups, after.groups);
            }
            if changes.merged_dirty_from().is_none() {
                assert_eq!(before.merged, after.merged);
            }
            assert!(changes.shared_available());
        }
    }

    #[test]
    fn delta_append_frontiers_match_full_kernel_and_preserve_untouched_prefixes() {
        // 已建方向、包含合并、同极值、转向与再封口；216 个三步延续，每步对完整旧核。
        let ranges = [(12, 7), (11, 8), (14, 6), (9, 4), (16, 11), (10, 5)];
        for a in ranges {
            for b in ranges {
                for c in ranges {
                    assert_delta_prefixes(&[
                        bar(2, 10, 5),
                        bar(7, 12, 7),
                        bar(12, a.0, a.1),
                        bar(17, b.0, b.1),
                        bar(22, c.0, c.1),
                    ]);
                }
            }
        }
        // #1393：建立前的包含原件保持未定，首个 raw 相邻严格对出现时从 raw0 重折。
        assert_delta_prefixes(&[bar(2, 10, 5), bar(7, 9, 6), bar(12, 14, 11)]);
        assert_delta_prefixes(&[bar(2, 10, 5), bar(7, 12, 4), bar(12, 14, 11)]);
    }

    #[test]
    fn delta_append_distinguishes_group_confirmation_merge_and_waiting() {
        let mut state = IncrFactsState::empty();
        let first = state.append_with_changes(bar(2, 10, 5));
        assert_eq!(first.unsettled_transition(), (true, true));
        assert_eq!(first.group_dirty_from(), None);
        assert_eq!(first.merged_dirty_from(), None);
        let establish = state.append_with_changes(bar(7, 12, 7));
        assert!(establish.replayed_from_zero());
        assert_eq!(establish.step_dirty_from(), 0);
        assert_eq!(establish.group_counts(), (0, 2));
        assert_eq!(establish.group_dirty_from(), Some(0));
        assert_eq!(establish.merged_dirty_from(), Some(0));
        assert_eq!(state.merged_confirmed_len(), 0, "重折这一代不发证");
        let merge = state.append_with_changes(bar(12, 11, 8));
        assert_eq!(merge.step_dirty_from(), 2);
        assert_eq!(merge.group_dirty_from(), Some(1));
        assert_eq!(merge.merged_dirty_from(), Some(1));
        assert_eq!(state.merged_confirmed_len(), 1);
        let seal = state.append_with_changes(bar(17, 14, 9));
        assert_eq!(seal.group_dirty_from(), Some(1), "前组封口必须脏");
        assert_eq!(seal.merged_dirty_from(), Some(2), "旧 acc 数值原样定稿");

        let mut waiting = IncrFactsState::empty();
        waiting.append(bar(2, 10, 5));
        let contained = waiting.append_with_changes(bar(7, 9, 6));
        assert_eq!(contained.unsettled_transition(), (true, true));
        assert_eq!(contained.group_dirty_from(), None);
        assert_eq!(waiting.pending_raw().len(), 2);
        assert!(waiting.groups_slice().is_empty());
        let later = waiting.append_with_changes(bar(12, 20, 15));
        assert_eq!(later.unsettled_transition(), (true, false));
        assert_eq!(later.step_counts(), (2, 3));
        assert_eq!(later.step_dirty_from(), 0);
        assert_eq!(later.group_dirty_from(), Some(0));
        assert_eq!(waiting.groups_slice()[0].members, vec![2, 7]);
    }

    #[test]
    fn delta_append_nonincreasing_coordinate_cannot_issue_shared_frontier() {
        let mut state = IncrFactsState::empty();
        state.append(bar(7, 10, 5));
        let changes = state.append_with_changes(bar(7, 12, 7));
        assert!(!changes.shared_available());
        assert!(state.shared_step(0).is_err());
        assert!(!state.append_with_changes(bar(12, 14, 9)).shared_available());
    }

    fn assert_rewind_and_reappend(bars: &[Bar]) {
        for cut in 0..=bars.len() {
            let mut state = IncrFactsState::empty();
            for incoming in bars {
                state.append(*incoming);
            }
            let retained_merged = state.merged_rc();
            let original_merged = retained_merged.as_ref().clone();
            state.rewind_to(cut).unwrap();
            assert_eq!(
                state.materialize_facts(),
                process_inclusion_with_facts(&bars[..cut])
            );
            assert_eq!(state.raw_iter().copied().collect::<Vec<_>>(), bars[..cut]);
            assert_eq!(state.ordinal, cut);
            assert_eq!(state.positions.len(), cut);
            assert_eq!(
                state.raw_prefix.materialize(),
                bars[..cut]
                    .iter()
                    .map(|bar| bar.source_index)
                    .collect::<Vec<_>>()
            );
            let mut prefix = IncrFactsState::empty();
            for incoming in &bars[..cut] {
                prefix.append(*incoming);
            }
            assert_eq!(state.group_versions, prefix.group_versions);
            assert_eq!(state.processed(), prefix.processed());
            assert_eq!(state.unmapped_sources(), prefix.unmapped_sources());
            for (offset, incoming) in bars[cut..].iter().enumerate() {
                state.append_with_changes(*incoming);
                assert_eq!(
                    state.materialize_facts(),
                    process_inclusion_with_facts(&bars[..=cut + offset])
                );
            }
            assert_eq!(
                *retained_merged, original_merged,
                "已借旧 merged 快照不能被撤回改写"
            );
        }
    }

    #[test]
    fn delta_rewind_every_cut_restores_metadata_and_matches_full_kernel() {
        for bars in [
            vec![
                bar(2, 10, 5),
                bar(7, 12, 7),
                bar(12, 12, 8),
                bar(17, 13, 8),
                bar(22, 16, 12),
                bar(27, 14, 10),
                bar(32, 13, 11),
            ],
            vec![
                bar(2, 12, 7),
                bar(7, 10, 5),
                bar(12, 9, 5),
                bar(17, 9, 4),
                bar(22, 12, 6),
                bar(27, 11, 7),
            ],
            vec![
                bar(2, 10, 5),
                bar(7, 9, 6),
                bar(12, 14, 11),
                bar(17, 15, 12),
            ],
            vec![bar(2, 10, 5)],
            vec![],
        ] {
            assert_rewind_and_reappend(&bars);
        }
    }

    #[test]
    fn delta_rewind_revision_can_change_initial_direction_and_equal_extreme_sources() {
        let original = [
            bar(2, 10, 5),
            bar(7, 9, 6),
            bar(12, 14, 11),
            bar(17, 15, 12),
        ];
        for replacement in [bar(7, 12, 7), bar(7, 8, 3), bar(7, 10, 5)] {
            let mut state = IncrFactsState::empty();
            for incoming in original {
                state.append(incoming);
            }
            state.rewind_to(1).unwrap();
            let revised = [original[0], replacement, original[2], original[3]];
            for (index, incoming) in revised.iter().enumerate().skip(1) {
                state.append_with_changes(*incoming);
                assert_eq!(
                    state.materialize_facts(),
                    process_inclusion_with_facts(&revised[..=index])
                );
            }
        }
        // 极值重合后真正换高/低值，撤回须恢复旧来源快照，而非只恢复数量。
        assert_rewind_and_reappend(&[
            bar(2, 10, 5),
            bar(7, 12, 7),
            bar(12, 12, 8),
            bar(17, 12, 9),
            bar(22, 13, 8),
            bar(27, 13, 10),
        ]);
    }

    #[test]
    fn delta_rewind_releases_removed_evidence_and_rejects_invalid_rank_atomically() {
        let mut state = IncrFactsState::empty();
        for incoming in [bar(2, 10, 5), bar(7, 12, 7), bar(12, 14, 9)] {
            state.append(incoming);
        }
        let node = state.current_evidence.as_ref().unwrap().clone();
        let weak = Rc::downgrade(&node);
        let contents = node.coords.materialize();
        let before = state.materialize_facts();
        assert!(state.rewind_to(4).is_err());
        assert_eq!(state.materialize_facts(), before);
        state.rewind_to(0).unwrap();
        assert_eq!(node.coords.materialize(), contents);
        assert_eq!(weak.strong_count(), 1, "只剩测试显式保留的快照");
        drop(node);
        assert!(weak.upgrade().is_none());
        assert_eq!(state.materialize_facts(), process_inclusion_with_facts(&[]));

        state.append(bar(7, 10, 5));
        state.append(bar(7, 12, 7));
        let degraded = state.materialize_facts();
        assert!(state.rewind_to(0).is_err());
        assert_eq!(state.materialize_facts(), degraded);
    }

    #[test]
    fn tb02a_trace_matches_existing_kernel_and_preserves_actual_roots() {
        for (bars, high, low) in [
            (
                vec![
                    bar(0, 10, 5),
                    bar(3, 12, 7),
                    bar(8, 11, 8),
                    bar(14, 13, 6),
                    bar(20, 16, 14),
                ],
                vec![14],
                vec![8],
            ),
            (
                vec![
                    bar(0, 12, 7),
                    bar(3, 10, 5),
                    bar(8, 9, 6),
                    bar(14, 11, 4),
                    bar(20, 3, 1),
                ],
                vec![8],
                vec![14],
            ),
        ] {
            let facts = process_inclusion_with_facts(&bars);
            assert_eq!(facts.merged, process_inclusion(&bars).merged);
            let group = &facts.groups[1];
            assert_eq!(group.members, vec![3, 8, 14]);
            assert_eq!(group.bar.source_index, 3);
            assert_eq!(group.high_sources, high);
            assert_eq!(group.low_sources, low);
            assert!(group.confirmed);
            let confirmation = group.confirmation_evidence.as_ref().unwrap();
            assert_eq!(confirmation.incoming.source_index, 20);
            assert!(confirmation.source_coords.contains(&20));
            assert_eq!(facts.groups.last().unwrap().confirmation_evidence, None);
        }
    }

    #[test]
    fn tb02u_initial_inclusion_waits_then_replays_at_current_cut() {
        let bars = vec![bar(0, 5, 1), bar(1, 4, 2), bar(2, 6, 3)];
        let old = process_inclusion_with_facts(&bars[..2]);
        assert!(old.initial_direction_unsettled);
        assert!(old.merged.is_empty() && old.groups.is_empty());
        assert_eq!(old.pending_raw, bars[..2]);
        let facts = process_inclusion_with_facts(&bars);
        assert!(!facts.initial_direction_unsettled);
        assert_eq!(facts.merged, process_inclusion(&bars).merged);
        assert_eq!(facts.groups[0].members, vec![0, 1]);
        assert!(facts.pending_raw.is_empty());
        assert!(old
            .steps
            .iter()
            .all(|s| s.action == "waiting" && s.acc_after.is_none()));
    }

    #[test]
    fn tb02a_direction_is_from_actual_accumulator_not_previous_raw() {
        let bars = vec![
            bar(0, 20, 10),
            bar(1, 24, 14),
            bar(2, 22, 16),
            bar(3, 23, 15),
        ];
        let facts = process_inclusion_with_facts(&bars);
        assert_eq!(facts.merged, process_inclusion(&bars).merged);
        assert!(contains(&bars[2], &bars[3]));
        let last = facts.steps.last().unwrap();
        assert_eq!(last.contains, Some(false));
        let e = last.direction_evidence.as_ref().unwrap();
        assert_eq!(e.direction, Direction::Down);
        assert_eq!((e.previous_acc.low, e.previous_acc.high), (16, 24));
        assert_eq!(e.source_coords, vec![0, 1, 2, 3]);
    }

    #[test]
    fn tb02a_equal_role_roots_wait_but_cross_axis_equality_does_not() {
        let facts = process_inclusion_with_facts(&[bar(0, 4, 1), bar(1, 6, 2), bar(2, 6, 3)]);
        assert_eq!(facts.groups[1].high_sources, vec![1, 2]);
        assert_eq!(
            facts.steps.last().unwrap().waiting_reasons,
            vec!["equal_extreme_identity"]
        );
        let cross = process_inclusion_with_facts(&[bar(0, 3, 1), bar(1, 6, 3)]);
        assert!(!cross.initial_direction_unsettled);
        assert_eq!(cross.steps[1].contains, Some(false));
        assert!(cross.steps[1].waiting_reasons.is_empty());
    }

    // -------- T1 (#170) 组锚供给线测试（先红后绿） --------
    //
    // 组锚 = 组内首根序号（merged bar 保留组内首根 `source_index`，本文件 :23/:57；
    // 教义 ADR adr-chain-confirmation-and-invariant-identity-20260722 裁定 3）。

    #[test]
    fn group_anchor_is_first_bar_of_merged_group() {
        // 分组：g0={0}；g1={1,2}（C 含于 B 并入）；g2={3}；g3={4}。
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 12, 7),
            bar(2, 11, 8),
            bar(3, 15, 13),
            bar(4, 14, 9),
        ];
        let r = process_inclusion(&bars);
        assert_eq!(
            r.merged.iter().map(|b| b.source_index).collect::<Vec<_>>(),
            vec![0, 1, 3, 4],
            "夹具分组前提（锚 = 各组首根序号）"
        );
        // 组内任一根 → 组锚；组锚自身幂等。
        assert_eq!(merged_group_anchor(&r.merged, 0), Some(0));
        assert_eq!(merged_group_anchor(&r.merged, 1), Some(1));
        assert_eq!(
            merged_group_anchor(&r.merged, 2),
            Some(1),
            "组内后续根映射回组内首根"
        );
        assert_eq!(merged_group_anchor(&r.merged, 3), Some(3));
        assert_eq!(merged_group_anchor(&r.merged, 4), Some(4));
        // 末组之后（尾部仍可生长）→ 末组锚。
        assert_eq!(
            merged_group_anchor(&r.merged, 5),
            Some(4),
            "末组覆盖 [src_last, ∞)"
        );
        // 空包含层 ⟹ None（诚实无锚）。
        assert_eq!(merged_group_anchor(&[], 0), None);
    }

    // -------- S9 编号域统一：稠密标注载体（bit-exact 对拍验收门） --------

    /// 组号供给线：原始序号 → 组号（merged 下标），与组锚同源互指。
    #[test]
    fn merged_group_index_matches_anchor() {
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 12, 7),
            bar(2, 11, 8),
            bar(3, 15, 13),
            bar(4, 14, 9),
        ];
        let r = process_inclusion(&bars);
        // merged 锚 = [0, 1, 3, 4]。
        assert_eq!(merged_group_index(&r.merged, 0), Some(0));
        assert_eq!(merged_group_index(&r.merged, 1), Some(1));
        assert_eq!(merged_group_index(&r.merged, 2), Some(1));
        assert_eq!(merged_group_index(&r.merged, 3), Some(2));
        assert_eq!(merged_group_index(&r.merged, 4), Some(3));
        // 末组之后（尾部仍可生长）→ 末组。
        assert_eq!(merged_group_index(&r.merged, 5), Some(3));
        // 空包含层 ⟹ None。
        assert_eq!(merged_group_index(&[], 0), None);
        // 组号 ↔ 组锚 互指一致。
        for i in 0..5 {
            let g = merged_group_index(&r.merged, i).unwrap();
            let anchor = merged_group_anchor(&r.merged, i).unwrap();
            assert_eq!(r.merged[g].source_index, anchor);
        }
    }

    /// 稠密标注的组号、组标准化高低、组内逐根高低与已确认/暂定位的语义。
    #[test]
    fn dense_annotation_group_and_confirmed_semantics() {
        // 分组：g0={0}；g1={1,2}；g2={3}；g3={4}。
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 12, 7),
            bar(2, 11, 8),
            bar(3, 15, 13),
            bar(4, 14, 9),
        ];
        let r = process_inclusion(&bars);
        assert_eq!(
            r.merged.iter().map(|b| b.source_index).collect::<Vec<_>>(),
            vec![0, 1, 3, 4]
        );
        let ann = dense_annotation(&bars, &r);
        assert_eq!(ann.len(), bars.len());
        assert_eq!(
            ann.iter().map(|a| a.group).collect::<Vec<_>>(),
            vec![Some(0), Some(1), Some(1), Some(2), Some(3)]
        );
        // 组 1 标准化高低 = bar1、bar2 向上合并 = [12, 8]。
        assert_eq!((ann[1].group_high, ann[1].group_low), (Some(12), Some(8)));
        assert_eq!((ann[2].group_high, ann[2].group_low), (Some(12), Some(8)));
        // 组内逐根高低：组 1 内两根本身的高/低（供「组内 high 最大的那根」判据读数）。
        assert_eq!((ann[1].high, ann[1].low), (12, 7));
        assert_eq!((ann[2].high, ann[2].low), (11, 8));
        // 已确认／暂定：前缀组（组号 0..2）确认，末组（组号 3）暂定。
        assert!(ann[0].confirmed && ann[1].confirmed && ann[2].confirmed && ann[3].confirmed);
        assert!(!ann[4].confirmed);
    }

    /// #1393：未定原件不冒充正式组，数值与组号全部为 None。
    #[test]
    fn dense_annotation_open_tail_all_tentative() {
        let bars = vec![bar(0, 20, 1), bar(1, 10, 5), bar(2, 8, 6)];
        let r = process_inclusion(&bars);
        assert!(r.only_open_tail);
        let ann = dense_annotation(&bars, &r);
        assert_eq!(
            ann.iter().map(|a| a.group).collect::<Vec<_>>(),
            vec![None, None, None]
        );
        assert!(ann
            .iter()
            .all(|a| !a.confirmed && a.group_high.is_none() && a.group_low.is_none()));
        // 单根与空输入。
        assert!(dense_annotation(&[], &process_inclusion(&[])).is_empty());
        let one = vec![bar(0, 10, 5)];
        let r1 = process_inclusion(&one);
        let ann1 = dense_annotation(&one, &r1);
        assert_eq!(ann1.len(), 1);
        assert!(!ann1[0].confirmed, "单根无方向 = 暂定");
    }

    /// 从稠密标注 + 原始 K 重建合并序列（bit-exact 对拍验收门）。
    fn reconstruct_merged(bars: &[Bar], ann: &[DenseBarAnnotation]) -> Vec<Bar> {
        assert_eq!(bars.len(), ann.len());
        let Some(last_group) = ann.iter().filter_map(|a| a.group).max() else {
            return Vec::new();
        };
        let num_groups = last_group + 1;
        let mut merged = Vec::with_capacity(num_groups);
        for g in 0..num_groups {
            let members: Vec<usize> = (0..bars.len())
                .filter(|&i| ann[i].group == Some(g))
                .collect();
            let first = members[0];
            let last = members[members.len() - 1];
            // 与 `merge()` 同一左折叠顺序累加 volume，逐字段 bit-exact。
            let mut volume = bars[first].volume;
            for &i in &members[1..] {
                volume = volume + bars[i].volume;
            }
            merged.push(Bar {
                source_index: bars[first].source_index,
                timestamp: bars[first].timestamp,
                open: bars[first].open,
                high: ann[first].group_high.unwrap(),
                low: ann[first].group_low.unwrap(),
                close: bars[last].close,
                volume,
                untradable: members.iter().any(|&i| bars[i].untradable),
            });
        }
        merged
    }

    #[test]
    fn dense_annotation_roundtrip_bit_exact() {
        // 各种合成序列：空、单根、open-tail、定方向合并、相 A→B 迁移、长合成、方向翻转。
        let mut sequences: Vec<Vec<Bar>> = vec![
            vec![],
            vec![bar(0, 10, 5)],
            vec![bar(0, 10, 5), bar(1, 8, 6)], // 互含 → open-tail
            vec![bar(0, 10, 5), bar(1, 12, 7), bar(2, 11, 8)], // 定方向 + 合并
            vec![bar(0, 20, 1), bar(1, 18, 3), bar(2, 25, 10), bar(3, 22, 12)], // 相 A→B
            vec![
                bar(0, 10, 5),
                bar(1, 15, 8),
                bar(2, 12, 9),
                bar(3, 8, 4),
                bar(4, 9, 5),
                bar(5, 14, 10),
                bar(6, 13, 11),
            ], // 多次方向翻转 + 合并
        ];
        sequences.push(
            (0..300usize)
                .map(|i| {
                    let base = 100i64 + (i as i64) * 3;
                    let cycle = ((i as f64) / 17.0).sin() as i64 * 40;
                    let close = base + cycle;
                    bar(i, close + 6, close - 6)
                })
                .collect(),
        );
        for bars in &sequences {
            let r = process_inclusion(bars);
            let ann = dense_annotation(bars, &r);
            assert_eq!(ann.len(), bars.len(), "标注长度须与原始 K 等长");
            let rebuilt = reconstruct_merged(bars, &ann);
            assert_eq!(
                rebuilt, r.merged,
                "稠密标注重建的合并序列 != 现役 process_inclusion（bit-exact 破裂）"
            );
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
        assert_eq!(
            strict_dir(&bar(0, 10, 5), &bar(1, 12, 7)),
            Some(Direction::Up)
        );
        assert_eq!(
            strict_dir(&bar(0, 10, 5), &bar(1, 8, 3)),
            Some(Direction::Down)
        );
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
            assert_eq!(
                incr.to_result(),
                full,
                "from_result 恢复后追加 bar {} 不匹配",
                k
            );
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // #1394：完整证据增量状态 —— 每个前缀与全量入口**全部字段逐位对拍**。
    //
    // 对拍面：merged / groups（含 members、high/low 实际来源集合、confirmed、
    // confirmation_evidence 与 direction_evidence 的 source_coords 顺序）/
    // steps（action、incoming、acc_before/after、contains、证据前驱、实际源集合、
    // high/low 快照、waiting_reasons）/ initial_direction_unsettled / unmapped_sources。
    // ────────────────────────────────────────────────────────────────────────

    fn assert_per_prefix_bit_exact(bars: &[Bar], label: &str) {
        let mut state = IncrFactsState::empty();
        for i in 0..bars.len() {
            state.append(bars[i]);
            if state.degraded() {
                // 退化域（坐标非严格递增）也必须逐位相等——展开回落全量入口。
                assert_eq!(
                    state.materialize_facts(),
                    process_inclusion_with_facts(&bars[..=i]),
                    "{label} 前缀 {i} 退化展开不匹配"
                );
                continue;
            }
            let want = process_inclusion_with_facts(&bars[..=i]);
            let got = state.materialize_facts();
            assert_eq!(got, want, "{label} 前缀 {i} 全字段不匹配");
            assert_eq!(got.merged, want.merged, "{label} 前缀 {i} merged");
            for (k, want_group) in want.groups.iter().enumerate() {
                assert_eq!(
                    state.group_fact(k).as_ref(),
                    Some(want_group),
                    "{label} 前缀 {i} 组 {k} 单组展开不匹配"
                );
                assert_eq!(
                    state.group_direction_coords(k),
                    want_group
                        .direction_evidence
                        .as_ref()
                        .map(|e| e.source_coords.clone())
                        .unwrap_or_default(),
                    "{label} 前缀 {i} 组 {k} 方向证据来源不匹配"
                );
                assert_eq!(
                    state.group_confirmation_coords(k),
                    want_group
                        .confirmation_evidence
                        .as_ref()
                        .map(|e| e.source_coords.clone())
                        .unwrap_or_default(),
                    "{label} 前缀 {i} 组 {k} 封口证据来源不匹配"
                );
            }
            for (k, want_step) in want.steps.iter().enumerate() {
                assert_eq!(
                    state.step_fact(k).as_ref(),
                    Some(want_step),
                    "{label} 前缀 {i} 步 {k} 单步展开不匹配"
                );
            }
        }
    }

    /// 非连续但严格递增的源坐标（S 的真实域：按源坐标序取有效事件）。
    fn sparse_bar(coord: usize, high: Tick, low: Tick) -> Bar {
        Bar {
            source_index: coord,
            timestamp: coord as i64 * 10,
            open: low,
            high,
            low,
            close: high,
            volume: 1.0,
            untradable: false,
        }
    }

    #[test]
    fn completeness_matches_full_on_append_and_contains_merge() {
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 15, 8),
            bar(2, 14, 9), // 含于 1 → merge
            bar(3, 9, 4),
            bar(4, 10, 5), // 含于 3 → merge
            bar(5, 18, 9),
            bar(6, 17, 10),
        ];
        assert_per_prefix_bit_exact(&bars, "append/merge");
    }

    #[test]
    fn completeness_matches_full_on_equal_extremes() {
        // 同价极值：组成员与极值来源出现身份竞争（waiting_reasons=equal_extreme_identity）。
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 15, 8),
            bar(2, 15, 6), // high 同价合并 → high_sources 竞争
            bar(3, 12, 6), // low 同价
            bar(4, 9, 4),
            bar(5, 9, 3),
        ];
        assert_per_prefix_bit_exact(&bars, "equal-extremes");
    }

    #[test]
    fn completeness_matches_full_on_initial_direction_unsettled() {
        // 开头互含、方向永久未定：后续 raw 全部 waiting，unmapped_sources 逐根累积。
        let bars = vec![
            bar(0, 10, 5),
            bar(1, 9, 6), // 含于 0，且此前无方向 → 永久未定
            bar(2, 12, 7),
            bar(3, 13, 8),
        ];
        assert_per_prefix_bit_exact(&bars, "initial-unsettled");
    }

    #[test]
    fn completeness_matches_full_on_sparse_source_coords() {
        // 非连续源坐标（10/11/37/…）：不得区间化，展开必须原样保留稀疏坐标。
        let bars = vec![
            sparse_bar(10, 10, 5),
            sparse_bar(11, 15, 8),
            sparse_bar(37, 14, 9),
            sparse_bar(9000, 9, 4),
            sparse_bar(9001, 10, 5),
            sparse_bar(12345, 18, 9),
        ];
        assert_per_prefix_bit_exact(&bars, "sparse-coords");
        let facts = process_inclusion_with_facts(&bars);
        assert!(facts
            .steps
            .iter()
            .all(|s| s.source_coords.iter().all(|c| *c == 10
                || *c == 11
                || *c == 37
                || *c == 9000
                || *c == 9001
                || *c == 12345)));
    }

    #[test]
    fn completeness_falls_back_on_non_monotonic_coords() {
        // 裸调用方给出非单调坐标：共享表示失效，展开回落全量入口（不得近似）。
        let bars = vec![bar(5, 10, 5), bar(3, 15, 8), bar(9, 14, 9), bar(1, 9, 4)];
        let mut state = IncrFactsState::empty();
        for b in &bars {
            state.append(*b);
        }
        assert!(state.degraded());
        assert_eq!(
            state.materialize_facts(),
            process_inclusion_with_facts(&bars)
        );
    }

    #[test]
    fn completeness_matches_full_on_randomized_prefixes() {
        // 确定性 LCG 合成 600 根，混合包含/严格/方向翻转/同价；逐前缀全字段对拍。
        let mut lcg: u64 = 0x2545_F491_4F6C_DD1D;
        let mut rand = move || {
            lcg = lcg
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (lcg >> 33) as i64
        };
        let mut high = 100i64;
        let bars: Vec<Bar> = (0..600usize)
            .map(|i| {
                let step = rand() % 25 - 12;
                high = (high + step).clamp(10, 400);
                let span = 1 + rand() % 9;
                let tie = rand() % 7 == 0;
                let h = if tie { high } else { high + rand() % 3 };
                bar(i * 2 + 1, h, h - span) // 稀疏（步长 2）+ 非连续
            })
            .collect();
        assert_per_prefix_bit_exact(&bars, "randomized");
    }

    #[test]
    fn completeness_appends_9011_bars_without_recursion_or_prefix_copy() {
        // 深链量级验收：9011 根全追加 + 一次全展开。递归实现会栈溢出；逐追加全复制会 O(n²)。
        let bars: Vec<Bar> = (0..9011usize)
            .map(|i| {
                let base = 1000 + (i as i64) * 2;
                let wave = ((i % 23) as i64 - 11) * 5;
                bar(i, base + wave + 8, base + wave - 8)
            })
            .collect();
        let mut state = IncrFactsState::empty();
        let start = std::time::Instant::now();
        for b in &bars {
            state.append(*b);
        }
        let append_elapsed = start.elapsed();
        assert_eq!(state.step_count(), 9011);
        let facts = state.materialize_facts();
        assert_eq!(facts.steps.len(), 9011);
        assert!(!state.degraded());
        // 追加阶段必须是 O(n)（不是 O(n²)）：给一个宽上限，只在量级回归时失败。
        assert!(
            append_elapsed.as_secs_f64() < 20.0,
            "9011 根增量追加耗时 {append_elapsed:?}（量级回归）"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // #1394 5.3：同核共享证据的**只读借用口**（直接成值边，不展开旧 Value/坐标）。
    //
    // 契约：`shared_step(index)` 借出单步完整证据视图——字段借用共享句柄与切片，
    // 不物化坐标/来源集合；索引不存在 = `Ok(None)`；共享态退化 = 显式 `Err`，
    // 不静默回落成另一套判据，也不无声返回缺对象。
    // ────────────────────────────────────────────────────────────────────────

    /// 手工遍历共享坐标树（借用口暴露的物理形状）：叶序 = 升序去重集合。
    fn collect_shared_coords(node: &super::super::evidence::CoordNode) -> Vec<usize> {
        use super::super::evidence::CoordNode;
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            match current {
                CoordNode::Leaf(coord) => out.push(*coord),
                CoordNode::Concat(left, right, _) => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        out
    }

    /// 逐字段对拍：借用视图 ≡ 同一状态 `step_fact` 的完整展开。
    fn assert_shared_views_match_step_facts(bars: &[Bar], label: &str) {
        let mut state = IncrFactsState::empty();
        for bar in bars {
            state.append(*bar);
        }
        assert!(!state.degraded(), "{label}：夹具不应退化");
        let want = process_inclusion_with_facts(bars);
        assert!(!want.steps.is_empty(), "{label}：夹具不应为空");
        assert_eq!(state.step_count(), want.steps.len(), "{label}：步数");
        for (index, want_step) in want.steps.iter().enumerate() {
            let fact = state.step_fact(index).expect("步骤在域内");
            assert_eq!(
                &fact, want_step,
                "{label}：步 {index} 既有 step_fact 与全量入口不一致"
            );
            let view = match state.shared_step(index) {
                Ok(Some(view)) => view,
                Ok(None) => panic!("{label}：步 {index} 在域内，借用口却报缺失"),
                Err(error) => panic!("{label}：步 {index} 借用口显式不可用：{error:?}"),
            };
            // 借用视图逐字段 ≡ 既有完整输出（`step_fact`）。
            assert_eq!(view.action, fact.action, "{label}：步 {index} action");
            assert_eq!(view.incoming, fact.incoming, "{label}：步 {index} incoming");
            assert_eq!(
                view.acc_before, fact.acc_before,
                "{label}：步 {index} acc_before"
            );
            assert_eq!(
                view.acc_after, fact.acc_after,
                "{label}：步 {index} acc_after"
            );
            assert_eq!(view.contains, fact.contains, "{label}：步 {index} contains");
            match (view.evidence, fact.direction_evidence.as_ref()) {
                (None, None) => {}
                (Some(shared), Some(full)) => {
                    assert_eq!(
                        shared.direction, full.direction,
                        "{label}：步 {index} 证据方向"
                    );
                    assert_eq!(
                        shared.previous_acc, full.previous_acc,
                        "{label}：步 {index} 证据前驱"
                    );
                    assert_eq!(
                        shared.incoming, full.incoming,
                        "{label}：步 {index} 证据 incoming"
                    );
                    assert_eq!(
                        shared.coords.materialize(),
                        full.source_coords,
                        "{label}：步 {index} 证据来源坐标"
                    );
                }
                (shared, full) => panic!(
                    "{label}：步 {index} 方向证据有无不一致（借用 {shared:?} / 全量 {full:?}）"
                ),
            }
            assert_eq!(
                view.coords.materialize(),
                fact.source_coords,
                "{label}：步 {index} 来源坐标"
            );
            assert_eq!(
                view.high,
                fact.high_sources.as_slice(),
                "{label}：步 {index} high 来源"
            );
            assert_eq!(
                view.low,
                fact.low_sources.as_slice(),
                "{label}：步 {index} low 来源"
            );
            assert_eq!(
                view.waiting_reasons,
                fact.waiting_reasons.as_slice(),
                "{label}：步 {index} 等待原因"
            );
        }
        // 只读借用不改变状态：同一状态的全量展开与借用前逐位相同。
        assert_eq!(
            state.materialize_facts(),
            want,
            "{label}：借用读取改动了状态"
        );
    }

    #[test]
    fn shared_step_matches_full_facts_for_every_step() {
        // 合并 / 定方向 / 换组（方向翻转）：含方向证据与共享坐标前缀。
        assert_shared_views_match_step_facts(
            &[
                bar(0, 10, 5),
                bar(1, 15, 8),
                bar(2, 12, 9),
                bar(3, 8, 4),
                bar(4, 9, 5),
                bar(5, 14, 10),
                bar(6, 13, 11),
            ],
            "merge/new-group",
        );
        // 初始方向永久未定：后续步骤全部 waiting（无证据、无极值快照）。
        assert_shared_views_match_step_facts(
            &[bar(0, 10, 5), bar(1, 9, 6), bar(2, 12, 7), bar(3, 13, 8)],
            "initial-unsettled",
        );
        // 非连续源坐标（不得区间化）+ 同价极值身份竞争。
        assert_shared_views_match_step_facts(
            &[
                sparse_bar(10, 10, 5),
                sparse_bar(11, 15, 8),
                sparse_bar(37, 15, 6),
                sparse_bar(9000, 12, 6),
                sparse_bar(9001, 9, 4),
                sparse_bar(12345, 9, 3),
            ],
            "sparse/equal-extremes",
        );
    }

    #[test]
    fn shared_step_handles_survive_later_appends() {
        let mut state = IncrFactsState::empty();
        for bar in [
            bar(0, 20, 10),
            bar(1, 24, 14),
            bar(2, 22, 16),
            bar(3, 23, 15),
        ] {
            state.append(bar);
        }
        assert!(!state.degraded());
        let index = state.step_count() - 1;
        let before = state.step_fact(index).expect("末步在域内");
        // 只从借用口取 O(1) 共享句柄（不展开历史前缀）。
        let (coords_handle, root, evidence_handle) = {
            let view = state
                .shared_step(index)
                .expect("共享态可用")
                .expect("步在域内");
            (
                view.coords.clone(),
                view.coords.root_node().map(Rc::clone),
                view.evidence.map(Rc::clone),
            )
        };
        let root = root.expect("本步来源集合非空");
        let coords_before = coords_handle.materialize();
        let root_leaves_before = collect_shared_coords(&root);
        assert_eq!(root_leaves_before, coords_before);
        let evidence_handle = evidence_handle.expect("本步应带方向证据");
        let evidence_before = evidence_handle.coords.materialize();

        // 实际追加：后续步骤继续推进合并与证据链。
        for bar in [bar(4, 26, 18), bar(5, 28, 20), bar(6, 27, 21)] {
            state.append(bar);
        }
        assert!(!state.degraded());
        assert!(state.step_count() > index + 1);

        // 旧句柄保持旧内容：共享前缀不被后续追加改写。
        assert_eq!(
            coords_handle.materialize(),
            coords_before,
            "旧坐标句柄被后续追加改写"
        );
        assert_eq!(
            collect_shared_coords(&root),
            root_leaves_before,
            "旧根节点被后续追加改写"
        );
        assert_eq!(
            evidence_handle.coords.materialize(),
            evidence_before,
            "旧证据节点被后续追加改写"
        );
        // 旧步的完整展开与追加前逐位相同。
        assert_eq!(state.step_fact(index), Some(before));
        // 重新借出的同一步与旧句柄同内容。
        let again = state
            .shared_step(index)
            .expect("共享态可用")
            .expect("步在域内");
        assert_eq!(again.coords.materialize(), coords_before);
        assert_eq!(
            collect_shared_coords(again.coords.root_node().expect("非空")),
            root_leaves_before
        );
    }

    #[test]
    fn shared_step_contract_distinguishes_absent_and_degraded() {
        let bars = vec![bar(0, 10, 5), bar(1, 15, 8), bar(2, 14, 9)];
        let mut state = IncrFactsState::empty();
        for bar in &bars {
            state.append(*bar);
        }
        assert!(!state.degraded());
        assert!(
            matches!(state.shared_step(0), Ok(Some(_))),
            "在域内必须借出视图"
        );
        assert!(
            matches!(state.shared_step(state.step_count()), Ok(None)),
            "索引不存在 = Ok(None)"
        );
        assert!(matches!(state.shared_step(usize::MAX), Ok(None)));

        // 共享态退化（source 坐标非严格递增）= 显式错误：不静默回落、不无声缺对象。
        let degraded_bars = vec![bar(5, 10, 5), bar(3, 15, 8), bar(9, 14, 9), bar(1, 9, 4)];
        let mut degraded = IncrFactsState::empty();
        for bar in &degraded_bars {
            degraded.append(*bar);
        }
        assert!(degraded.degraded());
        let error = degraded
            .shared_step(0)
            .expect_err("退化态必须显式报错，不得静默降级");
        assert!(
            error.reason.contains("严格递增"),
            "错误须点名成因，实际：{}",
            error.reason
        );
        assert!(
            degraded.shared_step(usize::MAX).is_err(),
            "退化态先于索引判定"
        );
        // 既有展开入口不受新接口影响：仍按既定契约回落全量入口。
        assert_eq!(
            degraded.step_fact(0),
            process_inclusion_with_facts(&degraded_bars)
                .steps
                .first()
                .cloned()
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // #1394 5.3：同核共享**组**证据的只读借用口（CC005/006/007 直接内容构造的值边）。
    //
    // 契约：`shared_group(index)` 借出单组完整证据视图——`bar`/成员/极值来源借用既有集合
    // 切片，方向与封口证据借出共享 `EvidenceNode` 句柄（坐标树不展开历史前缀）；索引不存在
    // = `Ok(None)`；共享态退化 = 显式 `Err` 且**先于索引判定**（与 `shared_step` 同款），
    // 不静默回落成另一套判据，也不无声返回缺对象。
    //
    // 成员**不另立判据**：视图借的是增量成员表，等价性依据见 `IncrFactsState::shared_group`
    // 文档；下面逐前缀同时与 `group_fact`（dense_annotation 映射）和全量入口对拍。
    // ────────────────────────────────────────────────────────────────────────

    /// 逐字段对拍：组借用视图 ≡ 全量入口 `groups[g]` ≡ 同一状态 `group_fact(g)`。
    fn assert_shared_group_views_match_facts(bars: &[Bar], label: &str) {
        let mut state = IncrFactsState::empty();
        for i in 0..bars.len() {
            state.append(bars[i]);
            assert!(!state.degraded(), "{label} 前缀 {i}：夹具不应退化");
            let want = process_inclusion_with_facts(&bars[..=i]);
            assert_eq!(
                state.group_count(),
                want.groups.len(),
                "{label} 前缀 {i}：组数"
            );
            for (index, want_group) in want.groups.iter().enumerate() {
                let fact = state.group_fact(index).expect("组在域内");
                assert_eq!(
                    &fact, want_group,
                    "{label} 前缀 {i} 组 {index}：既有 group_fact 与全量入口不一致"
                );
                let view = match state.shared_group(index) {
                    Ok(Some(view)) => view,
                    Ok(None) => panic!("{label} 前缀 {i} 组 {index}：在域内，借用口却报缺失"),
                    Err(error) => {
                        panic!("{label} 前缀 {i} 组 {index}：借用口显式不可用 {error:?}")
                    }
                };
                // 借用视图逐字段 ≡ 既有完整输出（`group_fact`）。其中 `members` 一栏即
                // 「借用增量成员表 ≡ dense_annotation 映射」的逐前缀证据。
                assert_eq!(view.bar, fact.bar, "{label} 前缀 {i} 组 {index}：bar");
                assert_eq!(
                    view.members,
                    fact.members.as_slice(),
                    "{label} 前缀 {i} 组 {index}：members（借用成员表 vs dense 映射）"
                );
                // `member_tail` 是同一成员序列去掉首锚（组锚 = `bar.source_index`）后的尾链：
                // 现存数据借用，不新算成员判据。
                assert_eq!(
                    view.member_tail.materialize(),
                    fact.members[1..].to_vec(),
                    "{label} 前缀 {i} 组 {index}：member_tail 尾链 vs members 去掉首锚"
                );
                assert_eq!(
                    view.high_sources,
                    fact.high_sources.as_slice(),
                    "{label} 前缀 {i} 组 {index}：high 来源"
                );
                assert_eq!(
                    view.low_sources,
                    fact.low_sources.as_slice(),
                    "{label} 前缀 {i} 组 {index}：low 来源"
                );
                assert_eq!(
                    view.confirmed, fact.confirmed,
                    "{label} 前缀 {i} 组 {index}：确认状态"
                );
                match (view.direction_evidence, fact.direction_evidence.as_ref()) {
                    (None, None) => {}
                    (Some(shared), Some(full)) => {
                        assert_eq!(
                            shared.direction, full.direction,
                            "{label} 前缀 {i} 组 {index}：方向证据方向"
                        );
                        assert_eq!(
                            shared.previous_acc, full.previous_acc,
                            "{label} 前缀 {i} 组 {index}：方向证据前驱"
                        );
                        assert_eq!(
                            shared.incoming, full.incoming,
                            "{label} 前缀 {i} 组 {index}：方向证据 incoming"
                        );
                        assert_eq!(
                            shared.coords.materialize(),
                            full.source_coords,
                            "{label} 前缀 {i} 组 {index}：方向证据来源坐标"
                        );
                    }
                    (shared, full) => panic!(
                        "{label} 前缀 {i} 组 {index}：方向证据有无不一致（借用 {shared:?} / 全量 {full:?}）"
                    ),
                }
                match (
                    view.confirmation_evidence,
                    fact.confirmation_evidence.as_ref(),
                ) {
                    (None, None) => {}
                    (Some(shared), Some(full)) => {
                        assert_eq!(
                            shared.direction, full.direction,
                            "{label} 前缀 {i} 组 {index}：封口证据方向"
                        );
                        assert_eq!(
                            shared.previous_acc, full.previous_acc,
                            "{label} 前缀 {i} 组 {index}：封口证据前驱"
                        );
                        assert_eq!(
                            shared.incoming, full.incoming,
                            "{label} 前缀 {i} 组 {index}：封口证据 incoming"
                        );
                        assert_eq!(
                            shared.coords.materialize(),
                            full.source_coords,
                            "{label} 前缀 {i} 组 {index}：封口证据来源坐标"
                        );
                    }
                    (shared, full) => panic!(
                        "{label} 前缀 {i} 组 {index}：封口证据有无不一致（借用 {shared:?} / 全量 {full:?}）"
                    ),
                }
            }
            assert!(
                matches!(state.shared_group(state.group_count()), Ok(None)),
                "{label} 前缀 {i}：索引不存在 = Ok(None)"
            );
        }
    }

    #[test]
    fn shared_group_matches_full_facts_for_every_prefix() {
        // 合并 / 定方向 / 换组（方向翻转）：方向证据与封口证据同时出现。
        assert_shared_group_views_match_facts(
            &[
                bar(0, 10, 5),
                bar(1, 15, 8),
                bar(2, 12, 9),
                bar(3, 8, 4),
                bar(4, 9, 5),
                bar(5, 14, 10),
                bar(6, 13, 11),
            ],
            "merge/new-group",
        );
        // 初始方向永久未定：组 0 只含种子成员，后续 raw 不冒充已分组。
        assert_shared_group_views_match_facts(
            &[bar(0, 10, 5), bar(1, 9, 6), bar(2, 12, 7), bar(3, 13, 8)],
            "initial-unsettled",
        );
        // 非连续源坐标（不得区间化）+ 同价极值身份竞争。
        assert_shared_group_views_match_facts(
            &[
                sparse_bar(10, 10, 5),
                sparse_bar(11, 15, 8),
                sparse_bar(37, 15, 6),
                sparse_bar(9000, 12, 6),
                sparse_bar(9001, 9, 4),
                sparse_bar(12345, 9, 3),
            ],
            "sparse/equal-extremes",
        );
        // 确定性随机多形态（包含/严格/方向翻转/同价 + 稀疏坐标）：逐前缀全字段对拍。
        let mut lcg: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rand = move || {
            lcg = lcg
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (lcg >> 33) as i64
        };
        let mut high = 80i64;
        let bars: Vec<Bar> = (0..240usize)
            .map(|i| {
                let step = rand() % 21 - 10;
                high = (high + step).clamp(10, 300);
                let span = 1 + rand() % 7;
                let tie = rand() % 5 == 0;
                let h = if tie { high } else { high + rand() % 3 };
                sparse_bar(i * 3 + 7, h, h - span)
            })
            .collect();
        assert_shared_group_views_match_facts(&bars, "randomized");
    }

    #[test]
    fn shared_group_handles_survive_later_appends() {
        let mut state = IncrFactsState::empty();
        for bar in [
            bar(0, 20, 10),
            bar(1, 24, 14),
            bar(2, 22, 16),
            bar(3, 23, 15),
        ] {
            state.append(bar);
        }
        assert!(!state.degraded());
        assert!(state.group_count() >= 2, "夹具应已建立方向并换组");
        // 组 0 带封口证据与 #1393 初始化证据、组 1 带方向证据——两侧都只取 O(1) 共享句柄
        // （不展开历史前缀）；末组尚未封口，单独核其封口位为空。
        let sealed = state.group_fact(0).expect("组 0 在域内");
        let directed = state.group_fact(1).expect("组 1 在域内");
        let (seal_handle, members_0, tail_0) = {
            let view = state
                .shared_group(0)
                .expect("共享态可用")
                .expect("组 0 在域内");
            assert!(view.confirmation_evidence.is_some(), "组 0 应带封口证据");
            assert!(
                Rc::ptr_eq(
                    view.direction_evidence.expect("组 0 携初始化证据"),
                    state.initial_direction_node().expect("已建立方向")
                ),
                "组 0 的方向证据就是初始化证据"
            );
            (
                view.confirmation_evidence.map(Rc::clone),
                view.members.to_vec(),
                view.member_tail.clone(),
            )
        };
        let (dir_handle, members_1, tail_1) = {
            let view = state
                .shared_group(1)
                .expect("共享态可用")
                .expect("组 1 在域内");
            assert!(view.direction_evidence.is_some(), "组 1 应带方向证据");
            assert!(view.confirmation_evidence.is_some(), "组 1 已被后组封口");
            (
                view.direction_evidence.map(Rc::clone),
                view.members.to_vec(),
                view.member_tail.clone(),
            )
        };
        let last = state.group_count() - 1;
        let tail_view = state
            .shared_group(last)
            .expect("共享态可用")
            .expect("末组在域内");
        assert!(tail_view.direction_evidence.is_some(), "末组应带方向证据");
        assert!(tail_view.confirmation_evidence.is_none(), "末组尚未封口");
        // 尾链句柄与成员表同源：首锚在 `bar.source_index`，尾链 = members[1..]。
        assert_eq!(tail_0.materialize(), members_0[1..].to_vec());
        assert_eq!(tail_1.materialize(), members_1[1..].to_vec());
        let seal_before = seal_handle
            .as_ref()
            .map(|node| node.coords.materialize())
            .expect("封口证据非空");
        let dir_before = dir_handle
            .as_ref()
            .map(|node| node.coords.materialize())
            .expect("方向证据非空");

        // 实际追加：继续推进成员、封口与证据链。
        for bar in [bar(4, 26, 18), bar(5, 28, 20), bar(6, 27, 21)] {
            state.append(bar);
        }
        assert!(!state.degraded());
        // 旧句柄保持旧内容：共享前缀不被后续追加改写。
        assert_eq!(
            seal_handle.as_ref().map(|node| node.coords.materialize()),
            Some(seal_before.clone()),
            "旧封口证据句柄被后续追加改写"
        );
        assert_eq!(
            dir_handle.as_ref().map(|node| node.coords.materialize()),
            Some(dir_before.clone()),
            "旧方向证据句柄被后续追加改写"
        );
        // 旧组完整事实与追加前逐位相同；重新借出的同组与旧内容一致。
        assert_eq!(state.group_fact(0), Some(sealed));
        assert_eq!(state.group_fact(1), Some(directed));
        let again0 = state
            .shared_group(0)
            .expect("共享态可用")
            .expect("组 0 在域内");
        let again1 = state
            .shared_group(1)
            .expect("共享态可用")
            .expect("组 1 在域内");
        assert_eq!(again0.members, members_0.as_slice());
        assert_eq!(again1.members, members_1.as_slice());
        // 旧尾链句柄与重新借出的尾链都保持旧内容（共享前缀不被后续追加改写）。
        assert_eq!(tail_0.materialize(), members_0[1..].to_vec());
        assert_eq!(tail_1.materialize(), members_1[1..].to_vec());
        assert_eq!(again0.member_tail.materialize(), members_0[1..].to_vec());
        assert_eq!(again1.member_tail.materialize(), members_1[1..].to_vec());
        assert_eq!(
            again0
                .confirmation_evidence
                .map(|node| node.coords.materialize()),
            Some(seal_before)
        );
        assert_eq!(
            again1
                .direction_evidence
                .map(|node| node.coords.materialize()),
            Some(dir_before)
        );
    }

    #[test]
    fn shared_group_contract_distinguishes_absent_and_degraded() {
        let bars = vec![bar(0, 10, 5), bar(1, 15, 8), bar(2, 14, 9)];
        let mut state = IncrFactsState::empty();
        for bar in &bars {
            state.append(*bar);
        }
        assert!(!state.degraded());
        assert!(
            matches!(state.shared_group(0), Ok(Some(_))),
            "在域内必须借出视图"
        );
        assert!(
            matches!(state.shared_group(state.group_count()), Ok(None)),
            "索引不存在 = Ok(None)"
        );
        assert!(matches!(state.shared_group(usize::MAX), Ok(None)));

        // 共享态退化（source 坐标非严格递增）= 显式错误，且先于索引判定。
        let degraded_bars = vec![bar(5, 10, 5), bar(3, 15, 8), bar(9, 14, 9), bar(1, 9, 4)];
        let mut degraded = IncrFactsState::empty();
        for bar in &degraded_bars {
            degraded.append(*bar);
        }
        assert!(degraded.degraded());
        let error = degraded
            .shared_group(0)
            .expect_err("退化态必须显式报错，不得静默降级");
        assert!(
            error.reason.contains("严格递增"),
            "错误须点名成因，实际：{}",
            error.reason
        );
        assert!(
            degraded.shared_group(usize::MAX).is_err(),
            "退化态先于索引判定"
        );
        // 既有展开入口不受新接口影响：仍按既定契约回落全量入口。
        assert_eq!(
            degraded.group_fact(0),
            process_inclusion_with_facts(&degraded_bars)
                .groups
                .first()
                .cloned()
        );
    }
}
