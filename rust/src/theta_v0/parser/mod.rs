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
//!    初始包含尚无方向证据时保持未定，不借未来方向回折。见 #1373 已签包含事实合同。
//! 2. **分型识别**（:20，↔ `Origin.ElementPipeline.fractalsOf`）：包含处理后，顶=中 K 高低**严格**
//!    高于左右；底反之；等价不成立；第三根 K 收盘确认。[缠论可导,62课]
//! 3. **新笔**（:21，↔ `Origin.ElementPipeline.strokesOf`）：旧笔禁用；顶/底分型不共用 K；两极值
//!    K 间排除两端 ≥3 根（config）；同类连续分型顶保留更高、底保留更低，同价保持身份未定（#1392/#1405）。
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

pub mod canonical;
pub mod feature_seq;
pub mod fractal;
/// Θ_parse 流水线分步实现（聚焦小文件，coding-style <400 行/文件）。
pub mod inclusion;
pub mod second_kind;
pub mod segment;
pub mod stroke;
pub mod tail;

/// 性能 profile（cfg(test) only，#93 标度测量，不入产物）。
#[cfg(test)]
mod profile;

/// #246 相切=重合口径的 Lean fixture 读取点（cfg(test) only，#312 主缝，不入产物）。
/// `feature_seq::tests` / `segment::tests` 共用——两个私有谓词的期望值由此接机器耦合链。
#[cfg(test)]
mod gap_overlap_fixture;

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
    parse_layer_with_inclusion_facts(bars, config).0
}

/// #1373/#1392：完整 raw 经同一包含 fold_step 和新笔核，返回同次生产结果与来源事实。
pub fn parse_layer_with_inclusion_facts(
    bars: &[Bar],
    config: &ThetaConfig,
) -> (ParseLayer, inclusion::InclusionFacts) {
    let mut facts = inclusion::process_inclusion_with_facts(bars);
    let layer = parse_layer_from_facts(&mut facts, &config.parse);
    (layer, facts)
}

// #1392：包含状态、新笔扫描 checkpoint 和分型尾部缓存共同推进。
// 新笔核给出真实稳定前缀后才复用段状态；深回退用同一段判据重建。
/// 逐 raw 输入的解析适配器；与同一原始前缀的全量入口逐字段对齐。
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

    /// 追加一个原始 bar；不沿用旧笔的冻结前缀证书。
    pub fn append(&mut self, bar: Bar) -> ParseLayer {
        append_incr_layer(
            &mut self.incr_inclusion,
            &mut self.incr_fractals,
            &mut self.incr_strokes,
            &mut self.incr_segments,
            bar,
            &self.config.parse,
        )
    }
}

/// #1392：借用与 owned 调用者共用同一 raw 前缀入口。
/// 旧缓存参数暂留调用合同；本叶明确不发出它们的复用许可。
pub(crate) fn append_incr_layer(
    incr_inclusion: &mut inclusion::IncrInclusion,
    incr_fractals: &mut fractal::IncrFractals,
    incr_strokes: &mut stroke::IncrStrokes,
    incr_segments: &mut segment::IncrSegments,
    bar: Bar,
    parse_config: &super::config::ParseConfig,
) -> ParseLayer {
    // 旧包含状态没有本票的初始未定与极值根事实；只保留调用者字段兼容。
    let _ = incr_inclusion;
    let previous_strokes_len = incr_strokes.to_result().len();
    incr_strokes.append_bar_incremental(bar);
    parse_layer_from_compact_facts(
        incr_strokes,
        incr_fractals,
        incr_segments,
        previous_strokes_len,
        parse_config,
    )
}

/// #1392：从紧凑包含状态构造 ParseLayer——与 [`parse_layer_from_facts`] 共用分型/新笔/段/tail
/// 同一批函数（判据同源），差别只在 `positions` 由状态维护、`merged_bars` 走 Rc 共享。
fn parse_layer_from_compact_facts(
    incr_strokes: &mut stroke::IncrStrokes,
    incr_fractals: &mut fractal::IncrFractals,
    incr_segments: &mut segment::IncrSegments,
    previous_strokes_len: usize,
    config: &super::config::ParseConfig,
) -> ParseLayer {
    let strokes = incr_strokes.update_strokes_incremental(config);
    let stable_strokes = incr_strokes.stable_strokes_prefix();
    debug_assert!(stable_strokes <= previous_strokes_len.min(strokes.len()));
    let state = incr_strokes.facts_state();
    let merged = state.merged_slice();
    *incr_fractals =
        std::mem::replace(incr_fractals, fractal::IncrFractals::empty()).append(merged);
    let fractals = incr_fractals.to_result_rc();
    // 旧段缓存只接受“旧末笔之前不变”的输入。撤尾或更深回退明确使其失效，
    // 不以新旧长度/末笔碰巧相等冒充整段输入相同。
    let can_reuse_segments = strokes.len() >= previous_strokes_len
        && stable_strokes >= previous_strokes_len.saturating_sub(1);
    let previous_segments = std::mem::replace(incr_segments, segment::IncrSegments::empty());
    let segments_state = if can_reuse_segments {
        previous_segments
    } else {
        segment::IncrSegments::empty()
    };
    *incr_segments = segments_state.append(&strokes, config);
    let (segments, pending_start) = incr_segments.to_result_rc();
    let tail = tail::build_tail(merged, &fractals, &strokes, &segments, pending_start);
    let layer = ParseLayer {
        merged_bars: state.merged_rc(),
        // #1392：由新包含状态**实际发证**（前缀一经定稿不再改写）；初始/等待为 0。
        merged_confirmed_len: state.merged_confirmed_len(),
        segments_confirmed_len: incr_segments.confirmed_len(),
        segments_earliest_unsealed: incr_segments.earliest_unsealed_from(),
        fractals,
        strokes,
        segments,
        tail: Rc::new(tail),
    };
    layer
}

/// 从同次包含事实构造 ParseLayer；双坐标与极值根属于必需输入。
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
fn parse_layer_from_facts(
    facts: &mut inclusion::InclusionFacts,
    config: &super::config::ParseConfig,
) -> ParseLayer {
    let merged = &facts.merged;
    let fractals = fractal::detect_fractals(merged);
    let stroke_facts = stroke::build_stroke_facts(facts, config);
    let strokes = stroke_facts.production_strokes();
    facts.stroke_facts = Some(stroke_facts);
    let (segments, pending_start) = segment::divide_segments_with_tail(&strokes, config);
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
