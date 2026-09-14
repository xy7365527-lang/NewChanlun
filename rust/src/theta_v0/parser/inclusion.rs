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

/// 包含合并输出：合并后的 K 序列 + 是否全程无方向（only_open_tail）。
#[derive(Debug, Clone, PartialEq, Default)]
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
    let (merged, _) = fold_all(bars, dir0);

    InclusionResult {
        merged,
        only_open_tail: false,
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
    /// 所属标准化组的组号（= merged 下标 = 合并位置）。
    pub group: usize,
    /// 该组标准化后的 high（组内所有成员同一值）。
    pub group_high: Tick,
    /// 该组标准化后的 low（组内所有成员同一值）。
    pub group_low: Tick,
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
    if bars.is_empty() {
        return Vec::new();
    }
    let merged = &result.merged;
    let mut ann = Vec::with_capacity(bars.len());
    for b in bars {
        // merged 非空（bars 非空 ⟹ process_inclusion 输出非空）且 `source_index` 单调
        // （左折叠前提）⟹ 必命中组号；`unwrap_or(0)` 仅防御非法输入，不改变合法路径。
        let group = merged_group_index(merged, b.source_index).unwrap_or(0);
        let g = &merged[group];
        ann.push(DenseBarAnnotation {
            group,
            group_high: g.high,
            group_low: g.low,
            confirmed: !result.only_open_tail && group + 1 < merged.len(),
            high: b.high,
            low: b.low,
        });
    }
    ann
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

    /// 从已有 `InclusionResult`（全量版输出）恢复增量状态。
    ///
    /// 用途：从全量基线起继续增量追加（如断点续算）。**仅当 `prev.only_open_tail=false`
    /// 时可精确恢复 left-fold 状态**——此时 `prev.merged` 直接成为相 B 的 `merged`
    /// （末根即 acc）。`dir` 从末两根严格高低变化推断（若末尾不足定方向，保持前序）。
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
                merged: Rc::new(Vec::new()),
                dir: Direction::Up,
                start_dir: None,
            };
        }
        // 有方向相 B：末根为 acc，前缀定稿。dir 从末两根严格高低变化推断（与全量
        // 左折叠中"acc 与新 bar 的 strict_dir 更新"语义一致——取末两根若有严格对）。
        let acc = prev.merged[prev.merged.len() - 1];
        let prefix = &prev.merged[..prev.merged.len() - 1];
        let dir = match (prefix.last(), Some(&acc)) {
            (Some(p), Some(a)) => strict_dir(p, a).unwrap_or(Direction::Up),
            _ => Direction::Up, // 仅一根 merged（合并成一根），无前对照——占位 Up。
        };
        IncrInclusion {
            raw_pending: Vec::new(),
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
        }
    }

    /// 当前状态快照的借用视图（零 clone，与全量输出 bit-exact）。
    ///
    /// 返回 `merged: &[Bar]` 指向内部连续存储（相 A = `raw_pending`，相 B = `merged`），
    /// 内容与 `to_result().merged` 逐字段相同。
    pub fn to_result_ref(&self) -> InclusionResultRef<'_> {
        let (merged, only_open_tail) = self.merged_view();
        InclusionResultRef {
            merged,
            only_open_tail,
        }
    }

    /// 合并序列的 Rc 共享句柄（O(1) refcount bump，#93 incr_total ceiling 突破）。
    ///
    /// 相 B 返回 `Rc::clone(&self.merged)`（与 `to_result_ref().merged` 内容 bit-exact，
    /// 共享同一分配）；相 A 返回 `Rc::new(self.raw_pending.clone())`（相 A 稀少，一次性
    /// O(n) 可接受——仅全程无方向时停留相 A）。供热路径 `ParseLayerIncr::append` 替代
    /// `to_result_ref().merged.to_vec()` 的每 bar O(n) clone。
    ///
    /// bit-exact：返回的 Rc<Vec<Bar>> 内容 == `to_result().merged` 逐字段。
    pub fn merged_rc(&self) -> Rc<Vec<Bar>> {
        match self.start_dir {
            None => Rc::new(self.raw_pending.clone()),
            Some(_) => Rc::clone(&self.merged),
        }
    }

    /// 内部：返回合并序列的借用切片 + only_open_tail 标志。
    fn merged_view(&self) -> (&[Bar], bool) {
        match self.start_dir {
            None => (&self.raw_pending, true),
            Some(_) => (self.merged.as_slice(), false),
        }
    }

    /// 相 A 推进：追加到 raw_pending，增量判定开头方向是否出现。
    ///
    /// 增量扫描性质：全量开头扫描查"第一对非包含严格单调对"。旧前缀已扫无非包含严格对
    /// （否则已进入相 B），故只需判 `raw_pending` 末根 与 `new_bar` 这一对。若严格 →
    /// 进入相 B，一次性全量左折叠 `raw_pending ++ [new_bar]`。
    ///
    /// mem::take 重用 raw_pending Vec 缓冲（仅 push 尾部，前缀不动），消除 clone。
    fn append_pending(self, new_bar: Bar) -> IncrInclusion {
        let mut raw = self.raw_pending;
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
                    merged: Rc::new(Vec::new()),
                    dir: Direction::Up,
                    start_dir: None,
                }
            }
            Some(dir0) => {
                // 进入相 B：从 raw 全量左折叠（一次性 O(n)，全程仅一次）。
                // fold_all 返回连续 merged（confirmed 前缀 + 末尾 acc）。
                let (merged, dir) = fold_all(&raw, dir0);
                IncrInclusion {
                    raw_pending: Vec::new(),
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
    pub merged: Vec<Bar>,
    pub groups: Vec<InclusionGroupFact>,
    pub steps: Vec<InclusionStepFact>,
    pub initial_direction_unsettled: bool,
    pub unmapped_sources: Vec<usize>,
}

/// #1373 已批有界入口：与全量/增量共享 fold_step，额外保存该步事实。
/// 未建方向的包含前缀不借未来方向回折；同价来源保留全集，不选择身份。
/// 这是知识域限制，不改变 contains/strict_dir/merge 的判据。
pub fn process_inclusion_with_facts(bars: &[Bar]) -> InclusionFacts {
    let mut result = InclusionFacts::default();
    let mut evidence: Option<InclusionDirectionEvidence> = None;
    let mut processed = 0usize;
    for (i, &incoming) in bars.iter().enumerate() {
        let before = result.merged.last().copied();
        if result.initial_direction_unsettled {
            result.unmapped_sources.push(incoming.source_index);
            result.steps.push(InclusionStepFact {
                action: "waiting",
                incoming,
                acc_before: before,
                acc_after: None,
                contains: None,
                direction_evidence: None,
                source_coords: bars[..=i].iter().map(|b| b.source_index).collect(),
                high_sources: vec![],
                low_sources: vec![],
                waiting_reasons: vec!["initial_direction_unsettled"],
            });
            continue;
        }
        if let Some(acc) = before {
            // 仅初始化知识门调用同一 contains；已建方向后的分支取自实际 fold_step 输出。
            if evidence.is_none() && contains(&acc, &incoming) {
                result.initial_direction_unsettled = true;
                result.unmapped_sources.push(incoming.source_index);
                result.steps.push(InclusionStepFact {
                    action: "waiting",
                    incoming,
                    acc_before: Some(acc),
                    acc_after: None,
                    contains: Some(true),
                    direction_evidence: None,
                    source_coords: vec![acc.source_index, incoming.source_index],
                    high_sources: vec![],
                    low_sources: vec![],
                    waiting_reasons: vec!["initial_direction_unsettled"],
                });
                continue;
            }
            let direction = evidence
                .as_ref()
                .map(|e| e.direction)
                .or_else(|| strict_dir(&acc, &incoming))
                .expect("合法非包含区间必有严格方向");
            let previous_members = result.groups.last().unwrap().members.clone();
            let was_established = evidence.is_some();
            result.merged.pop();
            let prefix_len = result.merged.len();
            let (after, new_direction) = fold_step(&acc, incoming, direction, &mut result.merged);
            let included = result.merged.len() == prefix_len;
            result.merged.push(after);
            if !included {
                let mut sources = previous_members.clone();
                if let Some(e) = &evidence {
                    sources.extend(&e.source_coords);
                }
                sources.push(incoming.source_index);
                sources.sort_unstable();
                sources.dedup();
                evidence = Some(InclusionDirectionEvidence {
                    direction: new_direction,
                    previous_acc: acc,
                    incoming,
                    source_coords: sources,
                });
                result.groups.last_mut().unwrap().confirmation_evidence = evidence.clone();
                result.groups.push(InclusionGroupFact {
                    bar: after,
                    members: vec![incoming.source_index],
                    high_sources: vec![incoming.source_index],
                    low_sources: vec![incoming.source_index],
                    confirmed: false,
                    confirmation_evidence: None,
                    direction_evidence: evidence.clone(),
                });
            } else {
                let group = result.groups.last_mut().unwrap();
                group.bar = after;
                group.members.push(incoming.source_index);
                // 数值来源投影：只读同核 merge 输出，不另算极值、不裁同价身份。
                group.high_sources = bars[..=i]
                    .iter()
                    .filter(|b| group.members.contains(&b.source_index) && b.high == after.high)
                    .map(|b| b.source_index)
                    .collect();
                group.low_sources = bars[..=i]
                    .iter()
                    .filter(|b| group.members.contains(&b.source_index) && b.low == after.low)
                    .map(|b| b.source_index)
                    .collect();
            }
            let group = result.groups.last().unwrap();
            let mut sources = previous_members;
            sources.push(incoming.source_index);
            if let Some(e) = &evidence {
                sources.extend(&e.source_coords);
            }
            sources.sort_unstable();
            sources.dedup();
            result.steps.push(InclusionStepFact {
                action: if included {
                    "merge"
                } else if was_established {
                    "new_group"
                } else {
                    "establish_direction"
                },
                incoming,
                acc_before: Some(acc),
                acc_after: Some(after),
                contains: Some(included),
                direction_evidence: evidence.clone(),
                source_coords: sources,
                high_sources: group.high_sources.clone(),
                low_sources: group.low_sources.clone(),
                waiting_reasons: if group.high_sources.len() > 1 || group.low_sources.len() > 1 {
                    vec!["equal_extreme_identity"]
                } else {
                    vec![]
                },
            });
        } else {
            result.merged.push(incoming);
            result.groups.push(InclusionGroupFact {
                bar: incoming,
                members: vec![incoming.source_index],
                high_sources: vec![incoming.source_index],
                low_sources: vec![incoming.source_index],
                confirmed: false,
                confirmation_evidence: None,
                direction_evidence: None,
            });
            result.steps.push(InclusionStepFact {
                action: "seed",
                incoming,
                acc_before: None,
                acc_after: Some(incoming),
                contains: None,
                direction_evidence: None,
                source_coords: vec![incoming.source_index],
                high_sources: vec![incoming.source_index],
                low_sources: vec![incoming.source_index],
                waiting_reasons: vec![],
            });
        }
        processed += 1;
    }
    // 正式成员映射复用 S9 稠密标注；未裁前缀之后的 raw 不冒充已分组。
    let annotations = dense_annotation(
        &bars[..processed],
        &InclusionResult {
            merged: result.merged.clone(),
            only_open_tail: evidence.is_none(),
        },
    );
    let group_count = result.groups.len();
    for (index, group) in result.groups.iter_mut().enumerate() {
        group.members = bars[..processed]
            .iter()
            .zip(&annotations)
            .filter(|(_, a)| a.group == index)
            .map(|(b, _)| b.source_index)
            .collect();
        group.confirmed = evidence.is_some() && index + 1 < group_count;
    }
    result
}

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
    fn tb02a_initial_inclusion_does_not_borrow_future_direction() {
        let bars = vec![bar(0, 5, 1), bar(1, 4, 2), bar(2, 6, 3)];
        let facts = process_inclusion_with_facts(&bars);
        assert!(facts.initial_direction_unsettled);
        assert_eq!(facts.merged, vec![bars[0]]);
        assert_eq!(facts.groups[0].members, vec![0]);
        assert_eq!(facts.unmapped_sources, vec![1, 2]);
        assert!(facts.steps[1..].iter().all(|s| s.action == "waiting"
            && s.direction_evidence.is_none()
            && s.acc_after.is_none()));
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
            vec![0, 1, 1, 2, 3]
        );
        // 组 1 标准化高低 = bar1、bar2 向上合并 = [12, 8]。
        assert_eq!((ann[1].group_high, ann[1].group_low), (12, 8));
        assert_eq!((ann[2].group_high, ann[2].group_low), (12, 8));
        // 组内逐根高低：组 1 内两根本身的高/低（供「组内 high 最大的那根」判据读数）。
        assert_eq!((ann[1].high, ann[1].low), (12, 7));
        assert_eq!((ann[2].high, ann[2].low), (11, 8));
        // 已确认／暂定：前缀组（组号 0..2）确认，末组（组号 3）暂定。
        assert!(ann[0].confirmed && ann[1].confirmed && ann[2].confirmed && ann[3].confirmed);
        assert!(!ann[4].confirmed);
    }

    /// open-tail（全程无方向）⟹ 每根自成一格、全部暂定（无 confirmed 结构）。
    #[test]
    fn dense_annotation_open_tail_all_tentative() {
        let bars = vec![bar(0, 20, 1), bar(1, 10, 5), bar(2, 8, 6)];
        let r = process_inclusion(&bars);
        assert!(r.only_open_tail);
        let ann = dense_annotation(&bars, &r);
        assert_eq!(
            ann.iter().map(|a| a.group).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(ann.iter().all(|a| !a.confirmed));
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
        if bars.is_empty() {
            return Vec::new();
        }
        let num_groups = ann.iter().map(|a| a.group).max().unwrap_or(0) + 1;
        let mut merged = Vec::with_capacity(num_groups);
        for g in 0..num_groups {
            let members: Vec<usize> = (0..bars.len()).filter(|&i| ann[i].group == g).collect();
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
                high: ann[first].group_high,
                low: ann[first].group_low,
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
}
