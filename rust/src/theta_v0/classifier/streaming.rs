//! **自持缓冲区的增量分类器变体**（#345，Nautilus 流式适配——#342 O(n²) 根因的修复）。
//!
//! 本模块必须**无条件编译**（不随 `backtest` 模块的 `any(test, feature = "backtest_bin")`
//! 门控）——[`super::super::nautilus::strategy::ThetaCore`]（生产/实盘路径）是本模块的唯一
//! 消费方，而 `nautilus` 模块本身不受 backtest 门控，故其依赖不能落在被门控的 `backtest` 里。

use super::super::config::ThetaConfig;
use super::super::parser;
use super::super::types::Bar;
use super::{
    cand_event::CandidateStreams, classify_with_tower_events_incremental,
    recursive_tower::LeveledMove, Classification, TowerCache,
};

/// **自持缓冲区的增量分类器变体**（#345，Nautilus 流式适配——#342 O(n²) 根因的修复）。
///
/// [`backtest::incremental::IncrementalClassifier<'a>`](crate::theta_v0::backtest::incremental::IncrementalClassifier)
/// 假设调用方持有一个**生命周期内不再变化的完整切片**（批量回测场景：`dataset.bars` 一次性
/// 加载，长度固定，`bars: &'a [Bar]` + `config: &'a ThetaConfig` 全程借用）。Nautilus 流式场景
/// （`ThetaCore::on_bar`）里 `self.bars: Vec<Bar>` 逐 bar `push`，`Vec` 增长可能触发重新分配，
/// 任何指向旧内存的 `&'a [Bar]` 借用都会失效——`IncrementalClassifier` 当前的 API 形状无法
/// 直接嵌入一个"边接收边增长"的宿主结构体（详见
/// `chanlun/review-results/nt-engine-scaling-profile-20260726.md` §4）。
///
/// 本变体消解借用依赖：`config` 为 **owned 拷贝**（`ThetaConfig: Clone`，构造期克隆一次，
/// 非每 bar），`append_bar(bar)` 直接吃**单根 owned bar**（非索引进外部切片）——整个类型
/// **无生命周期参数**，可安全作为字段嵌入任意增长的宿主结构体。
///
/// **不是第二份平行实现**：增量步进算法（inclusion→fractal→stroke→segment→tail）与
/// [`ParseLayerIncr`](super::super::parser::ParseLayerIncr) 共享同一个自由函数
/// [`append_incr_layer`](super::super::parser::append_incr_layer)（#345 抽取）——两个变体
/// 的差异只在"config/bars 以借用还是 owned 形态持有"，不在算法本身。这正是修复 #342
/// 根因（"本仓库自己两条平行实现之一忘了接现成的优化件"）时必须避免重蹈的病灶。
///
/// **bit-exact 契约**：`append_bar(bar)` 逐次调用的返回值序列 == 全量
/// `classify_with_tower(parse_layer(&bars[..=i]))`（`backtest::incremental::tests::owned_bit_exact_*`
/// 锁定，`owned_matches_borrowed_variant_synthetic` 额外证两变体互等）。
#[derive(Debug, Clone)]
pub struct OwnedIncrementalClassifier {
    /// owned 拷贝（构造期克隆一次；`ThetaConfig` 不可变，不随 bar 增长而变化）。
    config: ThetaConfig,
    incr_inclusion: parser::inclusion::IncrInclusion,
    incr_fractals: parser::fractal::IncrFractals,
    incr_strokes: parser::stroke::IncrStrokes,
    incr_segments: parser::segment::IncrSegments,
    /// 增量塔缓存（跨 bar 复用——与借用变体同一身份稳定机制）。
    tower_cache: TowerCache,
    /// ★#346 MED-2：`append_bar` 调用次数（= 分类器内部已消费的原始 bar 数，非 merged 后的
    /// `incr_inclusion` 计数——inclusion 会把包含关系的 bar 折叠掉，不能代表宿主侧 bar 数）。
    /// 唯一用途：给 [`bar_count`](Self::bar_count) 供宿主（`ThetaCore`）做锁步护栏比对
    /// （宿主 `self.bars.len()` 应恒等于本计数——跳 bar/重复调用会使二者失配）。
    bars_appended: usize,
}

impl OwnedIncrementalClassifier {
    /// 构造（owned config 拷贝，空缓存，首个 bar 从零起步）。
    pub fn new(config: ThetaConfig) -> Self {
        Self {
            config,
            incr_inclusion: parser::inclusion::IncrInclusion::empty(),
            incr_fractals: parser::fractal::IncrFractals::empty(),
            incr_strokes: parser::stroke::IncrStrokes::empty(),
            incr_segments: parser::segment::IncrSegments::empty(),
            tower_cache: TowerCache::new(),
            bars_appended: 0,
        }
    }

    /// `append_bar` 已被调用的次数（= 本分类器认为自己消费过的原始 bar 数）。
    ///
    /// 宿主结构体（[`crate::theta_v0::nautilus::strategy::ThetaCore`]）用它核对自己的
    /// `bars.len()` 是否与分类器内部状态锁步——二者失配即跳 bar/重复调用（增量血缘契约破裂）。
    pub fn bar_count(&self) -> usize {
        self.bars_appended
    }

    /// **per-bar 增量重分类（自持缓冲区）**：追加单根 bar，返回 `(classification, tower)` ==
    /// `classify_with_tower(parse_layer(&bars[..=i]))`，bit-exact（`i` = 累计追加次数-1）。
    ///
    /// 与借用变体 `classify_at` 的唯一差异：吃 owned `Bar` 而非索引进外部切片——调用方
    /// （如 `ThetaCore::bars.push(new_bar)` 后）直接把新 bar 传入本方法，无需持有一个生命
    /// 周期内稳定不变的完整切片。
    pub fn append_bar(&mut self, bar: Bar) -> (Classification, Vec<std::rc::Rc<Vec<LeveledMove>>>) {
        let (classification, tower, _) = self.append_bar_events(bar);
        (classification, tower)
    }

    /// #550 三元事件通道（SPEC #547 第四入口）。
    ///
    /// `append_bar` 保持既有二元 API；需要候选事件尾的消费方显式选择本入口。两者共用
    /// 同一增量 parse/tower/cache，事件簿随 `TowerCache` 跨 bar 持久。
    pub fn append_bar_events(
        &mut self,
        bar: Bar,
    ) -> (
        Classification,
        Vec<std::rc::Rc<Vec<LeveledMove>>>,
        CandidateStreams,
    ) {
        let l0 = parser::append_incr_layer(
            &mut self.incr_inclusion,
            &mut self.incr_fractals,
            &mut self.incr_strokes,
            &mut self.incr_segments,
            bar,
            &self.config.parse,
        );
        self.bars_appended += 1;
        classify_with_tower_events_incremental(&l0, &self.config, &mut self.tower_cache)
    }
}
