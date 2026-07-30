//! #110 投影层骨架 + 级别身份标签（SPEC #109 第一票，expand 阶段）+ T2 (#171) 三元锚索引
//! + T5a (#207) 方向退役：锚简化为两元（极值价, 组锚@ℓ）。
//!
//! 每级别一实例 [`LevelProjectionLayer`]：身份信息 + 候选集 `Rc` 共享 + 两元锚索引 +
//! 跨级确认查询入口（两元锚描述体）+ 时间跨度估计。
//!
//! 纪律（issue110-impl-20260721 / #168 裁定 3 / ADR 20260723 裁定 1）：
//! - **门关零开销**：投影构建（`Rc::clone`、跨度扫描、锚索引构建、Layer 分配）完整包在
//!   `enabled` 分支内（`classifier/mod.rs` stamping 路径），关闭时仅留 `None`；
//! - **禁第二查法**：查询入口只产**描述体**不持判定——本模块零判定消费
//!   （admission/nest/runner 不读本层做裁决）；
//! - **锚单一来源**：两元锚（极值价, 组锚@ℓ）经 T1 (#170) 供给线解析
//!   （`fractal_at_source` / `merged_group_anchor`），本层不二次推导；
//! - **方向退役**（ADR 20260723 裁定 1，T5a #207）：身份判据 = 同点递归，方向整体退役
//!   出身份层——索引键/查询描述体不再携带方向；同一 x 可在不同级别分别为顶/底
//!   （各级分型类型是各级自己的结构事实，都可为真），同点跨型不产生价格二义
//!   （T1 探针已证：价格与组锚同锚于 x 处 L0 分型）。
//! - SPEC story 5「完整级别身份（级别、源级别链、方向）」本票只落 `level`，
//!   #111 补齐。

use std::collections::HashMap;
use std::rc::Rc;

use super::super::parser::fractal::fractal_at_source;
use super::super::parser::inclusion::merged_group_anchor;
use super::super::types::{Bar, Fractal, Side, Tick};
use super::bsp::BspPoint;

/// #218 面 B：T1 两元锚解析 oracle 构造子（**单一来源**，禁第二查法）——
/// `anchor_at(source_index) = (fractal_at_source(fractals, x).price, merged_group_anchor(merged_bars, x))`，
/// 与本文件锚索引构建（:143-145 区域）同一查法；任一侧供给未命中 ⟹ `None`
///（锚不可解 = 身份合取不可证，诚实判负）。生产调用方（nest 终端背书二类点判同
/// oracle，spec owner-attribution-fix-20260724 ID-2）与测量共核同一来源。
pub fn anchor_resolver<'a>(
    fractals: &'a [Fractal],
    merged_bars: &'a [Bar],
) -> impl Fn(usize) -> Option<(Tick, usize)> + 'a {
    move |x| {
        match (fractal_at_source(fractals, x), merged_group_anchor(merged_bars, x)) {
            (Some(f), Some(a)) => Some((f.price, a)),
            _ => None,
        }
    }
}

/// 投影层门（T3 (#172) 并门后 = **派生机制位**：独立配置面退役，层载由链路径是否启用
/// 单一驱动——链活 ⟹ 层必载（含锚索引，恰好存在物化基座，索引成本即链判成本一部分）、
/// 链死不载（零开销红线不死，#110 纪律）。生产唯一写入点 = π 入口派生
///（`backtest::admission::chain_driven_level_projection`，#168 裁定 3）；classifier
/// stamping 仍读本机制位（classifier 不可读 backtest 门 env——层次纪律）。
/// `config.rs` 默认断言 = 「链启用 ⟹ 层必载」形态。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelProjectionConfig {
    /// 开启后 stamping 路径为每级构造 [`LevelProjectionLayer`]；关闭 = `None`（零开销）。
    pub enabled: bool,
}

impl LevelProjectionConfig {
    /// T3 (#172) 并门派生构造子：链路径启用态 ⟹ 层载态（恒等派生——「链启用 ⟹ 层必载」
    /// 形态锁；独立双门被否：「链开层关」错位态须靠断言禁掉，等于承认两门本不可独立，
    /// #168 裁定 3）。
    pub fn for_chain(chain_enabled: bool) -> Self {
        Self { enabled: chain_enabled }
    }
}

/// 级别身份（#110 第一票：仅 `level`；源级别链/方向 #111 补齐）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelIdentity {
    /// 所属级别 ℓ（与 `Classification.levels` 索引同源）。
    ///
    /// ★#455 诚实更新（#434 grilling 交棒件）：本字段是级别身份的**唯一**存储事实——
    /// 曾在 `BspPoint` 存在的同形拷贝字段全仓恒为 0、无下游级别语义消费者，仅有恒真
    /// equality 分量及 Debug/digest 机械消费，现已删除（`CONTEXT.md`「级别身份」词条）。
    pub level: u32,
}

/// 锚索引条目（T2 #171；T5a #207 去方向位后键 = 极值价单维）：命中候选的坐标 + 其组锚@ℓ 回执
///（纯描述，零判定语义）。类型名 `TripleAnchorEntry` 保留 T2 历史指称。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TripleAnchorEntry {
    /// 候选坐标（`BspPoint.source_index`，L0 原始 K 序坐标系）。
    pub source_index: usize,
    /// 该坐标所在合并组的锚（= 组内首根序号，T1 供给线 `merged_group_anchor` 同一来源）。
    pub group_anchor: usize,
}

/// 跨级确认查询**描述体**（只描述不判定，禁第二查法）：
/// 按两元锚前缀 (极值价) 命中条目的回执，判定语义零携带。
///
/// T2 (#171)：入参由旧序号锚 (`source_index`, `side`) 改锚前缀；组锚维度随条目
/// 回执（W 底同价双脚 → 同键多条目、组锚各异），跨级组锚对齐语义归 T3 消费侧，本层
/// 不揣测定级。命中计数 = `matches.len()`。
/// T5a (#207)：方向分量退役（ADR 20260723 裁定 1——同点递归即身份，方向完全多余）；
/// 查询只按极值价，分型类型（顶/底）是各级自己的结构事实，不进查询键。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossLevelConfirmationQuery {
    /// 查询发起级别（本层级别）。
    pub level: u32,
    /// 查询极值价（i64 Tick，精确等值无容差——v3 硬禁令）。
    pub extreme_price: Tick,
    /// 命中条目（组锚@ℓ + 候选坐标；按候选坐标升序登记）。空 = 未命中。
    pub matches: Vec<TripleAnchorEntry>,
}

/// 单级别投影层（#110 骨架 + T2 (#171) 锚索引 + T5a (#207) 去方向位）。
#[derive(Debug, Clone, PartialEq)]
pub struct LevelProjectionLayer {
    /// 级别身份。
    pub identity: LevelIdentity,
    /// 该级候选集（与 `LevelState.bsp` 同一 `Rc`，`Rc::clone` O(1) 共享，零拷贝）。
    pub candidates: Rc<Vec<BspPoint>>,
    /// 时间跨度估计：候选集 `source_index` 的 `[min, max]`（空集 = `None`）。
    pub span_estimate: Option<(usize, usize)>,
    /// T2 (#171) 锚索引 + T5a (#207) 去方向位：**极值价 → 条目集**（组锚@ℓ + 候选坐标）。
    ///
    /// - 键类型与 T1 (#170)/T5a (#207) gate `by_triple_anchor` 键域同型（`Tick`/`usize`）；
    ///   级别载体各随其域——T1 级别居键内 `(ℓ, …)`，本层级别居 `identity.level`（层即级别）。
    /// - **方向退役**（ADR 20260723 裁定 1）：索引键不再携带方向——同一 x 跨型共点
    ///   不产生价格二义（T1 探针已证），方向位在键中零工作。登记资格保持「任一方向
    ///   确认的买卖点（= 该级确认的拐点，顶或底皆可）」——登记人口与方向时代逐字节
    ///   同集，只是双向确认的候选从两键各一条合流为同键一条。
    /// - 极值价 = 候选坐标处 L0 confirmed 分型的 `Fractal.price`（i64 Tick，精确等值
    ///   无容差——v3 硬禁令）；跨级不变量只有极值价（教义 ADR 2026-07-22）。
    /// - 组锚 = 候选坐标所在合并组首根序号。两供给均为 T1 单一来源（禁第二查法）。
    /// - 同键多条目 = 同价的不同脚（W 底同价双脚 → 组锚各异；跨型同点 → 同点坐标各异），
    ///   按候选坐标升序。
    pub triple_anchor_index: HashMap<Tick, Vec<TripleAnchorEntry>>,
    /// T2 (#171) 缺锚跳过计数：有方向确认但 L0 分型/合并层供给未命中而未登记的候选数
    ///（T1 `n_anchor_misses` 同款诚实口径——照实落账，禁降级第二查法）。
    pub n_anchor_misses: usize,
}

impl LevelProjectionLayer {
    /// 由级别账本构造（仅在门开分支被调用；`Rc::clone` + 单趟跨度扫描 + 单趟锚索引构建）。
    ///
    /// `fractals`/`merged_bars` = T1 (#170) 两元锚供给（L0 confirmed 分型账本 + L0
    /// 包含层——当前唯一包含层，坐标全系 L0 source_index；「该级包含层」为将来多级
    /// 包含层预留的措辞。stamping 点从 `ParseLayer` 的 `Rc` 共享借用，只读零拷贝）；
    /// 层本体只留索引不持供给（索引自含 T3 查询所需全部回执，持供给只会延长分配寿命）。
    pub fn from_level(
        level: u32,
        candidates: &Rc<Vec<BspPoint>>,
        fractals: &[Fractal],
        merged_bars: &[Bar],
    ) -> Self {
        let span_estimate = candidates.iter().fold(None, |acc, point| match acc {
            None => Some((point.source_index, point.source_index)),
            Some((lo, hi)) => Some((lo.min(point.source_index), hi.max(point.source_index))),
        });
        // T2 (#171) + T5a (#207)：单趟锚索引构建。任一方向确认（`confirm_side`）是索引
        // 资格前提——无方向确认的候选不是该级确认拐点（顶/底皆否），在存在性查询下永不
        // 命中（与方向时代登记人口逐字节同集），不进索引、不计缺锚。方向不进键
        //（ADR 20260723 裁定 1：双向确认者同键一条，不再两键各一）。
        let mut triple_anchor_index: HashMap<Tick, Vec<TripleAnchorEntry>> = HashMap::new();
        let mut n_anchor_misses = 0usize;
        for point in candidates.iter() {
            let is_long = point.bits.confirm_side(Side::Long);
            let is_short = point.bits.confirm_side(Side::Short);
            if !is_long && !is_short {
                continue;
            }
            // 两元锚（极值价, 组锚）单一查法 = T1 供给线（禁第二查法）；缺一 = 诚实跳过 + 计数。
            let (Some(price), Some(group_anchor)) = (
                fractal_at_source(fractals, point.source_index).map(|f| f.price),
                merged_group_anchor(merged_bars, point.source_index),
            ) else {
                n_anchor_misses += 1;
                continue;
            };
            let entry = TripleAnchorEntry {
                source_index: point.source_index,
                group_anchor,
            };
            triple_anchor_index.entry(price).or_default().push(entry);
        }
        Self {
            identity: LevelIdentity { level },
            candidates: Rc::clone(candidates),
            span_estimate,
            triple_anchor_index,
            n_anchor_misses,
        }
    }

    /// 跨级确认查询入口（T5a #207 去方向形态）：按极值价查本层索引，只产描述体
    ///（命中组锚/候选坐标回执，计数 = `matches.len()`），不持判定；分型类型（顶/底）
    /// 是各级自己的结构事实，不进查询键（ADR 20260723 裁定 1）。
    pub fn cross_level_query(&self, extreme_price: Tick) -> CrossLevelConfirmationQuery {
        let matches = self
            .triple_anchor_index
            .get(&extreme_price)
            .cloned()
            .unwrap_or_default();
        CrossLevelConfirmationQuery {
            level: self.identity.level,
            extreme_price,
            matches,
        }
    }
}

impl CrossLevelConfirmationQuery {
    /// 单源：按 `source_index` 精确等值查 `matches` 中首个 [`TripleAnchorEntry`]（issue #747 C1，
    /// `admission.rs::resolve_foot` 单源——「禁第二查法」注释语义原样保留）。
    ///
    /// 族内独立成员：本查询作用于 [`TripleAnchorEntry`]（已按极值价窄化的组锚回执），
    /// 与 `classifier::bsp` 的 `bsp_at`/`bind_turn`/`bsp_bit_at` 三件族（作用于 [`super::bsp::BspPoint`]）
    /// 是同一「source_index 等值 join」模式在不同类型上的并行实现，二者不同型、不强并为
    /// 一函数（照实登记，见票内「先核语义是否逐字同构」条）。
    pub fn entry_at(&self, source_index: usize) -> Option<&TripleAnchorEntry> {
        self.matches.iter().find(|e| e.source_index == source_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::{BspBits, FractalKind};

    /// 门关守护：默认配置下不构造任何投影层（零开销红线）。
    #[test]
    fn constructs_nothing() {
        let config = LevelProjectionConfig::default();
        assert!(!config.enabled, "#110 门必须默认关");
        let layer: Option<LevelProjectionLayer> =
            config.enabled.then(|| unreachable!("门关分支不得构造 Layer"));
        assert!(layer.is_none());
    }

    fn point(source_index: usize, bits: BspBits) -> BspPoint {
        BspPoint {
            source_index,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        }
    }

    fn bar(i: usize) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: 0,
            high: 0,
            low: 0,
            close: 0,
            volume: 1,
            untradable: false,
        }
    }

    fn fractal(kind: FractalKind, source_index: usize, price: Tick) -> Fractal {
        Fractal {
            kind,
            source_index,
            timestamp: source_index as i64,
            price,
        }
    }

    /// T2 (#171) 夹具供给：merged 组锚序列 [2,4,8,10]（组 i 覆盖 [src_i, src_{i+1})），
    /// 分型 = x=4 底 @100 与 x=10 底 @100（W 底同价双脚）+ x=8 顶 @170（异价异向对照）。
    fn supplies() -> (Vec<Fractal>, Vec<Bar>) {
        let merged = vec![bar(2), bar(4), bar(8), bar(10)];
        let fractals = vec![
            fractal(FractalKind::Bottom, 4, 100),
            fractal(FractalKind::Top, 8, 170),
            fractal(FractalKind::Bottom, 10, 100),
        ];
        (fractals, merged)
    }

    #[test]
    fn span_estimate_covers_candidates() {
        let point_at = |source_index: usize| point(source_index, Default::default());
        let candidates = Rc::new(vec![point_at(7), point_at(3)]);
        let layer = LevelProjectionLayer::from_level(2, &candidates, &[], &[]);
        assert_eq!(layer.identity.level, 2);
        assert_eq!(layer.span_estimate, Some((3, 7)));
        assert!(Rc::ptr_eq(&layer.candidates, &candidates));
        // 空 bit 向量 ⟹ 无任何方向确认（非该级确认拐点）⟹ 不进索引、不计缺锚。
        assert!(layer.triple_anchor_index.is_empty());
        assert_eq!(layer.n_anchor_misses, 0);
    }

    /// T2 (#171) → T5a (#207)：层索引两元锚与 T1 供给线同源一致——同一候选经层索引
    /// 解析出的 (极值价, 组锚) 与 `fractal_at_source`/`merged_group_anchor` 直出逐项相等
    ///（= T5a 登记侧 `by_triple_anchor` 键 (ℓ, 极值价, 组锚@ℓ) 的后两元；禁第二查法
    /// 靠本对照锁死——若层内有第二套推导，键值会漂移）。
    #[test]
    fn triple_anchor_index_matches_t1_supply_lines() {
        let (fractals, merged) = supplies();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        let candidates = Rc::new(vec![point(4, buy1)]);
        let layer = LevelProjectionLayer::from_level(1, &candidates, &fractals, &merged);
        let price = fractal_at_source(&fractals, 4).expect("夹具前提：x=4 有分型").price;
        let anchor = merged_group_anchor(&merged, 4).expect("夹具前提：x=4 有组锚");
        let entries = layer
            .triple_anchor_index
            .get(&price)
            .expect("候选须按极值价登记（方向退役，不进键）");
        assert_eq!(
            entries.as_slice(),
            &[TripleAnchorEntry { source_index: 4, group_anchor: anchor }],
            "(极值价, 组锚) 须与 T1 供给线直出一致"
        );
        assert_eq!(layer.n_anchor_misses, 0);
    }

    /// T2 (#171) → T5a (#207)：W 底同价双脚——同价（@100）的两候选分属不同组锚，
    /// 同键登记两条目（按候选坐标升序）；跨级查询回执双脚全返（不问方向/分型类型）。
    #[test]
    fn w_bottom_double_foot_same_key_distinct_anchors() {
        let (fractals, merged) = supplies();
        let buy2 = BspBits { buy2: true, ..Default::default() };
        let candidates = Rc::new(vec![point(4, buy2), point(10, buy2)]);
        let layer = LevelProjectionLayer::from_level(1, &candidates, &fractals, &merged);
        let entries = layer
            .triple_anchor_index
            .get(&100)
            .expect("同价双脚须同键");
        assert_eq!(
            entries.as_slice(),
            &[
                TripleAnchorEntry { source_index: 4, group_anchor: 4 },
                TripleAnchorEntry { source_index: 10, group_anchor: 10 },
            ],
            "双脚同键多条目、组锚各异、按坐标升序"
        );
        let receipt = layer.cross_level_query(100);
        assert_eq!(receipt.level, 1);
        assert_eq!(receipt.extreme_price, 100);
        assert_eq!(receipt.matches.len(), 2, "描述体回执 = 双脚全量（计数 = len）");
        assert_eq!(receipt.matches.as_slice(), entries.as_slice());
        // 异价前缀不命中 = 空回执（诚实无命中，非判负）。
        assert!(layer.cross_level_query(170).matches.is_empty());
        assert!(layer.cross_level_query(999).matches.is_empty());
    }

    /// T2 (#171) → T5a (#207)：bit 非互斥——双向皆确认的候选是同一只脚（同一坐标同一
    /// 组锚），方向退役后同键**一条**登记（不再两键各一）；存在性/锚解析不因此重复计数。
    #[test]
    fn dual_side_confirmed_point_registers_under_both_keys() {
        let (fractals, merged) = supplies();
        let both = BspBits { buy1: true, sell1: true, ..Default::default() };
        let candidates = Rc::new(vec![point(8, both)]);
        let layer = LevelProjectionLayer::from_level(1, &candidates, &fractals, &merged);
        let expected = [TripleAnchorEntry { source_index: 8, group_anchor: 8 }];
        assert_eq!(layer.triple_anchor_index.get(&170).map(Vec::as_slice), Some(expected.as_slice()));
        assert_eq!(
            layer.triple_anchor_index.len(),
            1,
            "方向退役：双向确认 = 同一只脚，同键一条（方向时代为两键各一）"
        );
        assert_eq!(
            layer.cross_level_query(170).matches.len(),
            1,
            "同脚回执不重复（存在性语义 = 有/无，非方向计数）"
        );
    }

    /// T2 (#171)：缺锚候选跳过 + 计数（T1 `n_anchor_misses` 同款诚实口径）——
    /// x 处无 confirmed 分型，或 x 先于合并层首组（`merged_group_anchor` None），均不登记。
    #[test]
    fn anchor_miss_skips_and_counts() {
        let (fractals, merged) = supplies();
        let buy1 = BspBits { buy1: true, ..Default::default() };
        // x=5：合并组内（锚 4）但无分型 ⟹ 缺极值价；x=0：有分型（夹具外加）但先于首组 ⟹ 缺组锚。
        //（分型账本按 source_index 升序是 `fractal_at_source` 二分前提，插入须保序。）
        let mut fractals = fractals;
        fractals.insert(0, fractal(FractalKind::Bottom, 0, 42));
        let candidates = Rc::new(vec![point(5, buy1), point(0, buy1), point(4, buy1)]);
        let layer = LevelProjectionLayer::from_level(1, &candidates, &fractals, &merged);
        assert_eq!(layer.n_anchor_misses, 2, "两种缺锚各计一次（有方向确认才计）");
        assert_eq!(
            layer.triple_anchor_index.len(),
            1,
            "只有锚齐的 x=4 登记；缺锚候选零降级（禁第二查法）"
        );
        assert!(layer.triple_anchor_index.contains_key(&100));
    }
}
