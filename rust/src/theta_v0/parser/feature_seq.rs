//! 增量特征序列状态机（第67/71课）。第一种情况的包含与确认按 `071:30/:32/:42` 直落（#1490，
//! `xianduan.md` §7.1；#1493 TB-02-X）；Python v1 `a_segment_v1._FeatureSeqState` 为历史参考，非正本。
//!
//! ⚠口径例外（#246 裁定，supersede Lead #84 点3）：缺口谓词 `is_fractal_and_gap` 的
//! 「相切」边界自 2026-07-25 起为**严格** `>`（相切=重合 ⟹ 无缺口），对齐 Lean
//! `Origin.SegmentFeatureSeq.HasGap`，**不再 bit-exact 对齐 Python** `_is_fractal_and_gap`。
//! 裁定书：`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。
//! 本文件其余部分的 Python bit-exact 对齐声明不受影响。
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
//! 缺此核心。#1493 起第一种情况不再对齐该参考：互含处理与确认判据按 #1490 裁定（§7.1）直落原文。
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
//! 包含关系的」。#1490 裁定（§7.1 第 2/3/4 条）：转折点前后两元素 (a,b) 不做包含；c ⊂ b 记为 b 的
//! 窗成员；第一种情况唯一的确认判据是 `071:30/:32` 的先破判据。v1「先试不合并、以当下有无分型
//! 决定合并」在 `071:42` 两句上都做反（#1487 E1/E2），已删除。

use super::super::config::Tangency;
use super::super::types::{Direction, Stroke, Tick};
use serde::Serialize;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FeatElem {
    pub high: Tick,
    pub low: Tick,
    pub stroke_idx: usize,
}

/// #1404：在原状态机求值现场记录；笔索引由同次 StrokeFacts 绑定身份。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureElement {
    pub high: Tick,
    pub low: Tick,
    pub stroke_idx: usize,
    pub stroke_indices: Vec<usize>,
    /// #1490 §7.1 第 3 条：c ⊂ b（或有缺口的并列）时 c 记为 b 的窗成员，b 区间不动。
    pub window_members: Vec<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureStep {
    pub incoming: FeatureElement,
    pub before: Option<FeatureElement>,
    pub after: FeatureElement,
    pub action: &'static str,
    pub merge_direction: Option<&'static str>,
    pub scope: &'static str,
    /// #1490 §7.1 第 4 条：无缺口并列（`action="tie_pair"`）时与之并列的 b 的笔序号。
    pub tie_with: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureCheck {
    pub elements: [FeatureElement; 3],
    pub first_fractal: bool,
    /// 无分型时不适用，不填 false。
    pub gap: Option<bool>,
    pub gap_closed_by_c: Option<bool>,
    /// #1493：「先试不合并」已删除，恒为 false；保留字段以免旧线形读者断裂。
    pub provisional: bool,
    /// 第71课:40：假设转折点前/后的两元素是 a/b。`first_fractal` 或
    /// `turn_hypothesis_state != "NOT_CASE_ONE"` 时为 `Some([a, b])`，否则 `None`。
    pub turn_hypothesis_pair: Option<[usize; 2]>,
    /// #1490 §7.1 第 1 条：b 作第一种情况候选时的状态（`CONFIRMED`／`KILLED_BY_NEW_EXTREME`）；
    /// b 不是峰候选或 a、b 有缺口且未被 c 封闭（第二种情况）时为 `NOT_CASE_ONE`。
    /// 前缀末端尚无第三元素的候选不进本表，见 [`FeatureSequenceFacts::open_turn`]。
    pub turn_hypothesis_state: &'static str,
}
/// #1490 §7.1 第 1/3 条：前缀末端尚开着的第一种情况候选——末标准元素 b 是无缺口峰候选
/// （有窗成员或尚无第三元素），状态恒为 `OPEN_AT_PREFIX_END`，不发布为段端。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenTurn {
    pub elements: [FeatureElement; 2],
    pub turn_hypothesis_pair: [usize; 2],
    /// `OPEN_AT_PREFIX_END`；b 之后某根段方向笔已严格越过 P 而下一反向笔尚未进序列时为
    /// `KILLED_BY_NEW_EXTREME`（§7.1 第 1 条「某后续笔的高点 > b.high」），见证在 `killed_by`。
    pub turn_hypothesis_state: &'static str,
    pub killed_by: Option<TurnKill>,
}
/// 开窗候选作废见证：哪根段方向笔以哪个原始极值严格越过 P。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TurnKill {
    pub stroke_idx: usize,
    pub price: Tick,
    pub comparison: &'static str,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FeatureSequenceFacts {
    pub raw_elements: Vec<FeatureElement>,
    pub standard: Vec<FeatureElement>,
    pub steps: Vec<FeatureStep>,
    pub checks: Vec<FeatureCheck>,
    /// 最近一次追加后前缀末端的开窗候选；无则 `None`。
    pub open_turn: Option<OpenTurn>,
}

/// #1490 §7.1 第 1 条候选三态（命名沿用 #1344 §5.1）及「不是第一种情况候选」。
pub const TURN_CONFIRMED: &str = "CONFIRMED";
pub const TURN_KILLED: &str = "KILLED_BY_NEW_EXTREME";
pub const TURN_OPEN: &str = "OPEN_AT_PREFIX_END";
pub const TURN_NOT_CASE_ONE: &str = "NOT_CASE_ONE";

/// #1414：SECOND 的独立包含作用域与任意分型见证。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecondCheck {
    pub elements: [FeatureElement; 3],
    pub top_fractal: bool,
    pub bottom_fractal: bool,
    pub through_stroke: usize,
}
/// #1344 / xianduan §5.1：第二种候选语义终点的规则版本（随 S `rule_revision` 绑定）。
pub const SECOND_CANDIDATE_RULE_REVISION: &str = "second-candidate-terminal/1";
/// #1420 根裁定：第二特征序列扫描预算的具名版本——调度参数，不是语义。
///
/// ⚠ #1604 偏离登记：版本化记录已升 `second-seq-scan-budget/2`（加 `tail_window` 字段，第二序列窗口
/// 字段值不变）；本常数是 S 线形 `profile.budget_policy` 的已发布 token，改它须新 S rule_revision 与
/// JS/Python 校验器同步，留后续票处理。运行时所用记录版本见 `ParseConfig::scan_budget` 与
/// `segment::SegmentFacts::policy`。
pub const SECOND_SEQ_SCAN_BUDGET_POLICY: &str = "second-seq-scan-budget/1";

/// #1344 第一问：第二种候选 X（主序列分型极值 P）自 b 笔后逐笔推进的语义终点。
/// 只在扫描覆盖 `COMPLETE` 时成立；预算耗尽时语义状态为空（不是否定，A-ST-048-01）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecondTerminal {
    /// 第二特征序列先成分型（067:38）。
    Confirmed,
    /// 原方向价格先严格越过 P（078:52）；候选作废、不复活。
    KilledByNewExtreme,
    /// 到已知前缀末端二者皆未发生（065:28）；不是否定。
    OpenAtPrefixEnd,
}

/// #1344 第四问 / SPEC CC-014：扫描覆盖，与语义状态正交。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanCoverage {
    Complete,
    IncompleteResourceBound,
}

/// `KILLED_BY_NEW_EXTREME` 的见证：哪根同向笔以哪个原始极值严格越过 P。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecondKill {
    pub stroke_idx: usize,
    pub price: Tick,
    pub candidate_extreme: Tick,
    pub comparison: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecondSequenceFacts {
    pub raw_elements: Vec<FeatureElement>,
    pub standard: Vec<FeatureElement>,
    pub steps: Vec<FeatureStep>,
    pub checks: Vec<SecondCheck>,
    pub from_stroke: usize,
    pub through_stroke: usize,
    pub input_stroke_count: usize,
    /// 覆盖 = `INCOMPLETE_RESOURCE_BOUND` 的旧名（TB-02-D 线形保留）；失效时为 false。
    pub resource_truncated: bool,
    pub matched: Option<SecondCheck>,
    // #1420：以下语义字段随段合同 s-segment/3（rule_revision `…+segment-second/2`）进 S 线形；
    // s-segment/2 的 TB-02-D 已发布对象不回改（合同按 schema 分支校验）。
    /// 候选极值 P：主序列分型中心元素 b 的 high（向上段）/ low（向下段）。
    pub candidate_extreme: Tick,
    /// 越界水位：已消费同向笔原始极值的最远处（向上段取 max high，向下段取 min low）。
    pub breach_watermark: Option<Tick>,
    pub semantic_state: Option<SecondTerminal>,
    pub killed_by: Option<SecondKill>,
    pub coverage: ScanCoverage,
}

/// 笔的特征序列高低（= max/min(start_price, end_price)）。
pub(super) fn stroke_high_low(s: &Stroke) -> (Tick, Tick) {
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
///
/// #1604（#1586 A-12）：相切边界改读记录字段 `tangency`（`t`）；`closed` 即上述严格 `>`，`open` 为 `>=`。
#[allow(clippy::too_many_arguments)]
fn is_fractal_and_gap(
    a_h: Tick,
    a_l: Tick,
    b_h: Tick,
    b_l: Tick,
    c_h: Tick,
    c_l: Tick,
    seg_dir: Direction,
    t: Tangency,
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
                !t.meets(b_l, a_h)
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
                !t.meets(a_l, b_h)
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
pub(super) fn apply_inclusion(
    elements: &mut Vec<(Tick, Tick)>,
    h: Tick,
    l: Tick,
    dir_state: Option<Direction>,
) -> Option<Direction> {
    let (last_h, last_l) = *elements
        .last()
        .expect("apply_inclusion 调用前 elements 非空");
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
    candidate_extreme: Tick,
    scan_window: u32,
) -> bool {
    second_sequence(
        strokes,
        seg_dir,
        from_stroke_idx,
        candidate_extreme,
        scan_window,
    )
    .semantic_state
        == Some(SecondTerminal::Confirmed)
}

/// #1414：判定与证据来自同一遍扫描，正证据提前停，不把窗口耗尽当否定。
///
/// #1344 / #1420（xianduan §5.1）：语义终点 = 分型或越界先到者。逐笔推进，同一笔内先做包含
/// 处理与分型检查、后做越界检查（越界取该笔原始极值、严格不等）：先成分型 ⟹ `CONFIRMED`；
/// 原方向价格先严格越过 P ⟹ `KILLED_BY_NEW_EXTREME`（078:52，候选作废、不复活，失效即停）；
/// 已知前缀末端皆无 ⟹ `OPEN_AT_PREFIX_END`。预算 `scan_window` 只决定扫描覆盖，耗尽时语义为空。
/// 续扫与恢复用同一折叠的 K1 游标 [`super::second_fold::SecondFoldCursor`]，结果逐位相同。
pub(super) fn second_sequence(
    strokes: &[Stroke],
    seg_dir: Direction,
    from_stroke_idx: usize,
    candidate_extreme: Tick,
    scan_window: u32,
) -> SecondSequenceFacts {
    // #1420：折叠本体在 `second_fold`（K1 游标与本入口共用同一步进函数）。
    super::second_fold::fold_uninterrupted(
        strokes,
        super::second_fold::SecondCandidateId {
            seg_dir,
            b_stroke: from_stroke_idx,
            candidate_extreme,
        },
        scan_window,
    )
}

/// 峰候选：b 比其前一标准元素 a 在段方向更极端（Up：`b.high > a.high`；Down：`b.low < a.low`）。
fn is_peak(a: FeatElem, b: FeatElem, seg_dir: Direction) -> bool {
    match seg_dir {
        Direction::Up => b.high > a.high,
        Direction::Down => b.low < a.low,
    }
}

/// a、b 之间的缺口（`071:40`），与 `is_fractal_and_gap` 读同一个 `tangency`（#246 / #1604）。
fn gap_between(a: FeatElem, b: FeatElem, seg_dir: Direction, t: Tangency) -> bool {
    match seg_dir {
        Direction::Up => !t.meets(b.low, a.high),
        Direction::Down => !t.meets(a.low, b.high),
    }
}

/// c 是否封闭 a、b 之间的缺口（c 与 a 重合，`067:196`）；与缺口谓词读同一个 `tangency`（#1604）。
fn closed_by_c(a: FeatElem, c: FeatElem, seg_dir: Direction, t: Tangency) -> bool {
    match seg_dir {
        Direction::Up => t.meets(c.low, a.high),
        Direction::Down => t.meets(a.low, c.high),
    }
}

#[cfg(test)]
thread_local! {
    /// TestOnly（#1493 义务 8(e) 性质锁）：第一种情况改用 067 严格分型谓词作对照臂。
    /// 不参与生产准入（#799 限定词 2/3）。
    pub(super) static CASE_ONE_ORACLE_067: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// TestOnly（#1493 裁定 2 落地前置的三方对照）：有缺口峰候选 b 包含入元素时的对照臂。
    /// 0 = 本裁定（G-2 合并、代表笔保持 b）；1 = 窗成员（分支旧 R2）；2 = 旧 G-2（代表笔改为入元素）。
    /// 不参与生产准入（#799 限定词 3）。
    pub(super) static GAP_INCLUSION_ARM: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    /// TestOnly（#1493 裁定 1 对照见证）：真 = 旧规则「第二种候选 OPEN 时跳过、续判同段其后候选」。
    /// 只用于证明 OPEN 封段见证非空转；不参与生产准入（#799 限定词 3）。
    pub(super) static OPEN_SKIP_ARM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 有缺口峰候选包含入元素时的处理臂；生产恒为 0（#1493 裁定 2）。
fn gap_inclusion_arm() -> u8 {
    #[cfg(test)]
    {
        GAP_INCLUSION_ARM.with(|a| a.get())
    }
    #[cfg(not(test))]
    {
        0
    }
}

/// #1490 §7.1 第 4 条 先破判据（`071:30/:32`）：第一种情况候选 b 的唯一确认判据。
/// Up：`b.high > a.high ∧ c.high ≤ b.high ∧ c.low < b.low`；Down 镜像。破 = 严格越过，相等不是破。
fn case_one_confirms(w: [FeatElem; 3], seg_dir: Direction) -> bool {
    let [a, b, c] = w;
    #[cfg(test)]
    if CASE_ONE_ORACLE_067.with(|o| o.get()) {
        let t = super::super::config::ParseConfig::default().tangency;
        return is_fractal_and_gap(a.high, a.low, b.high, b.low, c.high, c.low, seg_dir, t).0;
    }
    match seg_dir {
        Direction::Up => b.high > a.high && c.high <= b.high && c.low < b.low,
        Direction::Down => b.low < a.low && c.low >= b.low && c.high > b.high,
    }
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
    /// 实际终结证据的前沿，与几何端 k-1 分开。
    pub through_stroke: usize,
    /// #1493：命中的分型三元素 (a,b,c)（冻结窗优先），供段端门 fail-closed 的最小复现。
    pub window: [FeatElem; 3],
}

/// 增量特征序列状态机（第一种情况按 #1490 §7.1；Python `_FeatureSeqState` 为历史参考）。
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
    /// 尾窗扫描大小（0 = 无限）。#1604：值来自 `second-seq-scan-budget` 记录（原代码常数 7）。
    tail_window: u32,
    /// 相切口径（#1604，记录 `segment-second.tangency`）：缺口与「c 封闭缺口」两谓词共读。
    tangency: Tangency,
    /// 尾窗挤出探查的进度：`[.., probed_upto)` 的标准元素下标已探查过（见 [`Self::probe_tail_excluded`]）。
    probed_upto: usize,
    /// #1604（#1586 C-04）：本段有可准入候选被尾窗挤出、未按定型窗口判过 ⟹ 输出覆盖
    /// `INCOMPLETE_RESOURCE_BOUND`。粘性，`reset` 清零。`tail_window >= 3` 时恒假。
    tail_truncated: bool,
    /// 第二特征序列扫描窗口（config second_seq_scan_window，0=无限）。
    second_seq_window: u32,
    /// #88 frontier 修复：本段扫描期间是否跳过过**尚可改判**的 SecondKind 候选。`reset` 清零
    /// （每段独立），跳过时置真（段内累积）。增量层 `IncrSegments::append` 读此标志把 seg_start 记入
    /// `earliest_unsealed_from`（检查点安全边界）。#1344：`OPEN_AT_PREFIX_END` 与预算耗尽的候选
    /// 未来可确认或失效 ⟹ 置真；`KILLED_BY_NEW_EXTREME` 不复活 ⟹ 不置真——例外是失效笔仍是
    /// 输入末笔（末笔可改写，失效尚可撤销）；`CONFIRMED` 的确认笔仍是输入末笔时同理置真（#1420）。
    skipped_secondkind: bool,
    /// 未决阻断（#1420 A-ST-048-01；#1493 裁定 1 / `xianduan.md` §7.1 第 8 条）：本段已遇语义状态
    /// 未决的第二种候选——`OPEN_AT_PREFIX_END` 或预算耗尽（二者同一路径、同一标志）。未决不是否定，
    /// 本段其后的候选一律不判，直到该候选在新输入上确认或失效；标志粘住，故尾窗 `tail_window` 的扫描起点
    /// 截断不能把它挪出去（尾窗是调度截断，不截断未决阻断）。`reset` 清零，增量层每次 append 新建
    /// 状态机，故不跨段、不跨扫描。
    unresolved_blocked: bool,
    /// #1420：S 层的每候选游标簿（K1，跨轮续扫）；`None` = 按 `second_seq_window` 无中断折叠。
    second_store: Option<super::second_fold::SecondFoldStore>,
    facts: Option<FeatureSequenceFacts>,
    second: Option<SecondSequenceFacts>,
    // #1414 / 067:196：同b的首次FIRST c，不能吸收后来的封闭而倒改第一种。
    frozen_first: std::collections::BTreeMap<usize, [FeatElem; 3]>,
    frozen_first_facts: std::collections::BTreeMap<usize, [FeatureElement; 3]>,
}

impl FeatureSeqState {
    /// 新建状态机。`seg_dir` = 线段方向。dir_state 初值对齐 Python（向下段 Down，向上 None）。
    pub fn new(
        seg_dir: Direction,
        extend_mode: ExtendMode,
        tail_window: u32,
        second_seq_window: u32,
        tangency: Tangency,
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
            tangency,
            probed_upto: 0,
            tail_truncated: false,
            second_seq_window,
            skipped_secondkind: false,
            unresolved_blocked: false,
            second_store: None,
            facts: None,
            second: None,
            frozen_first: Default::default(),
            frozen_first_facts: Default::default(),
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
        self.unresolved_blocked = false;
        self.probed_upto = 0;
        self.tail_truncated = false;
        self.second = None;
        self.frozen_first.clear();
        self.frozen_first_facts.clear();
        if self.facts.is_some() {
            self.facts = Some(FeatureSequenceFacts::default());
        }
    }

    pub fn observe(&mut self) {
        self.facts = Some(FeatureSequenceFacts::default());
    }

    /// #1420：挂上游标簿后，第二种候选经簿内 K1 游标折叠（预算 = 游标累计获批预算）。
    pub fn set_second_store(&mut self, store: Option<super::second_fold::SecondFoldStore>) {
        self.second_store = store;
    }

    pub fn take_second_store(&mut self) -> Option<super::second_fold::SecondFoldStore> {
        self.second_store.take()
    }

    pub fn facts(&self) -> FeatureSequenceFacts {
        self.facts.clone().unwrap_or_default()
    }

    pub fn second_facts(&self) -> Option<SecondSequenceFacts> {
        self.second.clone()
    }

    /// 本状态机所读的相切口径（段层事实输出与构造门同读，#1604）。
    pub fn tangency(&self) -> Tangency {
        self.tangency
    }

    /// #1604：本段是否有可准入候选被尾窗挤出（资源上界，见 [`Self::probe_tail_excluded`]）。
    pub fn tail_truncated(&self) -> bool {
        self.tail_truncated
    }

    /// #88：本段扫描期间是否跳过过 SecondKind 候选（unsealed 起点信号，增量层读取）。
    pub fn skipped_secondkind(&self) -> bool {
        self.skipped_secondkind
    }

    /// 标记跳过 stroke_idx <= 此值的分型（Python `skip_trigger`，:295-301）。
    pub fn skip_trigger(&mut self, stroke_idx: usize) {
        self.skip_until_stroke = stroke_idx as i64;
        self.frozen_first.retain(|k, _| *k > stroke_idx);
        self.frozen_first_facts.retain(|k, _| *k > stroke_idx);
    }

    /// 增量添加一个反向笔（第67/71课；#1490 裁定 `xianduan.md` §7.1 第 2/3/4 条直落，#1493）。
    ///
    /// 非包含 ⟹ 追加并更新方向态。与末标准元素互含时按三路分派（Up 段写法，Down 段镜像）：
    /// - R1：入元素 ⊃ 末元素且在段方向更极端（`high > last.high`）⟹ 追加、不合并，二者是可能的
    ///   转折点前后两元素 (a,b)（`071:42` 第一句），`action="boundary_pair"`；
    /// - 并列：入元素 ⊃ 末元素、段方向极值相等、另一侧严格越过、末元素是峰候选：a、b 无缺口 ⟹
    ///   追加并标 `tie_with`（由先破判据确认），`action="tie_pair"`；a、b 有缺口 ⟹ 同 R2；
    /// - R2：入元素 ⊂ 末元素且末元素是峰候选（比其前一标准元素在段方向更极端）⟹ 记为末元素的
    ///   窗成员（`071:32`），末元素区间与笔号不变，`action="window_member"`；
    /// - R3：其余互含（末元素不是峰候选）⟹ G-2 方向性合并（`xianduan.md` §2）。
    ///
    /// 与入元素完全相同的元素既不越过 P 也不破 b 的终点，按 R2 记为窗成员（§7.1 第 1 条「相等不是破」）。
    /// v1「先试不合并、以当下有无分型决定合并」已删除（#1487 E1/E2）；Python v1 为历史参考。
    pub fn append(&mut self, stroke_idx: usize, high: Tick, low: Tick, strokes: &[Stroke]) {
        let _ = strokes;
        let incoming = FeatureElement {
            high,
            low,
            stroke_idx,
            stroke_indices: if self.facts.is_some() {
                vec![stroke_idx]
            } else {
                Vec::new()
            },
            window_members: Vec::new(),
        };
        let before = self.facts.as_ref().and_then(|f| f.standard.last().cloned());
        if let Some(f) = &mut self.facts {
            f.raw_elements.push(incoming.clone());
        }
        if self.std.is_empty() {
            self.std.push(FeatElem {
                high,
                low,
                stroke_idx,
            });
            self.record_step(incoming, before, "append", None, "SAME_SEQUENCE", None);
            self.refresh_open_turn();
            return;
        }
        let last = *self.std.last().unwrap();
        let (last_h, last_l) = (last.high, last.low);
        let left_inc = last_h >= high && last_l <= low;
        let right_inc = high >= last_h && low <= last_l;
        let has_inclusion = left_inc || right_inc;

        if has_inclusion {
            let up = self.seg_dir == Direction::Up;
            // 入元素在段方向上更极端 / 相等（Up 看 high，Down 看 low）。
            let more_extreme = if up { high > last_h } else { low < last_l };
            let tie_extreme = if up { high == last_h } else { low == last_l };
            let peak = self.last_is_peak();
            let gap_ab = peak && self.last_has_gap();
            if right_inc && more_extreme {
                // R1：可能的转折点中元素，与末元素为 (a,b)，不合并（071:42 第一句）。
                self.std.push(FeatElem {
                    high,
                    low,
                    stroke_idx,
                });
                self.record_step(
                    incoming,
                    before,
                    "boundary_pair",
                    None,
                    "TURN_HYPOTHESIS",
                    None,
                );
            } else if right_inc && tie_extreme && !left_inc && peak && !gap_ab {
                // 并列（无缺口）：P 侧相等不是越过，另一侧严格越过 b 终点 ⟹ 交先破判据确认。
                self.std.push(FeatElem {
                    high,
                    low,
                    stroke_idx,
                });
                self.record_step(
                    incoming,
                    before,
                    "tie_pair",
                    None,
                    "TURN_HYPOTHESIS",
                    Some(last.stroke_idx),
                );
            } else if peak && (!gap_ab || !left_inc || gap_inclusion_arm() == 1) {
                // R2：第一种情况（a、b 无缺口）c ⊂ b ⟹ b 的窗成员，b 区间不动（071:32，§7.1 第 3 条）；
                // 有缺口的并列（入元素 ⊋ b、P 侧相等）同记窗成员（§7.1 第 4 条）。
                self.record_step(
                    incoming,
                    before,
                    "window_member",
                    None,
                    "TURN_HYPOTHESIS",
                    None,
                );
            } else {
                // R3：末元素不是峰候选 ⟹ G-2 方向性合并，代表笔改为入元素。
                // #1493 裁定 2（§7.1 第 4 条）：有缺口的峰候选 b 包含入元素（`left_inc`）⟹ 同样按 G-2
                // 方向性合并区间（`067:20`、`071:42` 第二句），但代表笔保持 b——段端与第二序列起点须是
                // 携带段方向极值的笔（`067:38`）；合并取 min/max 不改 b 的 P 侧极值。
                let keep_b = peak && gap_inclusion_arm() != 2;
                let effective_up = self.dir_state != Some(Direction::Down);
                let last_mut = self.std.last_mut().unwrap();
                if effective_up {
                    last_mut.high = last_h.max(high);
                    last_mut.low = last_l.max(low);
                } else {
                    last_mut.high = last_h.min(high);
                    last_mut.low = last_l.min(low);
                }
                if !keep_b {
                    last_mut.stroke_idx = stroke_idx;
                }
                self.last_checked = self.std.len().saturating_sub(3);
                self.record_step(
                    incoming,
                    before,
                    "merge",
                    Some(if effective_up { "UP" } else { "DOWN" }),
                    "SAME_SEQUENCE",
                    None,
                );
            }
        } else {
            if high > last_h && low > last_l {
                self.dir_state = Some(Direction::Up);
            } else if high < last_h && low < last_l {
                self.dir_state = Some(Direction::Down);
            }
            self.std.push(FeatElem {
                high,
                low,
                stroke_idx,
            });
            self.record_step(incoming, before, "append", None, "SAME_SEQUENCE", None);
        }
        self.refresh_open_turn();
    }

    /// 末标准元素是否峰候选：比其前一标准元素在段方向更极端（Up：high 更高；Down：low 更低）。
    fn last_is_peak(&self) -> bool {
        let n = self.std.len();
        n >= 2 && is_peak(self.std[n - 2], self.std[n - 1], self.seg_dir)
    }

    /// 末两标准元素 (a,b) 之间是否有缺口（`071:40`；与 `is_fractal_and_gap` 同为严格 `>`）。
    fn last_has_gap(&self) -> bool {
        let n = self.std.len();
        n >= 2
            && gap_between(
                self.std[n - 2],
                self.std[n - 1],
                self.seg_dir,
                self.tangency,
            )
    }

    /// #1490 §7.1 第 1 条：末标准元素 b 是无缺口峰候选（有窗成员或尚无第三元素）⟹ `OPEN_AT_PREFIX_END`。
    fn refresh_open_turn(&mut self) {
        let open = self.last_is_peak()
            && !self.last_has_gap()
            && (self.std.last().unwrap().stroke_idx as i64) > self.skip_until_stroke;
        if let Some(f) = &mut self.facts {
            let n = f.standard.len();
            f.open_turn = open.then(|| {
                let (a, b) = (f.standard[n - 2].clone(), f.standard[n - 1].clone());
                OpenTurn {
                    turn_hypothesis_pair: [a.stroke_idx, b.stroke_idx],
                    elements: [a, b],
                    turn_hypothesis_state: TURN_OPEN,
                    killed_by: None,
                }
            });
        }
    }

    /// 段方向笔（不进 FIRST 序列）只用于开窗候选的作废见证：严格越过 P ⟹ `KILLED_BY_NEW_EXTREME`。
    /// 只改事实，不改准入（下一反向笔进序列时 R1/追加即作废同一候选）。
    pub fn observe_trend_stroke(&mut self, stroke_idx: usize, high: Tick, low: Tick) {
        let up = self.seg_dir == Direction::Up;
        if let Some(o) = self.facts.as_mut().and_then(|f| f.open_turn.as_mut()) {
            let b = &o.elements[1];
            let (beyond, price) = if up {
                (high > b.high, high)
            } else {
                (low < b.low, low)
            };
            if o.killed_by.is_none() && stroke_idx > b.stroke_idx && beyond {
                o.turn_hypothesis_state = TURN_KILLED;
                o.killed_by = Some(TurnKill {
                    stroke_idx,
                    price,
                    comparison: if up { ">" } else { "<" },
                });
            }
        }
    }

    fn record_step(
        &mut self,
        incoming: FeatureElement,
        before: Option<FeatureElement>,
        action: &'static str,
        merge_direction: Option<&'static str>,
        scope: &'static str,
        tie_with: Option<usize>,
    ) {
        if let Some(f) = &mut self.facts {
            let after = if action == "window_member" {
                let b = f.standard.last_mut().unwrap();
                b.window_members.push(incoming.stroke_idx);
                b.clone()
            } else {
                let elem = self.std.last().unwrap();
                let (mut stroke_indices, window_members) = if action == "merge" {
                    let popped = f.standard.pop().unwrap();
                    (popped.stroke_indices, popped.window_members)
                } else {
                    (vec![], vec![])
                };
                stroke_indices.push(incoming.stroke_idx);
                let after = FeatureElement {
                    high: elem.high,
                    low: elem.low,
                    stroke_idx: elem.stroke_idx,
                    stroke_indices,
                    window_members,
                };
                f.standard.push(after.clone());
                after
            };
            f.steps.push(FeatureStep {
                incoming,
                before,
                after,
                action,
                merge_direction,
                scope,
                tie_with,
            });
        }
    }

    /// #1604（#1586 C-04）尾窗挤出探查：本次扫描起点 `tail_lo` 把 `[.., tail_lo)` 挤出窗外；对其中
    /// 尚未探查过的下标，按主循环同一判定（跳过集、冻结窗、峰候选、缺口与封闭、先破判据、第二种
    /// 候选的语义终点）纯判断一次，不改任何准入状态。窗口已定型（后一元素不是末元素）的下标只探一次；
    /// 窗口含末元素的下标 `n-2`（全扫此刻也按这个未定型窗口判，确认时才冻结，#1493）每次都重探。
    /// 可准入且不是 `KILLED_BY_NEW_EXTREME` ⟹ 该候选按全扫会确认或封段，却被尾窗挤掉 ⟹ 置
    /// `tail_truncated`（输出覆盖 `INCOMPLETE_RESOURCE_BOUND`）。
    ///
    /// 结构事实：标准序列只有末元素可变（合并 / R1 追加），故下标 `i` 的窗口 `(i-1,i,i+1)` 在长度
    /// 达到 `i+3` 时定型；那一次扫描的起点 `≤ max(last_checked, i+3-tail_window)`，`tail_window ≥ 3`
    /// 时 `i` 必在窗内且按定型窗口判过，判定结果之后不变（可准入非失效 ⟹ 命中复位或粘性封段）。
    /// 故 `tail_window ≥ 3` 与 0（全扫）的判定结果逐位相同、本探查恒不置位；`1`、`2` 才会挤掉未判
    /// 候选。（诊断轨迹 `FeatureCheck` 不在此列：全扫会重复求值已判过的峰，多记几条重复检查。）
    /// 第二种候选的终点按全扫（窗口 0）判：只问语义上是否失效，不让第二序列预算混入尾窗标记。
    fn probe_tail_excluded(&mut self, strokes: &[Stroke], tail_lo: usize) {
        let n = self.std.len();
        let finalized_end = tail_lo.min(n.saturating_sub(2));
        let end = tail_lo.min(n.saturating_sub(1));
        let from = 1.max(self.last_checked).max(self.probed_upto);
        if self.tail_truncated || from >= end {
            self.probed_upto = self.probed_upto.max(finalized_end);
            return;
        }
        for i in from..end {
            let b_stroke = self.std[i].stroke_idx;
            if (b_stroke as i64) <= self.skip_until_stroke {
                continue;
            }
            let window = self.frozen_first.get(&b_stroke).copied().unwrap_or([
                self.std[i - 1],
                self.std[i],
                self.std[i + 1],
            ]);
            let [a, b, c] = window;
            if !is_peak(a, b, self.seg_dir) {
                continue;
            }
            let raw_gap = gap_between(a, b, self.seg_dir, self.tangency);
            let case_two = raw_gap
                && (self.extend_mode == ExtendMode::Optimized
                    || !closed_by_c(a, c, self.seg_dir, self.tangency));
            let material = if case_two {
                // 分型谓词同 `is_fractal_and_gap`；不经它调用，免得探查计入 #246 相切探针计数。
                let is_fractal = match self.seg_dir {
                    Direction::Up => b.high > a.high && b.high > c.high,
                    Direction::Down => b.low < a.low && b.low < c.low,
                };
                if !is_fractal {
                    continue;
                }
                let candidate_extreme = match self.seg_dir {
                    Direction::Up => b.high,
                    Direction::Down => b.low,
                };
                let second = second_sequence(strokes, self.seg_dir, b_stroke, candidate_extreme, 0);
                // 失效笔仍是输入末笔 ⟹ 失效尚可撤销，本段保持 unsealed（同主循环规则），增量层重扫时重探。
                if second
                    .killed_by
                    .as_ref()
                    .is_some_and(|k| k.stroke_idx + 1 >= strokes.len())
                {
                    self.skipped_secondkind = true;
                }
                second.semantic_state != Some(SecondTerminal::KilledByNewExtreme)
            } else {
                case_one_confirms(window, self.seg_dir)
            };
            if material {
                // 被挤掉的候选未来可能失效（标记随之撤销）⟹ 本段保持 unsealed，增量层与全量同判（#804）。
                self.tail_truncated = true;
                self.skipped_secondkind = true;
                break;
            }
        }
        self.probed_upto = self.probed_upto.max(finalized_end);
    }

    /// 从 last_checked 向后扫描（受尾窗限制），找第一个可准入的 FIRST 分型。
    ///
    /// 向上段看顶、向下段看底；跳过 stroke_idx <= skip_until_stroke 的候选。对每个峰候选 b（前一
    /// 标准元素 a、后一标准元素 c）：
    /// - a、b 无缺口，或缺口被 c 封闭（strict）⟹ 第一种情况，唯一的确认判据是 `071:30/:32` 的
    ///   **先破判据**（Up：`b.high > a.high ∧ c.high ≤ b.high ∧ c.low < b.low`，Down 镜像；相等不是破）
    ///   ⟹ `CONFIRMED`；c 在段方向严格越过 P ⟹ `KILLED_BY_NEW_EXTREME`（不复活，旧段延续）；
    /// - a、b 有缺口且 c 未封闭 ⟹ 第二种情况，仍按 067 严格分型谓词 `is_fractal_and_gap` + 第二特征
    ///   序列（#1344/#1420），一字不改；
    /// - 末元素 b 尚无第三元素 ⟹ `OPEN_AT_PREFIX_END`，记在 `FeatureSequenceFacts.open_turn`。
    ///
    /// 同段候选按序判定（#1493 裁定 1，§7.1 第 8 条）：第二种候选的第二序列未决（`OPEN_AT_PREFIX_END`
    /// 或预算耗尽）⟹ 本段在该候选处封住（`unresolved_blocked`，粘性），其后的候选一律不判，直到它
    /// 在新输入上确认（段终于其 P）或失效（`078:52`，扫描继续向后）。
    pub fn scan_trigger(&mut self, strokes: &[Stroke]) -> Option<TriggerHit> {
        let n = self.std.len();
        if n < 3 || self.unresolved_blocked {
            return None;
        }
        // start = max(1, last_checked, n - tail_window)。tail_window=0 → 无尾窗下限。
        let tail_lo = if self.tail_window == 0 {
            0
        } else {
            n.saturating_sub(self.tail_window as usize)
        };
        self.probe_tail_excluded(strokes, tail_lo);
        let start = 1.max(self.last_checked).max(tail_lo);
        for i in start..n - 1 {
            let b_stroke = self.std[i].stroke_idx;
            if (b_stroke as i64) <= self.skip_until_stroke {
                continue;
            }
            let window = self.frozen_first.get(&b_stroke).copied().unwrap_or([
                self.std[i - 1],
                self.std[i],
                self.std[i + 1],
            ]);
            let (a_h, a_l) = (window[0].high, window[0].low);
            let (b_h, b_l) = (window[1].high, window[1].low);
            let (c_h, c_l) = (window[2].high, window[2].low);
            // 第二种情况的 067 严格谓词（含 #246 缺口口径）一字不改。
            let (is_fractal, _) =
                is_fractal_and_gap(a_h, a_l, b_h, b_l, c_h, c_l, self.seg_dir, self.tangency);
            let peak = is_peak(window[0], window[1], self.seg_dir);
            let raw_gap = peak && gap_between(window[0], window[1], self.seg_dir, self.tangency);
            let closed_by_c = closed_by_c(window[0], window[2], self.seg_dir, self.tangency);
            let case_two = raw_gap && (self.extend_mode == ExtendMode::Optimized || !closed_by_c);
            let (admissible, state) = if !peak {
                (false, TURN_NOT_CASE_ONE)
            } else if case_two {
                (is_fractal, TURN_NOT_CASE_ONE)
            } else if case_one_confirms(window, self.seg_dir) {
                (true, TURN_CONFIRMED)
            } else if match self.seg_dir {
                Direction::Up => c_h > b_h,
                Direction::Down => c_l < b_l,
            } {
                (false, TURN_KILLED)
            } else {
                // 标准序列里峰候选之后的元素不会落在 b 范围内（R2）；冻结窗保证确认后不被改写。
                (false, TURN_NOT_CASE_ONE)
            };
            let gap_closed_by_c = (admissible && raw_gap).then_some(closed_by_c);
            // #1414 / 067:196：同 b 的首次 FIRST c 冻结；#1493：窗 OPEN 期间（尚无第三元素）不冻结，确认时才冻结。
            if admissible {
                self.frozen_first.entry(b_stroke).or_insert(window);
            }
            if let Some(f) = &mut self.facts {
                let elements = self.frozen_first_facts.get(&b_stroke).cloned().unwrap_or([
                    f.standard[i - 1].clone(),
                    f.standard[i].clone(),
                    f.standard[i + 1].clone(),
                ]);
                if admissible {
                    self.frozen_first_facts
                        .entry(b_stroke)
                        .or_insert(elements.clone());
                }
                let check = FeatureCheck {
                    elements,
                    first_fractal: admissible,
                    gap: admissible.then_some(raw_gap),
                    gap_closed_by_c,
                    provisional: false,
                    turn_hypothesis_pair: (admissible || state != TURN_NOT_CASE_ONE)
                        .then_some([window[0].stroke_idx, window[1].stroke_idx]),
                    turn_hypothesis_state: state,
                };
                if f.checks.last() != Some(&check) {
                    f.checks.push(check);
                }
            }
            if !admissible {
                continue;
            }
            let has_gap = case_two;
            // 有缺口须第二特征序列出现分型才确认（#1344/#1420，不改）。
            // #1344：P = 主序列分型中心元素 b 的极值（冻结窗口优先，同上 FIRST 判定）。
            let candidate_extreme = match self.seg_dir {
                Direction::Up => b_h,
                Direction::Down => b_l,
            };
            let second = has_gap.then(|| match self.second_store.as_mut() {
                Some(store) => store.fold(
                    super::second_fold::SecondCandidateId {
                        seg_dir: self.seg_dir,
                        b_stroke,
                        candidate_extreme,
                    },
                    strokes,
                ),
                None => second_sequence(
                    strokes,
                    self.seg_dir,
                    b_stroke,
                    candidate_extreme,
                    self.second_seq_window,
                ),
            });
            let through_stroke = second.as_ref().map_or(window[2].stroke_idx, |s| {
                s.through_stroke.max(window[2].stroke_idx)
            });
            self.second = second.clone();
            match second.as_ref().map(|s| (s.semantic_state, s)) {
                None => {}
                Some((Some(SecondTerminal::Confirmed), s)) => {
                    // #1420（K1 等价条件：末笔可改写）：确认笔仍是输入末笔时，确认尚可撤销 ⟹
                    // 本段保持 unsealed，与失效同一规则（三路锁 tb02w_three_way 的末笔回缩反例）。
                    if s.matched
                        .as_ref()
                        .is_some_and(|m| m.through_stroke + 1 >= strokes.len())
                    {
                        self.skipped_secondkind = true;
                    }
                }
                Some((Some(SecondTerminal::KilledByNewExtreme), s)) => {
                    // #1344：失效候选作废、不复活（078:52），段继续；失效笔仍是输入末笔时末笔可改写，
                    // 失效尚可撤销 ⟹ 本段保持 unsealed，否则失效终局，检查点安全边界前移。
                    if s.killed_by
                        .as_ref()
                        .is_some_and(|k| k.stroke_idx + 1 >= strokes.len())
                    {
                        self.skipped_secondkind = true;
                    }
                    continue;
                }
                #[cfg(test)]
                Some((Some(SecondTerminal::OpenAtPrefixEnd), _))
                    if OPEN_SKIP_ARM.with(|a| a.get()) =>
                {
                    self.skipped_secondkind = true;
                    continue;
                }
                Some((Some(SecondTerminal::OpenAtPrefixEnd) | None, s)) => {
                    // #1493 裁定 1（§7.1 第 8 条）：OPEN 与预算耗尽（A-ST-048-01 / G-007）都是「未决」，
                    // 不是否定（`071:38`「不满足」才「原来的线段依然延续」）——不得跳过去判同段其后的
                    // 候选，封住本段（粘性标志）并留下该候选的扫描事实；本段 seg_start unsealed，增量层
                    // 回退到其前重扫。
                    self.skipped_secondkind = true;
                    self.unresolved_blocked = true;
                    self.second = Some(s.clone());
                    return None;
                }
            }
            self.last_checked = i.saturating_sub(1);
            return Some(TriggerHit {
                k: b_stroke,
                gap_second: has_gap,
                through_stroke,
                window,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn is_fractal_and_gap_up_top_with_gap() {
        // 向上段：b_h 最高 → 顶分型；b_l > a_h → 缺口（#246 严格）。
        let (f, g) = is_fractal_and_gap(10, 5, 20, 12, 8, 3, Direction::Up, Tangency::Closed);
        assert!(f); // 20 > 10 && 20 > 8
        assert!(g); // b_l=12 > a_h=10（严格）
    }

    #[test]
    fn is_fractal_and_gap_up_top_no_gap() {
        // 顶分型但 b_l < a_h → 无缺口（第一种情况）。
        let (f, g) = is_fractal_and_gap(10, 5, 20, 8, 8, 3, Direction::Up, Tangency::Closed);
        assert!(f);
        assert!(!g); // b_l=8 < a_h=10
    }

    #[test]
    fn is_fractal_and_gap_down_bottom() {
        // 向下段：b_l 最低 → 底分型；a_l > b_h → 缺口（#246 严格）。
        let (f, g) = is_fractal_and_gap(15, 12, 8, 3, 18, 14, Direction::Down, Tangency::Closed);
        assert!(f); // 3 < 12 && 3 < 14
        assert!(g); // a_l=12 > b_h=8（严格）
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
            let (f_up, g_up) = is_fractal_and_gap(
                lo_h,
                lo_l,
                up_h,
                up_l,
                lo_h,
                lo_l,
                Direction::Up,
                Tangency::Closed,
            );
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
            let (f_dn, g_dn) = is_fractal_and_gap(
                up_h,
                up_l,
                lo_h,
                lo_l,
                up_h,
                up_l,
                Direction::Down,
                Tangency::Closed,
            );
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

    // ★相切用例（#246 裁定：相切=重合 ⟹ 无缺口，supersede Lead #84 点3）。
    // 注意：上述既有用例全是严格不等构造，对本口径变化不敏感；以下相切用例才是本次口径的保障。

    #[test]
    fn is_fractal_and_gap_tangent_up_no_gap() {
        // 向上段顶分型，b 与 a 仅公共端点（b_l == a_h == 10，相切）。
        // 相切=重合 ⟹ 无缺口：has_gap 必须为 false。
        let (f, g) = is_fractal_and_gap(10, 5, 20, 10, 8, 3, Direction::Up, Tangency::Closed);
        assert!(f); // 20 > 10 && 20 > 8
        assert!(!g); // b_l=10 相切 a_h=10 → 有重合 → 无缺口
    }

    #[test]
    fn is_fractal_and_gap_tangent_down_no_gap() {
        // 向下段底分型，a 与 b 仅公共端点（a_l == b_h == 10，相切）。
        // 相切=重合 ⟹ 无缺口：has_gap 必须为 false。
        let (f, g) = is_fractal_and_gap(15, 10, 10, 3, 18, 14, Direction::Down, Tangency::Closed);
        assert!(f); // 3 < 10 && 3 < 14
        assert!(!g); // a_l=10 相切 b_h=10 → 有重合 → 无缺口
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
        let mut st =
            FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0, Tangency::Closed);
        st.append(0, 20, 10, &[]);
        assert_eq!(st.std.len(), 1);
    }

    /// #1493 义务 3（#1414 frozen_first）：窗 OPEN 期间（b 有窗成员、尚无第三元素）不冻结以 b 为中元素的
    /// FIRST 三元素，确认时才冻结。
    #[test]
    fn frozen_first_waits_for_confirmation() {
        let mut st =
            FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0, Tangency::Closed);
        st.observe();
        for (i, h, l) in [(1, 10, 4), (3, 20, 5), (5, 18, 7)] {
            st.append(i, h, l, &[]);
            assert_eq!(st.scan_trigger(&[]), None);
        }
        assert!(!st.frozen_first.contains_key(&3), "窗 OPEN 期间不得冻结");
        assert_eq!(st.facts().standard[1].window_members, vec![5]);
        assert_eq!(st.facts().open_turn.unwrap().turn_hypothesis_pair, [1, 3]);
        st.append(7, 15, 3, &[]);
        assert_eq!(st.scan_trigger(&[]).map(|h| h.k), Some(3));
        assert_eq!(
            st.frozen_first.get(&3).map(|w| w.map(|e| e.stroke_idx)),
            Some([1, 3, 7])
        );
        assert_eq!(
            st.facts().checks.last().unwrap().turn_hypothesis_state,
            TURN_CONFIRMED
        );
    }

    #[test]
    fn scan_trigger_too_few_none() {
        let mut st =
            FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0, Tangency::Closed);
        st.append(0, 20, 10, &[]);
        st.append(2, 25, 15, &[]);
        assert_eq!(st.scan_trigger(&[]), None); // < 3 元素
    }

    #[test]
    fn scan_trigger_finds_top_no_gap() {
        // 向上段特征序列 [10,5],[20,8],[8,3]：中间 high=20 最高 → 顶分型；b_l=8<a_h=10 无缺口。
        let mut st =
            FeatureSeqState::new(Direction::Up, ExtendMode::Strict, 0, 0, Tangency::Closed);
        st.append(1, 10, 5, &[]);
        st.append(3, 20, 8, &[]);
        st.append(5, 8, 3, &[]);
        let hit = st.scan_trigger(&[]);
        assert_eq!(
            hit,
            Some(TriggerHit {
                k: 3,
                through_stroke: 5,
                gap_second: false,
                window: [
                    FeatElem {
                        high: 10,
                        low: 5,
                        stroke_idx: 1
                    },
                    FeatElem {
                        high: 20,
                        low: 8,
                        stroke_idx: 3
                    },
                    FeatElem {
                        high: 8,
                        low: 3,
                        stroke_idx: 5
                    },
                ],
            })
        );
    }

    #[test]
    fn second_seq_window_unlimited_vs_bounded() {
        // 仅验证窗口参数生效不 panic：无同向笔 → 无分型。
        let strokes = vec![stroke(Direction::Up, 0, 4, 5, 20)];
        assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 20, 0));
        assert!(!second_seq_has_fractal(&strokes, Direction::Up, 0, 20, 50));
    }

    /// ★#1344 三态参照（#93 等价锚换含失效 oracle）：逐笔把已到同向笔**完整重建**为标准第二序列，
    /// 在整列上找任意分型（不调被测函数）；同一笔内先判分型、后判严格越界（xianduan §5.1 第二问）。
    /// 返回 (终态, 决定笔)。整列重找与被测的尾三元组提前停等价：包含合并只改末元素。
    fn reference_second_terminal(
        strokes: &[Stroke],
        seg_dir: Direction,
        from_stroke_idx: usize,
        candidate_extreme: Tick,
    ) -> (SecondTerminal, Option<usize>) {
        let mut members: Vec<(Tick, Tick)> = Vec::new();
        for (i, sk) in strokes.iter().enumerate().skip(from_stroke_idx + 1) {
            if sk.direction != seg_dir {
                continue;
            }
            let (h, l) = stroke_high_low(sk);
            members.push((h, l));
            let mut elements: Vec<(Tick, Tick)> = Vec::new();
            let mut dir_state: Option<Direction> = match seg_dir {
                Direction::Up => Some(Direction::Down),
                Direction::Down => None,
            };
            for &(mh, ml) in &members {
                if elements.is_empty() {
                    elements.push((mh, ml));
                } else {
                    dir_state = apply_inclusion(&mut elements, mh, ml, dir_state);
                }
            }
            if has_any_fractal(&elements) {
                return (SecondTerminal::Confirmed, Some(i));
            }
            let breached = match seg_dir {
                Direction::Up => h > candidate_extreme,
                Direction::Down => l < candidate_extreme,
            };
            if breached {
                return (SecondTerminal::KilledByNewExtreme, Some(i));
            }
        }
        (SecondTerminal::OpenAtPrefixEnd, None)
    }

    fn assert_matches_reference(
        strokes: &[Stroke],
        seg_dir: Direction,
        apex: usize,
        p: Tick,
        label: &str,
    ) {
        let got = second_sequence(strokes, seg_dir, apex, p, 0);
        let decided_at = got
            .matched
            .as_ref()
            .map(|m| m.through_stroke)
            .or(got.killed_by.as_ref().map(|k| k.stroke_idx));
        assert_eq!(
            (got.semantic_state, decided_at),
            {
                let (state, at) = reference_second_terminal(strokes, seg_dir, apex, p);
                (Some(state), at)
            },
            "{label}"
        );
        assert_eq!(got.coverage, ScanCoverage::Complete, "{label}");
    }

    #[test]
    fn property_second_seq_three_state_equals_full_rebuild_oracle() {
        // 手工序列：单调（无分型）、顶分型、底分型、包含后分型（P=100 不越界）。
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
            assert_matches_reference(strokes, Direction::Up, 0, 100, &format!("hand case {i}"));
            // 同一序列把 P 压到 25：越界与分型谁先到由参照逐笔裁决。
            assert_matches_reference(
                strokes,
                Direction::Up,
                0,
                25,
                &format!("hand case {i} P=25"),
            );
        }
        // 交替随机游走（确定性 LCG）：每根可作 b 的笔都以其极值为 P 对拍，两个方向都覆盖。
        let mut state: u64 = 0x1420;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as i64
        };
        let mut kinds = [0usize; 3];
        for round in 0..60 {
            let len = 20 + (round % 7) * 9;
            let mut price = 1_000i64;
            let mut strokes = Vec::with_capacity(len);
            for i in 0..len {
                let step = 1 + next() % 40;
                let end = if i % 2 == 0 {
                    price + step
                } else {
                    price - step
                };
                let dir = if end > price {
                    Direction::Up
                } else {
                    Direction::Down
                };
                strokes.push(stroke(dir, i * 4, i * 4 + 4, price, end));
                price = end;
            }
            for apex in 0..len {
                let seg_dir = strokes[apex].direction.flip();
                let (h, l) = stroke_high_low(&strokes[apex]);
                let p = match seg_dir {
                    Direction::Up => h,
                    Direction::Down => l,
                };
                assert_matches_reference(
                    &strokes,
                    seg_dir,
                    apex,
                    p,
                    &format!("random {round} apex {apex}"),
                );
                let (state, _) = reference_second_terminal(&strokes, seg_dir, apex, p);
                kinds[state as usize] += 1;
            }
        }
        assert!(
            kinds.iter().all(|&k| k > 0),
            "随机语料须同时覆盖三态：{kinds:?}"
        );
    }
}
