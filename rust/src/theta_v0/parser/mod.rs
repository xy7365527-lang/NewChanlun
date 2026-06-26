//! Θ_parse 子模块（reference-theta-v0.md:18-25）——**子任务，待 Lead spawn 实装**。
//!
//! ## 范围（bit-exact 对齐 `Strict/Parse.lean`）
//!
//! 唯一递归解析：K线包含 → 分型 → 新笔 → 线段（67课特征序列）→ 中枢 → canonical
//! 分解 → 未完成尾部。给定 Θ_parse ⟹ 解析结构唯一（Parse.lean total_unique 元定理）。
//!
//! ## 子任务清单（TaskCreate 分解，本骨架只定义接口契约）
//!
//! 1. **K线包含合并**（:19）：相邻区间包含即合并；向上 `high=max,low=max`，向下
//!    `low=min,high=min`；方向按前一对非包含 K 严格高低变化决定；开头无方向向前看第一个
//!    非包含对；全程无方向只输出 open-tail。[缠论可导,62/65课；开头 tie-break 设计选择]
//! 2. **分型识别**（:20）：包含处理后，顶=中 K 高低**严格**高于左右；底反之；等价不成立；
//!    第三根 K 收盘确认。[缠论可导,62课；严格等号设计选择]
//! 3. **新笔**（:21）：旧笔禁用；顶/底分型不共用 K；两极值 K 间排除两端 ≥3 根（config）；
//!    同类连续分型，顶保留更高/底保留更低/等价保留更早。[缠论可导,77/81课]
//! 4. **线段 67 课特征序列法**（:22）：向上线段看反向笔特征序列顶分型，向下反之；第一二
//!    元素无缺口直接确认段端，有缺口等反向特征序列出现相反分型确认。[缠论可导,67课]
//! 5. **中枢边界**（:23）：前三连续完成次级别走势 A,B,C，`ZD=max(low_*),ZG=min(high_*)`；
//!    闭区间 `[ZD,ZG]`，`ZD<=ZG` 成立即中枢成立。[缠论可导,17/18课；闭区间设计选择]
//! 6. **canonical 分解**（:24）：从左到右扫描；候选取最早确认端点；按最早确认时间、最低
//!    递归层、最早原始 index 破平局。[设计选择,默认值]（对齐 `Strict/Decomp.lean` gauge）
//! 7. **未完成尾部**（:25）：显式保存 `Pending*`/`AliveCenter`；不输出为 confirmed。
//!
//! ## 步骤5（中枢）移交 classifier（Lead 裁定）
//!
//! 中枢是递归构造（前三连续完成次级别走势 A,B,C），owner=classifier——classifier 从
//! parser 的 `segments` 自己构造中枢（见 `classifier/mod.rs` `detect_centers`），**不**读
//! parser 的中枢。故 `ParseLayer` **无** `centers` 字段（步骤5 不在 parser 单层流水线职责内）。
//!
//! ## 接口契约（已冻结，下游 classifier/backtest 依赖）

use super::config::ThetaConfig;
use super::types::{Bar, Fractal, PendingTail, Segment, Stroke};

/// Θ_parse 流水线分步实现（聚焦小文件，coding-style <400 行/文件）。
pub mod inclusion;
pub mod fractal;
pub mod stroke;
pub mod segment;
pub mod canonical;
pub mod tail;

/// 单层解析输出（一个级别的完整 confirmed 结构 + 未完成尾部）。
///
/// bit-exact 要求：confirmed 部分对齐 Parse.lean 的唯一递归解析（confirmed/active 切分）；
/// `tail` 显式保存未完成尾部（reference-theta-v0.md:25 + OpenTail.lean 当下状态），不混入
/// confirmed。中枢（步骤5）移交 classifier，不在本结构（见模块头）。
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
/// 七步流水线（步骤5 中枢移交 classifier，见模块头），bit-exact 对齐 `Strict/Parse.lean`：
/// 1. K线包含合并 → 2. 分型 → 3. 新笔 → 4. 线段（67课）→ 6. canonical 分解 →
/// 7. 未完成尾部 tail。签名已冻结——`(bars, config) -> ParseLayer`。
///
/// confirmed/active 切分（Parse.lean §6）：`merged_bars/fractals/strokes/segments` 是已确认
/// 结构（confirmed，交易可用）；`tail` 是未完成尾部（active，仅预警，不输出为 confirmed）。
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
    // 步骤 4：线段划分 v1 特征序列法（reference-theta-v0.md:22，第67课）。
    // 诚实范围：只断 FirstKind 确定段，SecondKind 动态确认/无分型停为 tail（见 segment.rs）。
    let segments = segment::divide_segments(&strokes);

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
    let tail = tail::build_tail(&incl.merged, &fractals, &strokes);

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

    /// 骨架契约测试：空输入 → 空 ParseLayer（接口存在性 + 空边界）。
    /// 实装后此测试扩展为 golden/property 对齐 Parse.lean fixture。
    #[test]
    fn parse_empty_bars_yields_empty_layer() {
        let cfg = ThetaConfig::default();
        let out = parse_layer(&[], &cfg);
        assert_eq!(out, ParseLayer::default());
    }
}
