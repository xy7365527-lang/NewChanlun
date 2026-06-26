//! Θ_parse 子模块（reference-theta-v0.md:18-25）——构造层契约重锚 Origin（task #127 A′ Phase2）。
//!
//! ## 契约重锚（legacy Strict/Parse.lean → Origin.ChanlunElements canonical）
//!
//! 契约锚点从 legacy `Strict/Parse.lean` **重锚到 `Origin.ChanlunElements.ElementPipeline`**
//! （ChanlunElements.lean，`namespace NewChanlun.Origin`）—— Origin 构造层把缠论解析定义为
//! **七字段流水线 record** `ElementPipeline`：
//! ```text
//! structure ElementPipeline where
//!   mergeBars  : List Bar -> List Bar
//!   fractalsOf : List Bar -> List Fractal
//!   strokesOf  : List Fractal -> List Stroke
//!   segmentsOf : List Stroke -> List Segment       -- ↔ Origin.SegmentConstruction.segmentsOf
//!   centersOf  : List Segment -> List Center       -- ↔ Origin.CenterConstruction.centersOf
//!   movesOf    : List Segment -> List Center -> List Move
//!   bspOf      : List Move -> List Bsp              -- ↔ Origin.BspConstruction.bspOf
//!   tailOf     : ... -> OpenTail
//! def ElementPipeline.parse (P) (bars) : ParseStruct := ...（七段顺序复合）
//! ```
//! 给定 `ElementPipeline` ⟹ `parse` 全函数唯一（`parse_total_unique`）。本 Rust [`parse_layer`]
//! 是 `ElementPipeline.parse` 的 bit-exact 实装（**批量**吃整个 bar 序列，与 Origin 同——批量唯一性
//! 而非逐 bar 增量）。各阶段子模块逐一对齐 Origin 构造层 def（见下子任务清单的契约锚标注）。
//!
//! ## 子任务清单（各阶段契约锚 Origin 构造层 def）
//!
//! 1. **K线包含合并**（:19，↔ `Origin.ElementPipeline.mergeBars`）：相邻区间包含即合并；向上
//!    `high=max,low=max`，向下 `low=min,high=min`；方向按前一对非包含 K 严格高低变化决定；
//!    开头无方向向前看第一个非包含对；全程无方向只输出 open-tail。[缠论可导,62/65课]
//! 2. **分型识别**（:20，↔ `Origin.ElementPipeline.fractalsOf`）：包含处理后，顶=中 K 高低**严格**
//!    高于左右；底反之；等价不成立；第三根 K 收盘确认。[缠论可导,62课]
//! 3. **新笔**（:21，↔ `Origin.ElementPipeline.strokesOf`）：旧笔禁用；顶/底分型不共用 K；两极值
//!    K 间排除两端 ≥3 根（config）；同类连续分型，顶保留更高/底保留更低/等价保留更早。[缠论可导,77/81课]
//! 4. **线段 67 课特征序列法**（:22，↔ `Origin.SegmentConstruction.segmentsOf` +
//!    `Origin.SegmentFeatureComplete.SegEndComplete`）：向上线段看反向笔特征序列顶分型，向下反之；
//!    `segmentsOf` 用 `nextSegmentEnd`/`scanSegEnd` 滑窗（成立支消费 ≥1 笔，well-founded 终止）；
//!    完整段端确认谓词 `SegEndComplete`（分型 ∧ gap-case ∧ TopAboveBottom 第78课）。[缠论可导,67/78课]
//! 5. **中枢边界**（:23，↔ `Origin.CenterConstruction.centersOf` + `Origin.CenterComplete`）：前两段定
//!    核心 `ZD=computeZD s1 s2, ZG=computeZG s1 s2`；完整判据方向交替 + 第三段贯穿（见 classifier/center.rs）。
//! 6. **canonical 分解**（:24，↔ `Origin.ChanlunElements` 解析端点序列）：从左到右扫描；候选取最早确认
//!    端点；按最早确认时间、最低递归层、最早原始 index 破平局。[设计选择,默认值]
//! 7. **未完成尾部**（:25，↔ `Origin.ElementPipeline.tailOf : ... -> OpenTail`）：显式保存
//!    `Pending*`/`AliveCenter`；不输出为 confirmed。
//!
//! ## 步骤5（中枢）移交 classifier（Lead 裁定）
//!
//! 中枢是递归构造（`Origin.CenterConstruction.centersOf : List Segment → List Center`），
//! owner=classifier——classifier 从 parser 的 `segments` 自己构造中枢（见 `classifier/mod.rs`
//! `detect_centers_complete`，完整判据），**不**读 parser 的中枢。故 `ParseLayer` **无** `centers`
//! 字段（中枢构造是 `centersOf` 的职责，不在 parser 单层流水线 `segmentsOf` 之内）。
//!
//! ## 接口契约（已冻结，下游 classifier/backtest 依赖）

use super::config::ThetaConfig;
use super::types::{Bar, Fractal, PendingTail, Segment, Stroke};

/// Θ_parse 流水线分步实现（聚焦小文件，coding-style <400 行/文件）。
pub mod inclusion;
pub mod fractal;
pub mod stroke;
pub mod segment;
pub mod feature_seq;
pub mod second_kind;
pub mod canonical;
pub mod tail;

/// 单层解析输出（一个级别的完整 confirmed 结构 + 未完成尾部）。
///
/// 契约锚 `Origin.ChanlunElements.ParseStruct` 的子集（mergedBars/fractals/strokes/segments/tail——
/// centers/moves/bsp 由 classifier 从 segments 构造）：confirmed 部分对齐 `ElementPipeline.parse`
/// 的批量唯一解析（`parse_total_unique`）；`tail` 显式保存未完成尾部（对齐 `tailOf : ... -> OpenTail`），
/// 不混入 confirmed。中枢（`centersOf`）移交 classifier，不在本结构（见模块头）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParseLayer {
    pub merged_bars: Vec<Bar>,
    pub fractals: Vec<Fractal>,
    pub strokes: Vec<Stroke>,
    pub segments: Vec<Segment>,
    pub tail: Vec<PendingTail>,
}

/// Θ_parse 顶层入口（L0=1分钟线段账本的解析）。
///
/// 七步流水线（步骤5 中枢移交 classifier，见模块头），bit-exact 实装 `Origin.ChanlunElements.
/// ElementPipeline.parse`：1. K线包含合并(`mergeBars`) → 2. 分型(`fractalsOf`) → 3. 新笔(`strokesOf`)
/// → 4. 线段(`segmentsOf`，67课) → 6. canonical 分解 → 7. 未完成尾部 tail(`tailOf`)。签名已冻结
/// ——`(bars, config) -> ParseLayer`。
///
/// confirmed/active 切分：`merged_bars/fractals/strokes/segments` 是已确认结构（confirmed，交易
/// 可用，对齐 `ElementPipeline.parse` 的 ParseStruct 主体）；`tail` 是未完成尾部（active，仅预警，
/// 不输出为 confirmed，对齐 `tailOf : ... -> OpenTail`）。
///
/// 边界条件：`bars` 为空 / 全程无方向 ⟹ confirmed 全空，只有 open-tail
/// （reference-theta-v0.md:19）；尾部延伸结构进 `tail`。
pub fn parse_layer(bars: &[Bar], config: &ThetaConfig) -> ParseLayer {
    // 步骤 1：K线包含合并（reference-theta-v0.md:19）。
    let incl = inclusion::process_inclusion(bars);
    // 步骤 2：分型识别（reference-theta-v0.md:20）。
    let fractals = fractal::detect_fractals(&incl.merged);
    // 步骤 3：新笔划分（reference-theta-v0.md:21）。
    let strokes = stroke::build_strokes(&fractals, &config.parse);
    // 步骤 4：线段划分 v1 特征序列法（reference-theta-v0.md:22，第67/71课）。
    // 增量「假设转折点」状态机（feature_seq.rs），对参考语义 a_segment_v1 认证（L1）。
    let segments = segment::divide_segments(&strokes, &config.parse);

    // 步骤 6：canonical 分解（reference-theta-v0.md:24，bit-exact 对齐 Decomp.lean gaugeFix）。
    // ★L0 段层的 canonical 端点序列 = `canonical::canonical_endpoints(&segments)`——但它在 L0
    // 单层与段端点序列**恒等**（段端点 source_index 严格递增、无平局，gauge 截面退化为恒等
    // 扫描，见 canonical.rs 诚实标注）。故 `ParseLayer` **不**额外存 canonical 端点（由
    // `segments` 唯一确定，存它是冗余）。canonical 模块的非平凡价值在**跨递归层**的 gauge
    // 截面选择（上级走势端点与段端点平局时），由 classifier 调 `canonical::gauge_fix` 复用。
    // 这里不调用丢弃结果（避免死代码）——canonical 模块经其 pub 原语 + 测试独立证明正确性。

    // 步骤 7：未完成尾部 tail（reference-theta-v0.md:25，bit-exact 对齐 Parse.lean §6 active +
    // OpenTail.lean 当下状态）。把流水线各阶段（段/笔/分型）未确认的延伸结构显式保存，
    // 不混入 confirmed。SecondKind 动态确认段/无分型段的剩余笔现经此进 tail（不再静默丢失）。
    let tail = tail::build_tail(&incl.merged, &fractals, &strokes, &config.parse);

    ParseLayer {
        merged_bars: incl.merged,
        fractals,
        strokes,
        segments,
        tail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 临时交叉验证导出（#84 验证门点3）：导出 Rust OKLO 笔序列 + 线段到 JSON，
    /// 供 Python a_segment_v1.py 用**同一笔序列**跑 segments_from_strokes_v1 对比（隔离笔层）。
    /// cfg(test) 下 backtest 可见。验证后删除。
    #[test]
    #[ignore]
    fn tmp_export_oklo_strokes_segments_for_xcheck() {
        use crate::theta_v0::backtest::data::{data_dir, load_symbol};
        use std::io::Write;
        let cfg = ThetaConfig::default();
        let path = data_dir().join("oklo_1m_databento.json");
        let ds = load_symbol(&path, "OKLO", &cfg).expect("加载 OKLO");
        let layer = parse_layer(&ds.bars, &cfg);
        // 导出前 2000 笔（足够覆盖 FirstKind+SecondKind 混合，避免 JSON 过大）。
        let n = layer.strokes.len().min(2000);
        let strokes: Vec<_> = layer.strokes[..n]
            .iter()
            .map(|s| {
                let (lo, hi) = if s.start_price <= s.end_price {
                    (s.start_price, s.end_price)
                } else {
                    (s.end_price, s.start_price)
                };
                let dir = match s.direction {
                    super::super::types::Direction::Up => "up",
                    super::super::types::Direction::Down => "down",
                };
                format!(
                    "{{\"i0\":{},\"i1\":{},\"direction\":\"{}\",\"high\":{},\"low\":{},\"p0\":{},\"p1\":{}}}",
                    s.start_index, s.end_index, dir, hi, lo, s.start_price, s.end_price
                )
            })
            .collect();
        // Rust 在这 n 笔上的线段（端点 start_index/end_index）。
        let rust_segs = segment::divide_segments(&layer.strokes[..n], &cfg.parse);
        let segs: Vec<_> = rust_segs
            .iter()
            .map(|s| format!("[{},{}]", s.start_index, s.end_index))
            .collect();
        let out = format!(
            "{{\"strokes\":[{}],\"rust_segments\":[{}]}}",
            strokes.join(","),
            segs.join(",")
        );
        let out_path = data_dir().join("_xcheck_oklo_strokes.json");
        let mut f = std::fs::File::create(&out_path).expect("创建导出文件");
        f.write_all(out.as_bytes()).expect("写导出");
        eprintln!(
            "导出 {} 笔 + {} Rust 段 → {:?}",
            n,
            rust_segs.len(),
            out_path
        );
    }

    /// 骨架契约测试：空输入 → 空 ParseLayer（接口存在性 + 空边界）。
    /// 实装后此测试扩展为 golden/property 对齐 Parse.lean fixture。
    #[test]
    fn parse_empty_bars_yields_empty_layer() {
        let cfg = ThetaConfig::default();
        let out = parse_layer(&[], &cfg);
        assert_eq!(out, ParseLayer::default());
    }
}
