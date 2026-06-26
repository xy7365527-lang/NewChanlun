//! 第六步：canonical 分解（reference-theta-v0.md:24，[设计选择,默认值]）。
//!
//! ## 契约重锚（legacy Strict/Decomp → Origin 行为商；minGauge 优化层待 Origin 端口，诚实声明）
//!
//! canonical 分解的**概念归属**在 Origin 行为商：`Origin.BehaviorQuotient.{BehEquiv,
//! CompleteClassifier,behaviorQuotientClassify}` + `Origin.FiniteTraceQuotient.finiteCorpusClassify`
//! ——同 base 多义分解的「规范等价类」由行为商定义（`behavior_quotient_fiber_iff`：同类 ⟺ 行为
//! 等价）。本文件的 gauge 截面选择（`minGauge` 字典序破平局）落到 parser 结构端点序列上：
//! - `min_gauge a b := if key(a) ≤ key(b) then a else b`（**平级保左**）。
//! - `gauge_fix := foldl min_gauge`（从候选列表折叠选截面）。
//! - 破平局键「最早确认时间 → 最低递归层 → 最早原始 index」（reference:24）。
//!
//! ★诚实有效域（no-workaround，待对齐依赖）：Origin canonical 当前**无** `minGauge` 字典序
//! gauge-fixing 的 native 对应物（行为商 `BehaviorQuotient` 定义了规范等价**类**的存在，但具体
//! 「字典序 tie-break 选唯一代表元」的优化层未 port 到 Origin）。故本文件的 gauge 截面选择是
//! **parser 实装侧的确定截面选择器**，其概念正确性锚 Origin 行为商（同纤维内选唯一代表元），
//! 但具体 `minGauge` 优化层的 Origin native 端口**诚实延后**——不冒充已有 Origin minGauge 锚点。
//!
//! ## reference-theta-v0.md:24 逐字
//!
//! 「从左到右扫描；候选取最早确认端点；按最早确认时间、最低递归层、最早原始 index 破平局。」
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 本文件 L0（纯结构选择，零数据依赖）。它实装给定 gauge 后的确定性截面选择（概念锚 Origin
//! 行为商同纤维代表元）——**不**声称「真实走势分解经验唯一」（后者 L2/L3）。多义性是真的
//! （行为商纤维非平凡），gauge 固定后规范截面唯一确定（本文件实装该 gauge 截面选择器）。

use super::super::types::{Segment, Timestamp};

/// canonical 候选端点（结构对象在 canonical 分解中的统一表示）。
///
/// gauge 截面选择器的破平局三键（reference:24；概念锚 Origin 行为商同纤维代表元选择）：
/// - `confirm_time`：最早**确认时间**（主键；端点被确认为分解边界的时刻）。
/// - `recursion_level`:**最低递归层**（次键；段=L0 层，上级走势层更高）。
/// - `source_index`:**最早原始 index**（末键；端点对应的原始 K 序号，对齐平局裁决 :16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// 确认时间（主键，升序优先）。
    pub confirm_time: Timestamp,
    /// 递归层（次键，最低优先；L0 段 = 0）。
    pub recursion_level: u32,
    /// 原始 K 序号（末键，最早优先；reference:16 平局裁决）。
    pub source_index: usize,
}

impl Candidate {
    /// gauge 排序键（reference:24 三键字典序：确认时间 → 递归层 → 原始 index）。
    ///
    /// 三键全部「越小越优先」（平级保左 = 三键字典序更小者胜）。返回元组用 Rust 的元组 `Ord`
    /// （按序字典比较）。
    pub fn gauge_key(&self) -> (Timestamp, u32, usize) {
        (self.confirm_time, self.recursion_level, self.source_index)
    }
}

/// gauge 截面选择：两候选取 gauge_key 较小者（reference:24 破平局，平级保左）。
///
/// `min_gauge a b := if key(a) ≤ key(b) then a else b`。`≤` 保证平局（key 相等）时保留 `a`
/// （更早扫描到的，左侧）——「平级最左」（gauge 截面选择器的确定代表元规则）。
fn min_gauge(a: Candidate, b: Candidate) -> Candidate {
    if a.gauge_key() <= b.gauge_key() {
        a
    } else {
        b
    }
}

/// 从候选端点列表折叠选 gauge 截面（reference:24，gauge 截面选择器）。
///
/// `gauge_fix := foldl min_gauge`。空列表无截面（`None`，偏函数诚实——不伪造全函数）；
/// 非空折叠选 gauge_key 最小候选（平级保左）。
///
/// 边界条件：候选为空 ⟹ `None`（无可选截面）。
pub fn gauge_fix(cands: &[Candidate]) -> Option<Candidate> {
    cands.split_first().map(|(&head, rest)| rest.iter().fold(head, |acc, &c| min_gauge(acc, c)))
}

/// canonical 分解（reference-theta-v0.md:24）——线段序列的 canonical 端点扫描。
///
/// 从左到右扫描已确认线段端点序列，按 gauge 截面选择产出 canonical 端点序列。每个线段
/// 贡献一个端点候选（其确认所在的原始 index + 确认时间 + L0 递归层）。
///
/// ★诚实范围（formalization-validity-domain + no-patch-mentality）：
/// L0 段全部同层（recursion_level=0），且 parser 的段端点已按确认时间从左到右产生且**互不
/// 平局**（每段端点的 source_index 严格递增）——故本层的 canonical 分解 = 线段端点的
/// 有序序列本身（gauge 截面在无平局时退化为恒等扫描）。`gauge_fix` 的破平局逻辑在**跨递归
/// 层**的 canonical 分解（上级走势端点与段端点平局时）才产生非平凡截面——那是 classifier
/// 递归级别的职责（上级走势端点带更高 recursion_level）。本文件提供 L0 段层的 canonical
/// 端点序列 + gauge 截面原语，供 classifier 跨层组装时复用（bit-exact 对齐 Decomp.gaugeFix）。
///
/// 边界条件：
/// - 线段为空 ⟹ 空 canonical 序列。
/// - L0 单层段端点无平局 ⟹ canonical 端点序列 = 段端点确认序（恒等扫描）。
pub fn canonical_endpoints(segments: &[Segment]) -> Vec<Candidate> {
    segments
        .iter()
        .map(|seg| {
            // 段端点确认所在的原始 K 序号（段末 index = 段被确认为边界的端点）。
            let end_idx = seg.end_index;
            Candidate {
                // 确认时间：段端点的原始 K 序号即其时间序代理（reference:16 时间序单调，
                // L0 段端点 source_index 与 timestamp 同序——parser 在整数 tick 域，
                // 端点确认时间 = 端点原始 K 序号）。
                confirm_time: end_idx as Timestamp,
                // L0 段层 = 递归层 0（最低层，Decomp.gaugeLevel 段层）。
                recursion_level: 0,
                source_index: end_idx,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Direction;

    fn cand(t: Timestamp, lvl: u32, idx: usize) -> Candidate {
        Candidate {
            confirm_time: t,
            recursion_level: lvl,
            source_index: idx,
        }
    }
    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    #[test]
    fn gauge_key_orders_confirm_time_first() {
        // 主键确认时间：t=1 优先于 t=2（无视次/末键）。
        let a = cand(1, 9, 99);
        let b = cand(2, 0, 0);
        assert_eq!(min_gauge(a, b), a);
    }

    #[test]
    fn gauge_key_breaks_tie_by_recursion_level() {
        // 同确认时间：低递归层优先（次键，对齐 Decomp.gaugeLevel 最小）。
        let a = cand(5, 0, 50);
        let b = cand(5, 1, 10);
        assert_eq!(min_gauge(a, b), a); // level 0 < 1，胜
    }

    #[test]
    fn gauge_key_breaks_tie_by_source_index() {
        // 同确认时间 + 同递归层：最早原始 index 优先（末键，平局裁决 :16）。
        let a = cand(5, 0, 3);
        let b = cand(5, 0, 7);
        assert_eq!(min_gauge(a, b), a); // idx 3 < 7，胜
    }

    #[test]
    fn min_gauge_keeps_left_on_full_tie() {
        // 三键全相等（平局）→ 保留左侧 a（平级最左，min_gauge 的 ≤ 规则）。
        let a = cand(5, 0, 3);
        let b = cand(5, 0, 3);
        assert_eq!(min_gauge(a, b), a);
    }

    #[test]
    fn gauge_fix_empty_none() {
        // 空候选 → None（偏函数诚实，无可选截面）。
        assert_eq!(gauge_fix(&[]), None);
    }

    #[test]
    fn gauge_fix_selects_minimal_candidate() {
        // 折叠选 gauge_key 最小（确认时间最早）候选。
        let cands = vec![cand(3, 0, 30), cand(1, 0, 10), cand(2, 0, 20)];
        assert_eq!(gauge_fix(&cands), Some(cand(1, 0, 10)));
    }

    #[test]
    fn canonical_endpoints_l0_identity_scan() {
        // L0 单层段端点无平局 → canonical 端点序列 = 段端点确认序（恒等扫描）。
        let segs = vec![
            seg(Direction::Up, 0, 4, 0, 10),
            seg(Direction::Down, 4, 8, 10, 5),
        ];
        let cands = canonical_endpoints(&segs);
        assert_eq!(cands.len(), 2);
        assert_eq!(cands[0].source_index, 4); // 段0 末 index
        assert_eq!(cands[1].source_index, 8); // 段1 末 index
        assert_eq!(cands[0].recursion_level, 0); // L0 层
    }

    #[test]
    fn canonical_endpoints_empty_segments() {
        assert!(canonical_endpoints(&[]).is_empty());
    }

    /// property：canonical 端点的 source_index 与段末 index 一致（端点 = 段确认边界）。
    #[test]
    fn property_canonical_endpoint_matches_segment_end() {
        let segs = vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 9, 10, 3)];
        let cands = canonical_endpoints(&segs);
        for (cand, seg) in cands.iter().zip(segs.iter()) {
            assert_eq!(cand.source_index, seg.end_index);
        }
    }
}
