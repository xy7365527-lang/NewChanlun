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
use std::rc::Rc;

/// Θ_parse 流水线分步实现（聚焦小文件，coding-style <400 行/文件）。
pub mod inclusion;
pub mod fractal;
pub mod stroke;
pub mod segment;
pub mod feature_seq;
pub mod second_kind;
pub mod canonical;
pub mod tail;

/// 性能 profile（cfg(test) only，#93 标度测量，不入产物）。
#[cfg(test)]
mod profile;

/// 单层解析输出（一个级别的完整 confirmed 结构 + 未完成尾部）。
///
/// 契约锚 `Origin.ChanlunElements.ParseStruct` 的子集（mergedBars/fractals/strokes/segments/tail——
/// centers/moves/bsp 由 classifier 从 segments 构造）：confirmed 部分对齐 `ElementPipeline.parse`
/// 的批量唯一解析（`parse_total_unique`）；`tail` 显式保存未完成尾部（对齐 `tailOf : ... -> OpenTail`），
/// 不混入 confirmed。中枢（`centersOf`）移交 classifier，不在本结构（见模块头）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParseLayer {
    /// Rc 共享——`ParseLayerIncr::append` 从 `IncrInclusion.merged_rc()` O(1) clone，
    /// `parse_layer`（批量）从 `Rc::new(merged.to_vec())` O(n) 构造（单次，非每 bar）。
    /// 下游 `&layer.merged_bars` 借用经 `Rc::deref → Vec::deref → &[Bar]`（透明）。
    pub merged_bars: Rc<Vec<Bar>>,
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
    // 步骤 2-7：从 merged 构造（与增量入口共享，下游步结构性 bit-exact）。
    parse_layer_from_merged(&incl.merged, config)
}

// ============================================================================
// 增量 parse_layer API（#93 per-bar substrate O(n²) 根因解——parser 侧入口）。
//
// ## 缺口锚点（aed4d5f5 + incremental.rs:10/23 + runner.rs:280）
//
// per-bar substrate 每 bar 调 `parse_layer(&bars[..=i])`。inclusion 全量左折叠 O(i)/bar 是
// 精确实证的 O(n²) 根因。本入口把 inclusion 增量化（`IncrInclusion` append-1-bar O(1)），
// 下游（fractal/stroke/segment/tail）基于增量 inclusion 的 merged 输出重算。
//
// ## 诚实标注（formalization-validity-domain：有效域边界）
//
// 本入口的增量收益**仅覆盖 inclusion 步**（O(i)→O(1)/bar）。下游 fractal/stroke/segment
// 仍是 O(merged_i)/bar 的全量重算——**若 merged_i 随 i 线性增长，下游仍是 O(n²) 项**。
// 即：本入口把 inclusion 这一项从 O(n²) 降到 O(n)，但**不改变下游步的标度**。
//
// 下游增量化（fractal 局部三元组可增量；stroke 的 collapse_consecutive 全局规整与 segment
// 的 FeatureSeqState 状态机需独立工位）超出 inclusion 缺口范围——实测下游是否成新 O(n²)
// 热点见 profile.rs `profile_parse_layer_incr_scaling`。本工位的严格范围 = inclusion 增量化。
//
// ## bit-exact 不变量（铁律）
//
// 对任意 bar 序列与任意 i：`parse_layer_append` 链的输出 == `parse_layer(&bars[..=i])`，
// 逐字段精确（ParseLayer #[derive(PartialEq)]）。逐 bar 断言见 profile.rs
// `bit_exact_parse_layer_incr_per_bar`（合成 + 真实数据）。
// ============================================================================

/// 增量 parse_layer 状态（封装增量 inclusion + 增量 fractal/stroke/segment + tail 重算）。
///
/// 每 bar `append` 推进：
/// 1. 增量 inclusion（O(1) 稳态）→ merged 输出。
/// 2. 增量 fractal（O(尾部)/bar，保留 confirmed 前缀）。
/// 3. 增量 stroke（O(尾部)/bar，保留 confirmed 前缀）。
/// 4. 增量 segment（O(pending)/bar，保留 confirmed 前缀）。
/// 5. tail 全量重算（O(merged_i)，tail 依赖全字段——非热点，profile 坐实）。
#[derive(Debug, Clone)]
pub struct ParseLayerIncr<'c> {
    incr_inclusion: inclusion::IncrInclusion,
    incr_fractals: fractal::IncrFractals,
    incr_strokes: stroke::IncrStrokes,
    incr_segments: segment::IncrSegments,
    config: &'c ThetaConfig,
}

impl<'c> ParseLayerIncr<'c> {
    /// 新建增量解析器（零 bar 起步）。
    pub fn new(config: &'c ThetaConfig) -> Self {
        ParseLayerIncr {
            incr_inclusion: inclusion::IncrInclusion::empty(),
            incr_fractals: fractal::IncrFractals::empty(),
            incr_strokes: stroke::IncrStrokes::empty(),
            incr_segments: segment::IncrSegments::empty(),
            config,
        }
    }

    /// 追加 1 bar，返回该 bar 后的 `ParseLayer`（bit-exact 对齐 `parse_layer(&bars[..=i])`）。
    ///
    /// 内部：增量 inclusion → 增量 fractal → 增量 stroke → 增量 segment → tail 重算。
    /// ponytail: 增量化把下游从 O(merged_i)/bar 降到 O(尾部)/bar（fractal/stroke/segment
    /// 均保留 confirmed 前缀，只重算尾部）。tail 仍全量但非热点（profile 坐实）。
    pub fn append(&mut self, bar: Bar) -> ParseLayer {
        // append 消费 self（by-value，mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        let prev = std::mem::replace(&mut self.incr_inclusion, inclusion::IncrInclusion::empty());
        self.incr_inclusion = prev.append(bar);
        // merged_rc: Rc 共享 clone（O(1)），替代旧 to_result_ref().merged.to_vec() 的 O(n)/bar。
        // 下游 fractal/stroke/segment/tail 借用 &merged（Rc::deref → &[Bar]，透明）。
        let merged = self.incr_inclusion.merged_rc();

        // 增量 fractal：保留 confirmed 前缀，重算尾部 2 个三元组。
        self.incr_fractals = self.incr_fractals.append(&merged);
        let fractals = self.incr_fractals.to_result();

        // 增量 stroke：保留 confirmed 交替序列前缀 + confirmed strokes，续扫配对。
        // by-value append（mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        let prev_strokes = std::mem::replace(&mut self.incr_strokes, stroke::IncrStrokes::empty());
        self.incr_strokes = prev_strokes.append(&fractals, &self.config.parse);
        let strokes = self.incr_strokes.to_result().to_vec();

        // 增量 segment：保留 confirmed segments 前缀，从 pending_start 续扫。
        self.incr_segments = self.incr_segments.append(&strokes, &self.config.parse);
        let (segments, pending_start) = self.incr_segments.to_result_vec();
        let pending_start = pending_start;

        // tail 全量重算（依赖全字段，非热点——profile 坐实 tail 耗时占比 <5%）。
        let tail = tail::build_tail(&merged, &fractals, &strokes, &segments, pending_start);

        ParseLayer {
            merged_bars: merged,
            fractals,
            strokes,
            segments,
            tail,
        }
    }
}

/// 从已合并的 merged bar 序列构造 ParseLayer（步骤 2-7，复用全量子步）。
///
/// 增量与全量共享此函数——保证下游步 bit-exact（同一代码路径）。
///
/// 步骤 2-7 契约锚（详见各子模块文档）：
/// - 步骤 2 分型（fractal::detect_fractals，reference:20，[缠论可导,62课]）。
/// - 步骤 3 新笔（stroke::build_strokes，reference:21，[缠论可导,77/81课]）。
/// - 步骤 4 线段 v1 特征序列法（segment::divide_segments_with_tail，reference:22，第67/71课）。
///   增量「假设转折点」状态机（feature_seq.rs），对参考语义 a_segment_v1 认证（L1）。
///   `(segments, pending_start)` 同时喂给步骤 7 tail——避免重跑段划分（性能 #93）。
/// - 步骤 6 canonical 分解：L0 段层与段端点序列恒等（见 mod 头注释），不额外存。
/// - 步骤 7 未完成尾部（tail::build_tail，reference:25，对齐 Parse.lean §6 + OpenTail）。
fn parse_layer_from_merged(merged: &[Bar], config: &ThetaConfig) -> ParseLayer {
    let fractals = fractal::detect_fractals(merged);
    let strokes = stroke::build_strokes(&fractals, &config.parse);
    let (segments, pending_start) = segment::divide_segments_with_tail(&strokes, &config.parse);
    let tail = tail::build_tail(merged, &fractals, &strokes, &segments, pending_start);
    ParseLayer {
        merged_bars: Rc::new(merged.to_vec()),
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
