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
///
/// 所有字段 Rc 共享——`ParseLayerIncr::append` 从增量子模块 `to_result_rc()` O(1) clone，
/// `parse_layer`（批量）从 `Rc::new(...)` O(n) 构造（单次，非每 bar）。
/// 下游 `&layer.fractals` 等借用经 `Rc::deref → Vec::deref → &[T]`（透明）。
/// ponytail: Rc 共享消除每 bar Vec clone（#93 incr_total exp 1.89 残余 O(n²)）。
#[derive(Debug, Clone, Default)]
pub struct ParseLayer {
    /// Rc 共享——`ParseLayerIncr::append` 从 `IncrInclusion.merged_rc()` O(1) clone，
    /// `parse_layer`（批量）从 `Rc::new(merged.to_vec())` O(n) 构造（单次，非每 bar）。
    /// 下游 `&layer.merged_bars` 借用经 `Rc::deref → Vec::deref → &[Bar]`（透明）。
    pub merged_bars: Rc<Vec<Bar>>,
    /// merged_bars 跨 bar 稳定的 confirmed 前缀长度（O(n²)→O(n) 证书，#106）。
    /// 相 B 稳态 = `merged_bars.len()-1`（仅末根 acc 可被下个 bar 改写）；相 A / 相 A→B
    /// 迁移本 bar = `0`（保守，下游退化全量重建，bit-exact）。全量 `parse_layer` = `0`
    /// （无增量血缘，退化）。classifier `update_closes_cache`/MACD 用它替代每 bar O(n) 前缀
    /// 全量比较——下游 fractal/stroke/segment 已信任此 confirmed 边界（增量重算保留前缀），
    /// classifier 之前是唯一每 bar 重新 O(n) 验证的冗余防御。
    pub merged_confirmed_len: usize,
    /// #106：segments 跨 bar 稳定的 confirmed 前缀长度（= IncrSegments.append 的 keep）。
    /// classifier l0_tower 复用证书。全量 `parse_layer` = 0（无血缘，退化全量重建）。PartialEq 排除
    /// （性能证书非结构语义）。**不可用 segments.len()-1**（codex：末段可古怪线段重划改写）。
    pub segments_confirmed_len: usize,
    /// #88：增量 segment 的 unsealed 起点（性能诊断——frontier 回退深度/前移轨迹实测）。
    /// 全量 `parse_layer` = None。PartialEq 排除（诊断字段，非结构语义）。
    pub segments_earliest_unsealed: Option<usize>,
    /// Rc 共享——`ParseLayerIncr::append` 从 `IncrFractals::to_result_rc()` O(1) clone。
    pub fractals: Rc<Vec<Fractal>>,
    /// Rc 共享——`ParseLayerIncr::append` 从 `IncrStrokes::to_result_rc()` O(1) clone。
    pub strokes: Rc<Vec<Stroke>>,
    /// Rc 共享——`ParseLayerIncr::append` 从 `IncrSegments::to_result_rc()` O(1) clone。
    pub segments: Rc<Vec<Segment>>,
    /// Rc 共享——tail 全量重算结果 O(1) 共享（tail 小 Vec，重算本身 O(tail) 非 O(n)）。
    pub tail: Rc<Vec<PendingTail>>,
}

// #106：`merged_confirmed_len` 是性能证书（跨 bar 稳定前缀长度），非结构语义——增量版填真值、
// 全量 `parse_layer` 填 0，两者结构内容（merged_bars/fractals/strokes/segments/tail）仍 bit-identical。
// 手写 PartialEq 排除证书字段，使「增量==全量」bit-exact 对拍不被证书差异误判。
impl PartialEq for ParseLayer {
    fn eq(&self, other: &Self) -> bool {
        self.merged_bars == other.merged_bars
            && self.fractals == other.fractals
            && self.strokes == other.strokes
            && self.segments == other.segments
            && self.tail == other.tail
    }
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
    /// ponytail: Rc 共享消除每 bar Vec clone——`to_result_rc()` 返回 Rc clone O(1)，
    /// 替代旧 `to_result().to_vec()` 的 O(n)/bar。tail 二分查找消除 O(merged_i) 线性扫描。
    pub fn append(&mut self, bar: Bar) -> ParseLayer {
        // append 消费 self（by-value，mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        // #106 证书：append 前的相位决定本 bar confirmed 前缀语义（O(1)）。
        let was_open_tail = self.incr_inclusion.to_result_ref().only_open_tail;
        let prev = std::mem::replace(&mut self.incr_inclusion, inclusion::IncrInclusion::empty());
        self.incr_inclusion = prev.append(bar);
        // merged_rc: Rc 共享 clone（O(1)），替代旧 to_result_ref().merged.to_vec() 的 O(n)/bar。
        // 下游 fractal/stroke/segment/tail 借用 &merged（Rc::deref → &[Bar]，透明）。
        let merged = self.incr_inclusion.merged_rc();
        // #106 confirmed 前缀长度（O(1) 证书）：相 B 稳态（append 前后均非 open_tail）⟹ 仅末根
        // acc 可改写，前缀 [..len-1] 物理不变（append_folded 用 Rc::make_mut pop/push 末根，
        // 前缀字节不动）⟹ confirmed = len-1。相 A（仍 open_tail）/ 相 A→B 迁移本 bar（was_open_tail
        // 但现非）⟹ 0（整段折叠或无方向，对 classifier 旧 closes cache 无可复用前缀，退化全量）。
        let now_open_tail = self.incr_inclusion.to_result_ref().only_open_tail;
        let merged_confirmed_len = if was_open_tail || now_open_tail {
            0
        } else {
            merged.len().saturating_sub(1)
        };

        // 增量 fractal：保留 confirmed 前缀，重算尾部 2 个三元组。
        // by-value append（mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        let prev_fractals = std::mem::replace(&mut self.incr_fractals, fractal::IncrFractals::empty());
        self.incr_fractals = prev_fractals.append(&merged);
        let fractals = self.incr_fractals.to_result_rc();

        // 增量 stroke：保留 confirmed 交替序列前缀 + confirmed strokes，续扫配对。
        // by-value append（mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        let prev_strokes = std::mem::replace(&mut self.incr_strokes, stroke::IncrStrokes::empty());
        self.incr_strokes = prev_strokes.append(&fractals, &self.config.parse);
        let strokes = self.incr_strokes.to_result_rc();

        // 增量 segment：保留 confirmed segments 前缀，从 pending_start 续扫。
        // by-value append（mem::take 重用 Vec 缓冲，消除 O(n)/bar clone）。
        let prev_segments = std::mem::replace(&mut self.incr_segments, segment::IncrSegments::empty());
        self.incr_segments = prev_segments.append(&strokes, &self.config.parse);
        let (segments, pending_start) = self.incr_segments.to_result_rc();
        let pending_start = pending_start;
        // #106 segments 证书（O(1)）：l0_tower 复用边界。
        let segments_confirmed_len = self.incr_segments.confirmed_len();
        let segments_earliest_unsealed = self.incr_segments.earliest_unsealed_from();

        // tail 全量重算（O(tail) 非 O(merged_i)——二分查找定位锚点 + 尾部延伸段扫描）。
        let tail = tail::build_tail(&merged, &fractals, &strokes, &segments, pending_start);

        ParseLayer {
            merged_bars: merged,
            merged_confirmed_len,
            segments_confirmed_len,
            segments_earliest_unsealed,
            fractals,
            strokes,
            segments,
            tail: Rc::new(tail),
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
///
/// ★契约锚（#239 实写，#236 裁定）：`Origin/Pipeline.lean` `originPipeline`。
/// ⚠段结构差（勿误对拍）：本七段复合**吃 raw bars**（步骤1包含合并起，全链从原始 K 线
/// 构造）；Lean `originPipeline` 是 3+2 段**吃 strokes/candidates**（构造链 segmentsOf→
/// centersOf→bspOf + 闭环链 chanlunTransitionFold，`OriginInput` 已携笔流与候选端点流）——
/// 非同一函数、输入域不同，禁直接对拍。重叠区 = strokes/segments（交接对账项在案，#240 终裁）。
fn parse_layer_from_merged(merged: &[Bar], config: &ThetaConfig) -> ParseLayer {
    let fractals = fractal::detect_fractals(merged);
    let strokes = stroke::build_strokes(&fractals, &config.parse);
    let (segments, pending_start) = segment::divide_segments_with_tail(&strokes, &config.parse);
    let tail = tail::build_tail(merged, &fractals, &strokes, &segments, pending_start);
    ParseLayer {
        merged_bars: Rc::new(merged.to_vec()),
        merged_confirmed_len: 0, // 全量构造无增量血缘 ⟹ 0 = classifier 退化全量重建（bit-exact）。
        segments_confirmed_len: 0,
        segments_earliest_unsealed: None,
        fractals: Rc::new(fractals),
        strokes: Rc::new(strokes),
        segments: Rc::new(segments),
        tail: Rc::new(tail),
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
        let ds = load_symbol(&path, "OKLO", &cfg, 60).expect("加载 OKLO");
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
