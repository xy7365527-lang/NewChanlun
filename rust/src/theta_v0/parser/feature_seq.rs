//! 增量特征序列状态机（第67/71课）——bit-exact 移植 Python `a_segment_v1._FeatureSeqState`。
//!
//! ## 认识论等级（formalization-validity-domain，强制标注）
//!
//! **本文件 = L1（忠实于 v1 spec `a_segment_v1.py`，**非** L2 经验验证 / **非** 对 Lean
//! bit-exact）。**
//!
//! 参考语义 Spec_seg = `src/newchan/a_segment_v1.py`（编排者裁定 v1 唯一口径，37 测试）。
//! 教义（codex ReferenceSemantics）：引擎不是定义——theta_v0 segment 须**对参考语义认证**
//! `Normalize(theta_v0)=Spec_seg`，认证 harness = `analysis/segment_refsem_cert.py`。
//!
//! ★为何重写（cc-refsem-harness 归因，#84）：旧 `segment.rs::divide_segments` 用「无状态
//! 批处理找首个分型即断段」，对参考语义认证**失败**（真实数据 406 段 vs 参考 237 段，70%
//! 过分段，前 237 段仅 2 段 bit-exact 一致）。归因证伪「inclusion 是根因」（外包络→方向性
//! 仅 Δ=2 段，1.2%）——真因是**算法范式整体差异**：参考用增量「假设转折点」状态机，旧实装
//! 缺此核心。本文件移植参考的增量算法以通过认证。
//!
//! ## 静态层契约 vs 动态层有效域（无矛盾，formalization-validity-domain）
//!
//! Origin `SegmentConstruction.segmentsOf`（committed naive cut）+ `SegmentFeatureComplete.SegEndComplete`
//! （静态完整段端确认谓词）形式化了线段划分的**静态层**；**动态划分状态机**（第71课「假设转折点」
//! 增量过程）**不在 Origin 形式化范围**（是 `_FeatureSeqState` 状态机职责，非遗漏——与 legacy
//! Claim10:334-346 同样的有意非形式化边界）。故本文件移植动态算法**不碰静态层契约**：静态原语
//! （`Interval`/`feature_elements`/`classify_termination`，segment.rs）保持对
//! `Origin.SegmentFeatureSeq`/`SegmentFeatureComplete` 对齐；动态扫描（本文件）对 Python 参考
//! bit-exact。两个认证目标在不同有效域，不冲突。
//!
//! ## 第71课博文权威（一级权威，CLAUDE.md 三级权威链）
//!
//! 071-第71课.md:38「线段的划分，都是可以当下完成的……假设某转折点是两线段的分界点，然后
//! 对此用线段划分的两种情况去考察是否满足」+ :42「在这假设的转折点前后那两元素，是不存在
//! 包含关系的」——确认「假设转折点」逻辑（包含时先试不合并看是否触发分型）是正确缠论语义。

use super::super::types::{Direction, Stroke, Tick};

// ════════════════════════════════════════════════════════════
// #246 相切探针（票 #248 裁定影响量化）——thread_local 计数器
// ════════════════════════════════════════════════════════════
//
// 仿 `strategy/coverage.rs` `ancok_probe` 模式：thread_local +=1，零分配、不改返回值，
// 只为量化「边界相切」在真实数据上的命中频次（裁定书 §6 实装约束：必须带影响量化）。
// 探针埋在生产谓词本体内 ⟹ 计数路径 = 生产调用路径（无离线复算的等价性论证负担）。
// 计数语义与谓词新旧口径无关（只记「边界相等」数据事实），改前/改后可直接对比。
// 留存理由同 ancok_probe：增量成本一次 +=1，换任意数据 run 可复测相切频次。

/// #246 相切探针计数（`is_fractal_and_gap` 缺口谓词）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GapTangentProbe {
    /// `has_gap` 谓词求值次数（仅分型成立的调用才求值缺口）。
    pub gap_evals: u64,
    /// 向上段顶分型且 `b_l == a_h`（相切）：旧口径（>=）判缺口 / 新口径（>）判重合。
    pub tangent_up: u64,
    /// 向下段底分型且 `a_l == b_h`（相切）：同上。
    pub tangent_down: u64,
}

thread_local! {
    static GAP_TANGENT_PROBE: std::cell::RefCell<GapTangentProbe> =
        std::cell::RefCell::new(GapTangentProbe::default());
}

fn gap_probe_bump(f: impl FnOnce(&mut GapTangentProbe)) {
    GAP_TANGENT_PROBE.with(|p| f(&mut p.borrow_mut()));
}

/// 相切探针清零（probe binary / 测试用）。
pub fn gap_tangent_probe_reset() {
    gap_probe_bump(|p| *p = GapTangentProbe::default());
}

/// 相切探针快照（probe binary / 测试用）。
pub fn gap_tangent_probe_snapshot() -> GapTangentProbe {
    GAP_TANGENT_PROBE.with(|p| *p.borrow())
}

/// 段方向字符串等价（Python seg_direction "up"/"down"）。本模块内部用 Direction，
/// 但需保留 Python 的方向语义映射。

/// 特征序列元素：`[high, low, stroke_idx]`（Python `std` 元素，high 在前）。
///
/// ★注意（Python 移植）：用笔的 `high/low` 而非 `[lo,hi]` 几何投影。Stroke 当前类型
/// 用 `start_price/end_price`，其极值 `max/min(start,end)` = high/low（笔端点即极值，
/// 无 K 线包含残留时恒等；L2 OKLO 2000 笔验证 0 例外）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FeatElem {
    high: Tick,
    low: Tick,
    stroke_idx: usize,
}

/// 笔的特征序列高低（= max/min(start_price, end_price)）。
fn stroke_high_low(s: &Stroke) -> (Tick, Tick) {
    if s.start_price >= s.end_price {
        (s.start_price, s.end_price)
    } else {
        (s.end_price, s.start_price)
    }
}

/// 分型 + 缺口判定。
///
/// 向上段顶分型：`b_h > a_h && b_h > c_h`（只看 high）；缺口 `b_l > a_h`（b 完全在 a 上方，边界相切不算缺口）。
/// 向下段底分型：`b_l < a_l && b_l < c_l`（只看 low）；缺口 `a_l > b_h`（a 完全在 b 上方，边界相切不算缺口）。
/// 返回 `(is_fractal, has_gap)`。
///
/// ★口径（#246 裁定，2026-07-25，supersede Lead #84 点3）：缺口谓词用**严格 `>`**——
/// 边界相切（`b_l == a_h` / `a_l == b_h`）算「有重合区间」⟹ 无缺口，全域生效。裁定书：
/// `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。强制性依据：Lean 已证
/// `gap_iff_not_overlap : HasGap a b ↔ ¬ Overlaps a b`（`formal/Origin/SegmentFeatureSeq.lean`），
/// 缺口与重合严格互补，本谓词与 `segment.rs::three_stroke_overlap` 必须同口径、同批改。
/// ⚠原「bit-exact 对齐 Python `_is_fractal_and_gap`」声明在**缺口谓词边界上作废**：Python 参考
/// 实现（`>=`，相切算缺口）不再是该谓词的权威基线；分型判定及其余 Python 对齐声明不受影响。
fn is_fractal_and_gap(
    a_h: Tick,
    a_l: Tick,
    b_h: Tick,
    b_l: Tick,
    c_h: Tick,
    c_l: Tick,
    seg_dir: Direction,
) -> (bool, bool) {
    match seg_dir {
        Direction::Up => {
            let is_fractal = b_h > a_h && b_h > c_h;
            let has_gap = if is_fractal {
                gap_probe_bump(|p| {
                    p.gap_evals += 1;
                    if b_l == a_h {
                        p.tangent_up += 1;
                    }
                });
                b_l > a_h
            } else {
                false
            };
            (is_fractal, has_gap)
        }
        Direction::Down => {
            let is_fractal = b_l < a_l && b_l < c_l;
            let has_gap = if is_fractal {
                gap_probe_bump(|p| {
                    p.gap_evals += 1;
                    if a_l == b_h {
                        p.tangent_down += 1;
                    }
                });
                a_l > b_h
            } else {
                false
            };
            (is_fractal, has_gap)
        }
    }
}

/// 在元素序列上检测**任意**分型（顶或底，第67课"第二个序列只要有分型就可以"）。
/// bit-exact 对齐 Python `_has_any_fractal`（:221-239）。
fn has_any_fractal(elements: &[(Tick, Tick)]) -> bool {
    let n = elements.len();
    if n < 3 {
        return false;
    }
    for j in 1..n - 1 {
        let (b_h, b_l) = elements[j];
        let (a_h, a_l) = elements[j - 1];
        let (c_h, c_l) = elements[j + 1];
        if b_h > a_h && b_h > c_h {
            return true;
        }
        if b_l < a_l && b_l < c_l {
            return true;
        }
    }
    false
}

/// 对 elements 尾部做包含处理或追加（bit-exact 对齐 Python `_apply_inclusion`，:193-218）。
///
/// 方向性合并：`effective_up = dir_state != Down` → `max/max`；否则 `min/min`。返回新 dir_state。
fn apply_inclusion(
    elements: &mut Vec<(Tick, Tick)>,
    h: Tick,
    l: Tick,
    dir_state: Option<Direction>,
) -> Option<Direction> {
    let (last_h, last_l) = *elements.last().expect("apply_inclusion 调用前 elements 非空");
    let left_inc = last_h >= h && last_l <= l;
    let right_inc = h >= last_h && l <= last_l;
    if left_inc || right_inc {
        let effective_up = dir_state != Some(Direction::Down);
        let last = elements.last_mut().unwrap();
        if effective_up {
            last.0 = last_h.max(h);
            last.1 = last_l.max(l);
        } else {
            last.0 = last_h.min(h);
            last.1 = last_l.min(l);
        }
        return dir_state;
    }
    let mut ds = dir_state;
    if h > last_h && l > last_l {
        ds = Some(Direction::Up);
    } else if h < last_h && l < last_l {
        ds = Some(Direction::Down);
    }
    elements.push((h, l));
    ds
}

/// 第二特征序列是否存在分型（bit-exact 对齐 Python `_second_seq_has_fractal`，:354-384）。
///
/// 从 `from_stroke_idx` 之后取 **seg_dir 同向笔**构造第二特征序列，方向性包含处理后查任意分型。
/// `scan_window`：0 = 无限全扫（second_seq_scan_window config，default 0，bit-exact 优先）；
/// >0 = Python `MAX_SECOND_SEQ_SCAN` 风格窗口（笔数上限，含双向）。
///
/// ★性能（#93，bit-exact）：单遍构建 + 尾部三元组提前终止。`has_any_fractal` 对完整序列的判定
/// = 「存在某三元组成分型」。构建过程中每加入一个元素，只需检查**新尾部三元组**（旧三元组
/// 未变，前序无分型已隐含）——一旦成分型立即返回 true，避免完整 collect + 二次遍历。包含合并
/// 修改尾元素后同样只影响尾部三元组，检查时机一致。语义等价于完整 `has_any_fractal`。
pub(super) fn second_seq_has_fractal(
    strokes: &[Stroke],
    seg_dir: Direction,
    from_stroke_idx: usize,
    scan_window: u32,
) -> bool {
    let mut elements: Vec<(Tick, Tick)> = Vec::new();
    // Python: dir_state = "DOWN" if seg_dir=="up" else None（注意：与主序列相反！）。
    let mut dir_state: Option<Direction> = match seg_dir {
        Direction::Up => Some(Direction::Down),
        Direction::Down => None,
    };
    let end_idx = if scan_window == 0 {
        strokes.len()
    } else {
        (from_stroke_idx + 1 + scan_window as usize).min(strokes.len())
    };
    let mut i = from_stroke_idx + 1;
    while i < end_idx {
        let sk = &strokes[i];
        if sk.direction != seg_dir {
            i += 1;
            continue;
        }
        let (h, l) = stroke_high_low(sk);
        if elements.is_empty() {
            elements.push((h, l));
        } else {
            dir_state = apply_inclusion(&mut elements, h, l, dir_state);
        }
        // 尾部三元组提前终止：elements 尾三元组成任意分型 → 确认（等价 has_any_fractal 前缀判定）。
        let n = elements.len();
        if n >= 3 {
            let (a_h, a_l) = elements[n - 3];
            let (b_h, b_l) = elements[n - 2];
            let (c_h, c_l) = elements[n - 1];
            // 顶分型：b_h 严格高于左右 hi；底分型：b_l 严格低于左右 lo（同 has_any_fractal）。
            if b_h > a_h && b_h > c_h {
                return true;
            }
            if b_l < a_l && b_l < c_l {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// 延续模式（Python `extend_mode`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendMode {
    /// 67课严格延续：缺口被 c 封闭 → 按第一种情况处理（段更易终结）。
    Strict,
    /// 优化延续：任何缺口都走第二种情况（段更易延续）。
    Optimized,
}

/// scan_trigger 命中结果：`(trigger_stroke_k, gap_is_second)`。
/// `k` = 分型中心 b 对应的笔索引（Python `b_stroke`）；段端 = `k - 1`（同向笔）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerHit {
    pub k: usize,
    pub gap_second: bool,
}

/// 增量特征序列状态机（bit-exact 移植 Python `_FeatureSeqState`，:269-433）。
pub struct FeatureSeqState {
    /// 标准特征序列：每元素 `[high, low, stroke_idx]`。
    std: Vec<FeatElem>,
    /// 方向态（None = 默认 UP）。向上段初始 None，向下段初始 Down。
    dir_state: Option<Direction>,
    /// 上次分型检查起始位置。
    last_checked: usize,
    /// 跳过 stroke_idx <= 此值的分型（i64：-1 = 无跳过）。
    skip_until_stroke: i64,
    seg_dir: Direction,
    extend_mode: ExtendMode,
    /// 尾窗扫描大小（0 = 无限，对齐 Python TAIL_WINDOW=7 但 L2 验证窗口非约束）。
    tail_window: u32,
    /// 第二特征序列扫描窗口（config second_seq_scan_window，0=无限）。
    second_seq_window: u32,
    /// #88 frontier 修复：本段扫描期间是否跳过过 SecondKind 候选（`has_gap &&
    /// !second_seq_has_fractal`）。跳过的候选未来 bar 可能让第二序列出现分形而复活，改写本段
    /// ⟹ 本段 seg_start 是 unsealed 起点。`reset` 清零（每段独立），跳过时置真（段内累积）。
    /// 增量层 `IncrSegments::append` 读此标志把 seg_start 记入 `earliest_unsealed_from`。
    skipped_secondkind: bool,
}

impl FeatureSeqState {
    /// 新建状态机。`seg_dir` = 线段方向。dir_state 初值对齐 Python（向下段 Down，向上 None）。
    pub fn new(
        seg_dir: Direction,
        extend_mode: ExtendMode,
        tail_window: u32,
        second_seq_window: u32,
    ) -> Self {
        FeatureSeqState {
            std: Vec::new(),
            dir_state: match seg_dir {
                Direction::Down => Some(Direction::Down),
                Direction::Up => None,
            },
            last_checked: 0,
            skip_until_stroke: -1,
            seg_dir,
            extend_mode,
            tail_window,
            second_seq_window,
            skipped_secondkind: false,
        }
    }

    /// 重置为新段（Python `reset`，:289-293）。#88：清 skipped_secondkind（每段独立）。
    pub fn reset(&mut self, seg_dir: Direction) {
        self.std.clear();
        self.dir_state = match seg_dir {
            Direction::Down => Some(Direction::Down),
            Direction::Up => None,
        };
        self.last_checked = 0;
        self.skip_until_stroke = -1;
        self.seg_dir = seg_dir;
        self.skipped_secondkind = false;
    }

    /// #88：本段扫描期间是否跳过过 SecondKind 候选（unsealed 起点信号，增量层读取）。
    pub fn skipped_secondkind(&self) -> bool {
        self.skipped_secondkind
    }

    /// 标记跳过 stroke_idx <= 此值的分型（Python `skip_trigger`，:295-301）。
    pub fn skip_trigger(&mut self, stroke_idx: usize) {
        self.skip_until_stroke = stroke_idx as i64;
    }

    /// 增量添加一个反向笔，包含处理遵循71课"假设转折点"（bit-exact Python `append`，:303-350）。
    ///
    /// 检测到包含时**先尝试不合并**（push 看 scan_trigger 是否触发分型）：
    /// - 有分型 → 不合并（转折点，两元素属不同特征序列，第71课:42）。
    /// - 无分型 → pop 回退，按方向性 K 线包含规则合并。
    pub fn append(&mut self, stroke_idx: usize, high: Tick, low: Tick, strokes: &[Stroke]) {
        if self.std.is_empty() {
            self.std.push(FeatElem { high, low, stroke_idx });
            return;
        }
        let last = *self.std.last().unwrap();
        let (last_h, last_l) = (last.high, last.low);
        let left_inc = last_h >= high && last_l <= low;
        let right_inc = high >= last_h && low <= last_l;
        let has_inclusion = left_inc || right_inc;

        if has_inclusion {
            // 假设转折点：先 push 试探，看是否触发分型（第71课:42）。
            self.std.push(FeatElem { high, low, stroke_idx });
            if self.scan_trigger(strokes).is_some() {
                return;
            }
            self.std.pop();

            // 无分型 → 方向性合并到 last。
            let effective_up = self.dir_state != Some(Direction::Down);
            let last_mut = self.std.last_mut().unwrap();
            if effective_up {
                last_mut.high = last_h.max(high);
                last_mut.low = last_l.max(low);
            } else {
                last_mut.high = last_h.min(high);
                last_mut.low = last_l.min(low);
            }
            last_mut.stroke_idx = stroke_idx;
            self.last_checked = self.std.len().saturating_sub(3);
        } else {
            if high > last_h && low > last_l {
                self.dir_state = Some(Direction::Up);
            } else if high < last_h && low < last_l {
                self.dir_state = Some(Direction::Down);
            }
            self.std.push(FeatElem { high, low, stroke_idx });
        }
    }

    /// 从 last_checked 向后扫描（受尾窗限制），找第一个匹配分型（bit-exact Python `scan_trigger`）。
    ///
    /// 向上段找顶分型，向下段找底分型。跳过 stroke_idx <= skip_until_stroke 的分型。
    /// 缺口处理：strict 模式下缺口被 c 封闭 → 变第一种情况；有缺口须第二特征序列出现分型才确认。
    pub fn scan_trigger(&mut self, strokes: &[Stroke]) -> Option<TriggerHit> {
        let n = self.std.len();
        if n < 3 {
            return None;
        }
        // start = max(1, last_checked, n - tail_window)。tail_window=0 → 无尾窗下限。
        let tail_lo = if self.tail_window == 0 {
            0
        } else {
            n.saturating_sub(self.tail_window as usize)
        };
        let start = 1.max(self.last_checked).max(tail_lo);
        for i in start..n - 1 {
            let b_stroke = self.std[i].stroke_idx;
            if (b_stroke as i64) <= self.skip_until_stroke {
                continue;
            }
            let (a_h, a_l) = (self.std[i - 1].high, self.std[i - 1].low);
            let (b_h, b_l) = (self.std[i].high, self.std[i].low);
            let (c_h, c_l) = (self.std[i + 1].high, self.std[i + 1].low);
            let (is_fractal, mut has_gap) =
                is_fractal_and_gap(a_h, a_l, b_h, b_l, c_h, c_l, self.seg_dir);
            if !is_fractal {
                continue;
            }
            // 67课严格延续：缺口被 c 封闭 → 第一种情况（Python :414-422）。
            if has_gap && self.extend_mode == ExtendMode::Strict {
                let gap_closed_by_c = match self.seg_dir {
                    Direction::Up => c_l <= a_h,
                    Direction::Down => c_h >= a_l,
                };
                if gap_closed_by_c {
                    has_gap = false;
                }
            }
            // 有缺口须第二特征序列出现分型才确认（Python :424-427）。
            if has_gap
                && !second_seq_has_fractal(strokes, self.seg_dir, b_stroke, self.second_seq_window)
            {
                // #88 frontier：跳过的 SecondKind 候选未来可复活 ⟹ 本段 seg_start unsealed，
                // 增量层必须回退到其前重扫（第二序列 scan_window=0 无限 ⟹ 任意早的候选都可复活）。
                self.skipped_secondkind = true;
                continue;
            }
            self.last_checked = i.saturating_sub(1);
            return Some(TriggerHit { k: b_stroke, gap_second: has_gap });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Stroke {
        Stroke { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    #[test]
    fn is_fractal_and_gap_up_top_with_gap() {
        // 向上段：b_h 最高 → 顶分型；b_l >= a_h → 缺口。
        let (f, g) = is_fractal_and_gap(10, 5, 20, 12, 8, 3, Direction::Up);
        assert!(f); // 20 > 10 && 20 > 8
        assert!(g); // b_l=12 >= a_h=10
    }

    #[test]
    fn is_fractal_and_gap_up_top_no_gap() {
        // 顶分型但 b_l < a_h → 无缺口（第一种情况）。
        let (f, g) = is_fractal_and_gap(10, 5, 20, 8, 8, 3, Direction::Up);
        assert!(f);
        assert!(!g); // b_l=8 < a_h=10
    }

    #[test]
    fn is_fractal_and_gap_down_bottom() {
        // 向下段：b_l 最低 → 底分型；a_l >= b_h → 缺口。
        let (f, g) = is_fractal_and_gap(15, 12, 8, 3, 18, 14, Direction::Down);
        assert!(f); // 3 < 12 && 3 < 14
        assert!(g); // a_l=12 >= b_h=8
    }

    /// ★#312 主缝：`is_fractal_and_gap` 的缺口真值 == Lean `decide (HasGap a b)`（fixture 机器导出）。
    ///
    /// 遍历 Lean 侧 5 条用例 × 向上/向下两分支。期望值**全部从 fixture 读**
    /// （`gap_overlap_fixture`，禁手填）——口径若回改（相切算缺口，旧 `>=`），相切用例立刻红，
    /// 且失败信息指认到本谓词与具体形态。
    ///
    /// ★如实登记：`ordered()` 定序后 `*_rev` 两条与其正向用例输入逐字相同，本测试实得 **3 组互异
    /// 输入**（相切 / 严格分离 / 严格重叠）；相切的两个方向由下面 Up / Down 两分支取得，不靠 rev
    /// 用例（见 `gap_overlap_fixture::GapOverlapCase::ordered` 的副作用登记）。
    ///
    /// 定序 + 构造：`ordered()` 取（下方元素, 上方元素）；向上段喂 `(a=下, b=上, c=下)`，向下段喂
    /// `(a=上, b=下, c=上)`——`c` 复用同一元素只为满足分型前件（`b_h > a_h && b_h > c_h` /
    /// `b_l < a_l && b_l < c_l`），使 `has_gap` 分支真正被求值（前件不成立时该谓词恒返回 false，
    /// 断言会退化为同义反复，故先 `assert!(f)` 坐实前件）。
    #[test]
    fn is_fractal_and_gap_lean_fixture_bit_exact() {
        for case in super::super::gap_overlap_fixture::gap_overlap_cases() {
            let ((lo_l, lo_h), (up_l, up_h)) = case.ordered();
            // 向上段顶分型：b 在上，缺口谓词 = `b_l > a_h`。
            let (f_up, g_up) =
                is_fractal_and_gap(lo_h, lo_l, up_h, up_l, lo_h, lo_l, Direction::Up);
            assert!(
                f_up,
                "{}: 向上段前件（顶分型）应成立，否则断言退化",
                case.name
            );
            assert_eq!(
                g_up, case.has_gap,
                "{}: is_fractal_and_gap(Up).has_gap 须 == Lean decide(HasGap)（bit-exact）",
                case.name
            );
            // 向下段底分型：b 在下，缺口谓词 = `a_l > b_h`。
            let (f_dn, g_dn) =
                is_fractal_and_gap(up_h, up_l, lo_h, lo_l, up_h, up_l, Direction::Down);
            assert!(
                f_dn,
                "{}: 向下段前件（底分型）应成立，否则断言退化",
                case.name
            );
            assert_eq!(
                g_dn, case.has_gap,
                "{}: is_fractal_and_gap(Down).has_gap 须 == Lean decide(HasGap)（bit-exact）",
                case.name
            );
        }
    }

    #[test]
    fn has_any_fractal_detects_top_and_bottom() {
        assert!(has_any_fractal(&[(5, 1), (10, 6), (7, 3)])); // 顶
        assert!(has_any_fractal(&[(10, 8), (6, 2), (12, 7)])); // 底
        assert!(!has_any_fractal(&[(5, 1), (8, 3), (12, 6)])); // 单调无分型
    }

    #[test]
    fn apply_inclusion_directional_up() {
        // 向上（dir_state=None=UP）：包含 → max/max。
        let mut e = vec![(20, 10)];
        let ds = apply_inclusion(&mut e, 18, 12, None);
        assert_eq!(e, vec![(20, 12)]); // max(20,18)=20, max(10,12)=12
        assert_eq!(ds, None);
    }

    #[test]
    fn apply_inclusion_directional_down() {
        // 向下（dir_state=Down）：包含 → min/min。
        let mut e = vec![(20, 10)];
        let ds = apply_inclusion(&mut e, 18, 12, Some(Direction::Down));
        assert_eq!(e, vec![(18, 10)]); // min(20,18)=18, min(10,12)=10
        assert_eq!(ds, Some(Direction::Down));
    }

    #[test]
    fn apply_inclusion_no_inclusion_appends() {
        let mut e = vec![(10, 5)];
        let ds = apply_inclusion(&mut e, 20, 12, None); // 20>10 && 12>5 → 不含，追加，UP
        assert_eq!(e, vec![(10, 5), (20, 12)]);
        assert_eq!(ds, Some(Direction::Up));
    }

    #[test]
    fn append_empty_pushes_first() {
        let mut st = FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0);
        st.append(0, 20, 10, &[]);
        assert_eq!(st.std.len(), 1);
    }

    #[test]
    fn scan_trigger_too_few_none() {
        let mut st = FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0);
        st.append(0, 20, 10, &[]);
        st.append(2, 25, 15, &[]);
        assert_eq!(st.scan_trigger(&[]), None); // < 3 元素
    }

    #[test]
    fn scan_trigger_finds_top_no_gap() {
        // 向上段特征序列 [10,5],[20,8],[8,3]：中间 high=20 最高 → 顶分型；b_l=8<a_h=10 无缺口。
        let mut st = FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0);
        st.append(1, 10, 5, &[]);
        st.append(3, 20, 8, &[]);
        st.append(5, 8, 3, &[]);
        let hit = st.scan_trigger(&[]);
        assert_eq!(hit, Some(TriggerHit { k: 3, gap_second: false }));
    }

    #[test]
    fn second_seq_window_unlimited_vs_bounded() {
        // 仅验证窗口参数生效不 panic：无同向笔 → 无分型。
        let strokes = vec![stroke(Direction::Up, 0, 4, 5, 20)];
        assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 0));
        assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 50));
    }

    /// ★bit-exact 等价性验证（#93 性能优化的等价锚）：提前终止版 `second_seq_has_fractal`
    /// 必须等价于"完整 collect elements + apply_inclusion + has_any_fractal"的参照实现。
    ///
    /// 这是合成数据 L1 交叉验证（formalization-validity-domain：L0→L1 信息增量为零，仅验证
    /// 管线等价；L2 真实数据等价由 diag_segment_window_effect 的 seg_n=1072 全规模 bit-exact 提供）。
    /// 参照实现独立重写完整扫描逻辑（不调被测函数），二者比对。
    fn reference_second_seq_has_fractal(
        strokes: &[Stroke],
        seg_dir: Direction,
        from_stroke_idx: usize,
    ) -> bool {
        let mut elements: Vec<(Tick, Tick)> = Vec::new();
        let mut dir_state: Option<Direction> = match seg_dir {
            Direction::Up => Some(Direction::Down),
            Direction::Down => None,
        };
        for sk in strokes.iter().skip(from_stroke_idx + 1) {
            if sk.direction != seg_dir {
                continue;
            }
            let (h, l) = stroke_high_low(sk);
            if elements.is_empty() {
                elements.push((h, l));
            } else {
                dir_state = apply_inclusion(&mut elements, h, l, dir_state);
            }
        }
        has_any_fractal(&elements)
    }

    #[test]
    fn property_second_seq_early_terminate_equals_full_scan() {
        // 多组手工序列：单调（无分型）、顶分型、底分型、包含后分型、长序列无分型。
        // 向上线段（seg_dir=Up）→ 第二特征序列取 Up 笔（同向），dir_state 初始 Down。
        let cases: Vec<Vec<Stroke>> = vec![
            // 单调递增 Up 笔（无分型）。
            vec![
                stroke(Direction::Down, 0, 1, 100, 0),
                stroke(Direction::Up, 1, 2, 0, 10),
                stroke(Direction::Down, 2, 3, 10, 1),
                stroke(Direction::Up, 3, 4, 1, 20),
                stroke(Direction::Down, 4, 5, 20, 2),
                stroke(Direction::Up, 5, 6, 2, 30),
            ],
            // 顶分型（中间 hi 最高）。
            vec![
                stroke(Direction::Down, 0, 1, 100, 0),
                stroke(Direction::Up, 1, 2, 0, 10),
                stroke(Direction::Down, 2, 3, 10, 1),
                stroke(Direction::Up, 3, 4, 1, 50),
                stroke(Direction::Down, 4, 5, 50, 2),
                stroke(Direction::Up, 5, 6, 2, 20),
            ],
            // 底分型（中间 lo 最低）。
            vec![
                stroke(Direction::Down, 0, 1, 100, 0),
                stroke(Direction::Up, 1, 2, 0, 30),
                stroke(Direction::Down, 2, 3, 30, 1),
                stroke(Direction::Up, 3, 4, 1, 5),
                stroke(Direction::Down, 4, 5, 5, 2),
                stroke(Direction::Up, 5, 6, 2, 40),
            ],
            // 包含后成顶分型（apply_inclusion 后中元素最高）。
            vec![
                stroke(Direction::Down, 0, 1, 100, 0),
                stroke(Direction::Up, 1, 2, 0, 10),
                stroke(Direction::Down, 2, 3, 10, 1),
                stroke(Direction::Up, 3, 4, 1, 60), // [1,60]
                stroke(Direction::Down, 4, 5, 60, 2),
                stroke(Direction::Up, 5, 6, 2, 55), // [2,55] 被 [1,60] 包含 → 合并看 dir_state
                stroke(Direction::Down, 6, 7, 55, 3),
                stroke(Direction::Up, 7, 8, 3, 20),
            ],
        ];
        for (i, strokes) in cases.iter().enumerate() {
            let early = second_seq_has_fractal(strokes, Direction::Up, 0, 0);
            let full = reference_second_seq_has_fractal(strokes, Direction::Up, 0);
            assert_eq!(early, full, "case {i}: early-terminate ({early}) != full-scan ({full})");
        }
    }
}
