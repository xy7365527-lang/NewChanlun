//! 逐笔成交数据加载层——tick → `Bar`(O=H=L=C=成交价) → 现役 theta_v0 判定链（seam 零改）。
//!
//! ## seam（issue #973 一行）
//!
//! `逐笔成交 → Bar(O=H=L=C=成交价, 亚秒时间戳) → 现役 theta_v0 判定链（分型/笔/线段/中枢，零改）`
//!
//! 本模块是 seam 的**左半边**（数据接入）：每笔成交直接填一根 `Bar`（O=H=L=C=成交价），
//! 喂现役 `parser::parse_layer`（分型/包含/笔/线段）与 `classifier::center::center_from_segments`
//! （中枢）——判定链代码**一行不改**（#969 已证四层粒度无关）。
//!
//! ## 与现役 `data.rs` 的关系（#971 裁定 ③并列，不复用 `bar_seconds`）
//!
//! 逐笔 bar **不是等时长**（活跃时段一秒多笔、停牌可隔数小时无笔），`bar_seconds`（秒/bar）
//! 对逐笔无定义（#971）。本模块走**独立数据路径**：
//! - 不复用 `data.rs` 的 `bar_seconds` / `date_to_timestamp`（14 位秒，会塌缩同秒多笔）/
//!   `bars_per_year`（等时长公式）；
//! - **只复用** `types::Bar` + `types::quantize` + 结构判定链（这是唯一且真正的复用面，见 #971 §三）；
//! - 时间戳直接落 `Bar.timestamp`（i64 epoch ms/ns，types.rs:20「只用于排序，不参与价格运算」）。
//!
//! ## 数据源（#968）
//!
//! - **Binance `aggTrade`**（REST `/api/v3/aggTrades` 数组，ms）：字段 `a`(聚合成交ID)/`p`(价)/
//!   `q`(量)/`f`/`l`/`T`(成交时间ms)/`m`/`M`。逐字核到官方文档（#968 §一）。
//! - **Binance `trade`**（REST `/api/v3/trades` 数组，ms）：字段 `id`/`price`/`qty`/`quoteQty`/
//!   `time`(ms)/`isBuyerMaker`/`isBestMatch`。逐字核到官方文档（#968 §一「严格逐笔用 trade 流」）。
//! - **Databento `Trades`**（ns，可选）：本模块只断言 `ts_event`(ns)/`price`/`size` 三个字段——
//!   这是 Databento DBN 各 schema 的通用字段；**精确字段清单/文件格式（DBN 二进制 vs CSV/JSON
//!   导出）/rtype/action/side 过滤未经人工核**（#968 §二「docs 是 JS 渲染 SPA，查不到」），落地前
//!   须在 databento.com portal 人工核，见 [`load_databento_trades`] 的诚实声明。
//!
//! ## 认识论等级：L1（管线正确性，零信息增量）
//!
//! 读 JSON → 校验 → 时间序还原 → 量化为整数 tick `Bar`，确定性数据变换，验证管线无 bug，
//! 不验证 Θ 在市场有效（formalization-validity-domain 231号）。
//!
//! ## 校验口径（fail-loud schema 校验 ≠ #922 坏 tick 清洗）
//!
//! 本模块做**结构级 fail-loud 校验**：文件缺失 / JSON 解析失败 / 空数组 / 核心字段缺失 / 价格或量
//! 非有限正数 / 时间戳非正 —— 一律 `Err`（数据漂移即炸，no-patch）。**不做** #922 的统计级坏 tick
//! 清洗（如 BRN 全日价格≈83 而 low=0.080 那类「通过结构校验却异常」的坏点）——本票用干净样本绕开
//! （见报告照实残余）。
//!
//! ## volume 口径（诚实标注）
//!
//! `Bar.volume: i64` 是整数域，小数基准资产量（BTC qty≈1e-4）`qty as i64` 截断为 0——对齐现役
//! `data.rs`（`volume: v as i64`）与 `bar_adapter.rs`（`nb.volume as i64`）口径。结构判定链不消费
//! volume（#969 已证只碰 OHLC），忠实成交量保存在 [`TickFeed::volumes`]（f64，与 bars 同索引同长）
//! 供执行层（#872/#960）接线，不静默丢弃。
//!
//! `untradable` 恒 `false`（每笔成交 qty>0 经 fail-loud 校验后必为可交易）——**刻意不用截断后的
//! `i64` volume 判 untradable**：小数基准资产量截断为 0 不代表「无成交」（spec:53 的 `volume=0 ⟹
//! untradable` 是为 OHLC 聚合 bar 写的，那里 volume 是整单位、0 = 该时段无成交；逐笔 bar 每根
//! 本身就是一笔成交）。

use super::super::config::ThetaConfig;
use super::super::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::path::Path;

/// 逐笔成交加载结果。`bars` 是 seam 输入（直接喂 `parser::parse_layer`）。
#[derive(Debug, Clone)]
pub struct TickFeed {
    pub symbol: String,
    /// 逐笔 → Bar：O=H=L=C=成交价（量化后整数 tick），volume=qty（i64 截断，见模块头），
    /// timestamp=成交时间（epoch ms/ns，直接作 `Bar.timestamp`）。source_index=局部下标 0..n。
    pub bars: Vec<Bar>,
    /// 与 `bars` 同索引、同长的原始成交量（f64，基准资产 qty/size）。`Bar.volume` 是 i64 整数域，
    /// 小数基准资产量截断后丢精度——结构链不消费 volume，忠实量保存在此（见模块头）。
    pub volumes: Vec<f64>,
}

impl TickFeed {
    /// 成交笔数（= bars 长度）。
    pub fn n_trades(&self) -> usize {
        self.bars.len()
    }
}

/// 最小内部记录：seam 只消费价格/量/时间/ID 四项，其余 vendor 字段（quoteQty/isBuyerMaker/f/l/m/M）
/// 不进 `Bar`，serde 默认忽略。
#[derive(Debug, Clone, Copy, PartialEq)]
struct RawTick {
    /// 成交时间（epoch ms 或 ns，直接作 `Bar.timestamp`）。
    timestamp: Timestamp,
    /// 聚合成交 ID / trade ID（排序二级键）。Databento 无逐笔 ID ⟹ 0，靠稳定排序 + source_index
    /// 破平局（reference:16 的 `(timestamp, source_index)` 在 source_index 唯一处恒破平局）。
    trade_id: i64,
    price: f64,
    qty: f64,
}

/// Binance REST `/api/v3/aggTrades` 数组元素（只取 seam 需要的最小字段，其余 serde 忽略）。
#[derive(Deserialize)]
struct BinanceAggTrade {
    #[serde(rename = "a")]
    agg_id: i64,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    qty: String,
    #[serde(rename = "T")]
    time_ms: i64,
}

/// Binance REST `/api/v3/trades` 数组元素（严格逐笔，字段名与 aggTrade 不同）。
#[derive(Deserialize)]
struct BinanceTrade {
    id: i64,
    price: String,
    qty: String,
    time: i64,
}

/// Databento `Trades` 记录（JSON 导出）。只断言 `ts_event`/`price`/`size` 三个通用字段。
#[derive(Deserialize)]
struct DatabentoTrade {
    ts_event: i64,
    price: f64,
    size: f64,
}

/// 读 tick 文件（与 data.rs `load_symbol` 同款 fail-loud）。
fn read_tick_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("读取 {path:?} 失败: {e}"))
}

/// 把 Binance 字符串价格/量解析为 f64（Binance 价格/量是 JSON 字符串；`"NaN"`/`"Infinity"` 能过
/// `parse::<f64>` 但被 `finalize_feed` 的 `is_finite` 拒掉）。
fn parse_binance_f64(s: &str, source: &str, field: &str, idx: usize) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|e| format!("{source} 第 {idx} 笔 {field} 非法（`{s}`）: {e}"))
}

/// 校验 + 时间序还原 + 量化为 `Bar`（三个 loader 共用，单源）。
///
/// 1. fail-loud 校验：空 / 价格非有限正数 / 量非有限正数 / 时间戳非正。
/// 2. 时间序还原：按 `(timestamp, trade_id)` 升序排序。Binance REST `limit` 分页拼接后整体可能
///    降序（最新在前），喂时间序判定链前必须还原——排序是**时间序还原**，不是清洗（不删不改任何
///    一笔成交）。
/// 3. 量化：O=H=L=C=quantize(price)；volume=qty as i64；source_index=排序后局部下标 0..n。
fn finalize_feed(
    symbol: &str,
    config: &ThetaConfig,
    mut raw: Vec<RawTick>,
) -> Result<TickFeed, String> {
    if raw.is_empty() {
        return Err(format!("{symbol} 无成交记录"));
    }
    for (i, t) in raw.iter().enumerate() {
        if !t.price.is_finite() || t.price <= 0.0 {
            return Err(format!("{symbol} 第 {i} 笔价格非法：{}", t.price));
        }
        if !t.qty.is_finite() || t.qty <= 0.0 {
            return Err(format!("{symbol} 第 {i} 笔量非法：{}", t.qty));
        }
        if t.timestamp <= 0 {
            return Err(format!("{symbol} 第 {i} 笔时间戳非法：{}", t.timestamp));
        }
    }
    raw.sort_by_key(|t| (t.timestamp, t.trade_id));

    let tick_size = config.tick.tick_size;
    let mut bars = Vec::with_capacity(raw.len());
    let mut volumes = Vec::with_capacity(raw.len());
    for (i, t) in raw.iter().enumerate() {
        let px = quantize(t.price, tick_size);
        volumes.push(t.qty);
        bars.push(Bar {
            source_index: i,
            timestamp: t.timestamp,
            open: px,
            high: px,
            low: px,
            close: px,
            volume: t.qty as i64,
            untradable: false,
        });
    }
    Ok(TickFeed {
        symbol: symbol.to_string(),
        bars,
        volumes,
    })
}

/// 加载 Binance `aggTrade`（REST `/api/v3/aggTrades` JSON 数组，ms）。
pub fn load_binance_agg_trades(
    path: &Path,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let text = read_tick_file(path)?;
    parse_binance_agg_trades(&text, &path.display().to_string(), symbol, config)
}

/// 从文本解析 Binance `aggTrade` 数组（load 与测试共用）。
fn parse_binance_agg_trades(
    text: &str,
    source: &str,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let arr: Vec<BinanceAggTrade> =
        serde_json::from_str(text).map_err(|e| format!("解析 {source} 失败: {e}"))?;
    let raw = arr
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let price = parse_binance_f64(&t.price, source, "price", i)?;
            let qty = parse_binance_f64(&t.qty, source, "qty", i)?;
            Ok(RawTick {
                timestamp: t.time_ms,
                trade_id: t.agg_id,
                price,
                qty,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    finalize_feed(symbol, config, raw)
}

/// 加载 Binance `trade`（REST `/api/v3/trades` JSON 数组，ms，严格逐笔）。
pub fn load_binance_trades(
    path: &Path,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let text = read_tick_file(path)?;
    parse_binance_trades(&text, &path.display().to_string(), symbol, config)
}

/// 从文本解析 Binance `trade` 数组（load 与测试共用）。
fn parse_binance_trades(
    text: &str,
    source: &str,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let arr: Vec<BinanceTrade> =
        serde_json::from_str(text).map_err(|e| format!("解析 {source} 失败: {e}"))?;
    let raw = arr
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let price = parse_binance_f64(&t.price, source, "price", i)?;
            let qty = parse_binance_f64(&t.qty, source, "qty", i)?;
            Ok(RawTick {
                timestamp: t.time,
                trade_id: t.id,
                price,
                qty,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    finalize_feed(symbol, config, raw)
}

/// 加载 Databento `Trades`（JSON 数组，ns，**可选**）。
///
/// ★诚实声明（#968 §二）：本 loader 只断言 `ts_event`(ns)/`price`/`size` 三个字段，且**把每个记录
/// 都当作成交**（不按 `rtype`/`action`/`side` 过滤——Databento Trades schema 只含成交记录，但若喂了
/// 含其他记录类型的导出须先过滤）。精确字段清单 / 文件格式（DBN 二进制 vs CSV/JSON 导出）未经人工核
/// （databento docs 是 JS 渲染 SPA，curl 拿不到正文），落地前须在 databento.com portal 核。字段名按
/// Databento DBN 公开惯例（`ts_event` ns 精度、`price`、`size`）。
pub fn load_databento_trades(
    path: &Path,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let text = read_tick_file(path)?;
    parse_databento_trades(&text, &path.display().to_string(), symbol, config)
}

/// 从文本解析 Databento `Trades` 数组（load 与测试共用）。
fn parse_databento_trades(
    text: &str,
    source: &str,
    symbol: &str,
    config: &ThetaConfig,
) -> Result<TickFeed, String> {
    let arr: Vec<DatabentoTrade> =
        serde_json::from_str(text).map_err(|e| format!("解析 {source} 失败: {e}"))?;
    let raw = arr
        .iter()
        .map(|t| RawTick {
            timestamp: t.ts_event,
            trade_id: 0,
            price: t.price,
            qty: t.size,
        })
        .collect();
    finalize_feed(symbol, config, raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::parser::parse_layer;

    fn cfg_with_gap(new_stroke_min_gap: u32) -> ThetaConfig {
        let mut c = ThetaConfig::default();
        c.parse.new_stroke_min_gap = new_stroke_min_gap;
        c
    }

    /// 20 页 × 1000 笔 Binance BTCUSDT aggTrade（2026-08-15，约 2.3 小时，8298s）。下载 + 排序脚本
    /// 与 provenance 见报告 §1。用 20000 笔而非 prototype 的 5000 笔：5000 笔（~37min）过短，
    /// 线段层因样本短而「塌缩到 4 段」（prototype #972 退化点2 的实情），见报告 §2 的订正。
    const BTC_FIXTURE: &str = include_str!("../../../tests/fixtures/btc_agg_trades_20000.json");

    /// 重标定后的 `new_stroke_min_gap`（扫参表见报告 §2；稳定点判据：笔中位尺寸逃出 $0.01 噪声底 +
    /// 线段层非平凡 + 笔/段比稳定带）。gap=3 时笔中位尺寸 = $0.01（噪声主导）；gap=10 起逃出。
    const RECALIBRATED_GAP: u32 = 10;

    /// L1：单笔 aggTrade → Bar 的 O=H=L=C 量化 / 时间戳 / source_index / volume 口径。
    ///
    /// 非平凡断言：量化数学（63054.00 / 1e-8 = 6.3054e12 tick）与 volume 截断（0.00279 → 0，
    /// 忠实量在 TickFeed.volumes），不是「喂 X 得 X」。
    #[test]
    fn binance_agg_trade_quantizes_ohlc_equal_and_preserves_raw_volume() {
        let cfg = cfg_with_gap(3);
        let json = r#"[{"a":4033643721,"p":"63054.00000000","q":"0.00279000","f":6575313871,"l":6575313871,"T":1786760105614,"m":false,"M":true}]"#;
        let feed = parse_binance_agg_trades(json, "inline", "BTCUSDT", &cfg).unwrap();
        assert_eq!(feed.n_trades(), 1);
        let b = &feed.bars[0];
        let expected_px = (63054.0_f64 / 1e-8).round() as i64;
        assert_eq!(
            (b.open, b.high, b.low, b.close),
            (expected_px, expected_px, expected_px, expected_px)
        );
        assert_eq!(b.timestamp, 1786760105614);
        assert_eq!(b.source_index, 0);
        assert_eq!(
            b.volume, 0,
            "qty=0.00279 的 i64 截断 = 0（对齐 data.rs 口径）"
        );
        assert!(
            !b.untradable,
            "逐笔成交 qty>0 ⟹ tradable（不用截断后 volume 判）"
        );
        assert_eq!(feed.volumes, vec![0.00279]);
    }

    /// L1：时间序还原——降序输入的按 (T,a) 升序输出，source_index 重置为 0..n，时间戳不减。
    #[test]
    fn sorts_descending_input_to_time_order() {
        let cfg = cfg_with_gap(3);
        // 故意降序（最新在前，模拟 Binance 分页拼接）。
        let json = r#"[
            {"a":30,"p":"100.0","q":"1","f":0,"l":0,"T":300,"m":false,"M":true},
            {"a":10,"p":"90.0","q":"1","f":0,"l":0,"T":100,"m":false,"M":true},
            {"a":20,"p":"95.0","q":"1","f":0,"l":0,"T":200,"m":false,"M":true}
        ]"#;
        let feed = parse_binance_agg_trades(json, "inline", "X", &cfg).unwrap();
        let ts: Vec<i64> = feed.bars.iter().map(|b| b.timestamp).collect();
        assert_eq!(ts, vec![100, 200, 300], "按 (T,a) 升序还原时间序");
        for (i, b) in feed.bars.iter().enumerate() {
            assert_eq!(b.source_index, i);
        }
    }

    /// L1：fail-loud——价格缺失 / 非正 / 量非正 / NaN / 空数组 都 Err（不静默）。
    #[test]
    fn fail_loud_on_bad_ticks() {
        let cfg = cfg_with_gap(3);
        let cases = [
            r#"[{"a":1,"q":"1","T":100}]"#,            // 价格缺失
            r#"[{"a":1,"p":"0","q":"1","T":100}]"#,    // 价格 0
            r#"[{"a":1,"p":"-1.5","q":"1","T":100}]"#, // 价格负
            r#"[{"a":1,"p":"100","q":"0","T":100}]"#,  // 量 0
            r#"[{"a":1,"p":"NaN","q":"1","T":100}]"#,  // 价格 NaN
            r#"[]"#,                                   // 空数组
        ];
        for (i, json) in cases.iter().enumerate() {
            assert!(
                parse_binance_agg_trades(json, "x", "S", &cfg).is_err(),
                "case {i} 应 fail-loud"
            );
        }
    }

    /// L1：Binance `trade`（REST /api/v3/trades 字段名 id/price/qty/time）→ Bar。
    #[test]
    fn binance_trade_schema_parses() {
        let cfg = cfg_with_gap(3);
        let json = r#"[{"id":6575317538,"price":"63080.01000000","qty":"0.00003000","quoteQty":"1.89240030","time":1786760404205,"isBuyerMaker":false,"isBestMatch":true}]"#;
        let feed = parse_binance_trades(json, "inline", "BTCUSDT", &cfg).unwrap();
        assert_eq!(feed.n_trades(), 1);
        let b = &feed.bars[0];
        assert_eq!(b.timestamp, 1786760404205);
        assert_eq!(b.volume, 0, "qty=0.00003 截断 = 0");
        assert_eq!(feed.volumes, vec![0.00003]);
    }

    /// L1：Databento `Trades`（ns）→ Bar.timestamp 直接用 ns 值（不塌缩）。
    #[test]
    fn databento_trades_ns_timestamp_preserved() {
        let cfg = cfg_with_gap(3);
        let json = r#"[{"ts_event":1786760404205000000,"price":100.5,"size":2},{"ts_event":1786760404205000001,"price":100.6,"size":1}]"#;
        let feed = parse_databento_trades(json, "inline", "ES", &cfg).unwrap();
        assert_eq!(feed.bars[0].timestamp, 1786760404205000000);
        assert_eq!(feed.bars[1].timestamp, 1786760404205000001);
        assert_eq!(feed.bars[0].volume, 2);
    }

    /// ★seam 测试（#973 验收第一条）：真实 BTC tick 样本 → 分型/笔/线段/中枢，结构非平凡。
    ///
    /// 断言非平凡结构（不是「喂 X 得 X」）：
    /// - loader 产出 20000 根逐笔 bar（fail-loud 计数）；
    /// - 分型 > 0、笔 > 0、线段 > 0；
    /// - 线段数**不是 4**（prototype #972 的塌缩值——重标定 `new_stroke_min_gap` 后须恢复非平凡）；
    /// - 连续三段经 `center_from_segments`（中心判据，零改）至少产出一个真中枢（方向交替 ∧ 核心严格非空）。
    #[test]
    fn seam_btc_tick_bars_yield_non_trivial_structure() {
        let cfg = cfg_with_gap(RECALIBRATED_GAP);
        let feed =
            parse_binance_agg_trades(BTC_FIXTURE, "btc_agg_trades_20000.json", "BTCUSDT", &cfg)
                .expect("20000 笔样本加载失败");
        assert_eq!(feed.n_trades(), 20000);

        let layer = parse_layer(&feed.bars, &cfg);
        let n_fractal = layer.fractals.len();
        let n_stroke = layer.strokes.len();
        let n_segment = layer.segments.len();
        assert!(n_fractal > 0, "分型应非空");
        assert!(n_stroke > 0, "笔应非空");
        assert!(n_segment > 0, "线段应非空");
        assert_ne!(n_segment, 4, "线段数不得塌缩到 prototype 的 4 段");

        // 中枢：连续三段 → center_from_segments（零改判据：方向交替 ∧ ZD<ZG）。
        let units: Vec<_> = layer
            .segments
            .iter()
            .map(crate::theta_v0::classifier::segment_to_unit)
            .collect();
        let mut n_center = 0usize;
        for w in units.windows(3) {
            if crate::theta_v0::classifier::center::center_from_segments(&w[0], &w[1], &w[2])
                .is_some()
            {
                n_center += 1;
            }
        }
        assert!(n_center > 0, "线段层应产出至少一个真中枢");
    }

    /// ★扫参记录（#973 验收第二条）：笔层 gap 扫参 → 笔数/线段数稳定点。
    ///
    /// 扫参表见报告 §2；本测试把「稳定点判据」编码成断言（防回归）：
    /// 1. `RECALIBRATED_GAP` 处线段非平凡（> 4）；
    /// 2. 笔中位尺寸在 `RECALIBRATED_GAP` 处**逃出 $0.01 噪声底**（gap=5 仍噪声主导，gap=10 逃出）。
    /// 用 `--nocapture` 跑本测试可重放完整扫参表。
    #[test]
    fn calibration_sweep_finds_non_collapsed_segments() {
        let feed = parse_binance_agg_trades(
            BTC_FIXTURE,
            "btc_agg_trades_20000.json",
            "BTCUSDT",
            &ThetaConfig::default(),
        )
        .expect("20000 笔样本加载失败");
        let gaps = [3u32, 5, 10, 20, 50, 100];
        // (gap, strokes, segments, 笔中位尺寸$) —— tick_size=1e-8，Tick 差 ×1e-8 = 美元。
        let mut table: Vec<(u32, usize, usize, f64)> = Vec::new();
        for g in gaps {
            let cfg = cfg_with_gap(g);
            let layer = parse_layer(&feed.bars, &cfg);
            let mut sizes: Vec<f64> = layer
                .strokes
                .iter()
                .map(|st| (st.start_price - st.end_price).abs() as f64 * 1e-8)
                .collect();
            sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = if sizes.is_empty() {
                0.0
            } else {
                sizes[sizes.len() / 2]
            };
            table.push((g, layer.strokes.len(), layer.segments.len(), med));
        }
        for (g, ns, nseg, med) in &table {
            println!("gap={g:>3}  strokes={ns:>5}  segments={nseg:>4}  median_stroke=${med:.4}");
        }

        let get = |g: u32| *table.iter().find(|(x, _, _, _)| *x == g).unwrap();
        let (_, _, nseg_cal, med_cal) = get(RECALIBRATED_GAP);
        assert!(
            nseg_cal > 4,
            "RECALIBRATED_GAP={RECALIBRATED_GAP} 处线段数 {nseg_cal} 仍塌缩"
        );
        let (_, _, _, med_5) = get(5);
        assert!(
            med_cal > med_5,
            "gap={RECALIBRATED_GAP} 笔中位尺寸 ${med_cal:.4} 未逃出 gap=5 的 ${med_5:.4} 噪声底"
        );
    }
}
