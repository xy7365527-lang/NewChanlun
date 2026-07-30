//! ⑥ 真实 `BacktestEngine` 驱动 S_Θ 跑 BTC（goal acceptance[5] L2：真引擎产非空订单流）。
//!
//! feature `nautilus` 门控。把 [`super::theta_strategy::ThetaStrategy`] 装进真实
//! `BacktestEngine`（低层命令式：add_venue → add_instrument → add_strategy → add_data → run），
//! 喂 S_Θ dataset 的 BTC bar，返回 `BacktestResult`（`total_orders>0` = L2 非空订单流验收）。
//!
//! ## 与 `run_theta_v0_pi`（in-crate fill 模拟）的关系
//!
//! `run_theta_v0_pi` 是 in-crate 回测（自模拟撮合，bit-exact π_Θ 七链）。本函数是**真实 nautilus
//! 引擎**路径：撮合/账户/持仓全交 nautilus venue，证明「回测实盘同引擎」（设计 §4.1）。两者并存——
//! in-crate 路径产 S_Θ 指标（strat_return 等），nautilus 路径证生产引擎贯通（订单流非空）。
//!
//! ## ★硬约束（task#8 breaking #5）
//!
//! - `add_data(validate=true)` 要求 bar_type.aggregation_source()==External 且**先** add_instrument。
//! - bar 包成 `Data::Bar`，OHLC/volume 经 dequantize（tick→f64）建真实 `Price`/`Quantity`。

use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
    result::BacktestResult,
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{Bar, BarSpecification, BarType, Data},
    enums::{AccountType, AggregationSource, BarAggregation, BookType, OmsType, PriceType},
    identifiers::{InstrumentId, Symbol, Venue},
    instruments::{CurrencyPair, Instrument, InstrumentAny},
    types::{Currency, Money, Price, Quantity},
};
use rust_decimal_macros::dec;

use crate::theta_v0::backtest::data::Dataset;
use crate::theta_v0::config::ThetaConfig;

use super::theta_strategy::ThetaStrategy;

/// BTC instrument（BTCUSDT.BINANCE，镜像 nautilus stub `currency_pair_btcusdt` 字段值）。
///
/// ★诚实：不启用 model 的 `stubs` feature（需 rstest 测试依赖），inline 构造同值 CurrencyPair。
/// price_precision=2, size_precision=6, tick_size=0.01（与 BTC 真实最小变动一致）。
fn btc_instrument() -> InstrumentAny {
    let pair = CurrencyPair::new(
        InstrumentId::from("BTCUSDT.BINANCE"),
        Symbol::from("BTCUSDT"),
        Currency::from("BTC"),
        Currency::from("USDT"),
        2,                            // price_precision
        6,                            // size_precision
        Price::from("0.01"),          // price_increment
        Quantity::from("0.000001"),   // size_increment
        None,                         // lot_size
        None,                         // max_quantity
        Some(Quantity::from("9000")), // max ... (mirror stub)
        Some(Quantity::from("0.000001")),
        None,
        None,
        Some(Price::from("1000000")),
        Some(Price::from("0.01")),
        Some(dec!(0.001)),
        Some(dec!(0.001)),
        Some(dec!(0.001)),
        Some(dec!(0.001)),
        None,
        None,
        UnixNanos::default(),
        UnixNanos::default(),
    );
    InstrumentAny::CurrencyPair(pair)
}

/// 构造 BTC 的 External 1-Last bar_type（add_data validate=true 要求 External）。
fn btc_bar_type(instrument_id: InstrumentId, step: usize, aggregation: BarAggregation) -> BarType {
    let spec = BarSpecification::new(step, aggregation, PriceType::Last);
    BarType::new(instrument_id, spec, AggregationSource::External)
}

/// dataset.bar_seconds → BarAggregation + step（1min→(Minute,1), 1s→(Second,1), 日线→(Day,1)）。
fn aggregation_for(bar_seconds: u32) -> (usize, BarAggregation) {
    match bar_seconds {
        1 => (1, BarAggregation::Second),
        60 => (1, BarAggregation::Minute),
        86_400 => (1, BarAggregation::Day),
        // 其它粒度：按秒聚合（step=bar_seconds），覆盖任意自定义粒度。
        other => (other as usize, BarAggregation::Second),
    }
}

/// S_Θ `types::Bar`（整数 tick）→ 真实 nautilus `Bar`（dequantize 到 f64 Price）。
///
/// price_precision=2（BTCUSDT）⟹ Price::new 按 2 位精度。volume：S_Θ i64 → Quantity（6 位精度）。
/// ts：S_Θ timestamp（ns 或排序键）→ UnixNanos。
fn to_nautilus_bar(bar: &crate::theta_v0::types::Bar, bar_type: BarType, tick_size: f64) -> Bar {
    let px = |tick: i64| Price::new(tick as f64 * tick_size, 2);
    let ts = UnixNanos::from(bar.timestamp.max(0) as u64);
    Bar::new(
        bar_type,
        px(bar.open),
        px(bar.high),
        px(bar.low),
        px(bar.close),
        Quantity::new(bar.volume.max(0) as f64, 6),
        ts,
        ts,
    )
}

/// #343 品种门控：非 BTC 品种 fail-fast，不静默套用 `btc_instrument()`。
///
/// 纯函数（不摸 `BacktestEngine`）——独立可测，不需要真实引擎跑一圈才能验证门控是否生效。
///
/// `pub`（#344 LOW-3）：CLI（`bin/theta_backtest.rs`）在 ⑤ 段（in-crate 回测）起跑前复用
/// 同一门控 fail-fast，避免非 BTC 品种白跑一遍全量回测才在 ⑥ 段发现不支持。
pub fn require_btc_symbol(symbol: &str) -> anyhow::Result<()> {
    if symbol.eq_ignore_ascii_case("BTC") {
        return Ok(());
    }
    anyhow::bail!(
        "NT 段（真实 nautilus BacktestEngine）当前仅支持 BTC：instrument 硬编码为 \
         BTCUSDT.BINANCE（CurrencyPair，加密货币专属结构），品种 `{symbol}` 未接入 —— \
         ES/CL/GC/BRN/DX 需 FuturesContract instrument，QQQ/OKLO 需 Equity instrument，\
         均未实装。门控 fail-fast（#343），避免默默套用 BTC instrument 产出 ID 标签失真的\
         回测结果（价格值域正确但 instrument 身份错误）。"
    );
}

/// 用真实 `BacktestEngine` 跑 dataset，返回 `BacktestResult`。
///
/// 流程（task#8 ⑥）：BacktestEngine::new → add_venue(Sim) → add_instrument(BTC) →
/// add_strategy(ThetaStrategy) → add_data(Data::Bar, validate=true) → run → get_result。
///
/// ★`theta` 须 `entry_delay_bars=0`（#7：流式逐 bar 退出在末根触发，delay=1 丢单；延迟归 venue）。
/// 调用方（CLI）负责设 0（本函数不静默改 config，保留调用方 Param 主权）。
///
/// ★门控（#343，090 声明=能力）：instrument 硬编码 `btc_instrument()`（`CurrencyPair`
/// BTCUSDT.BINANCE），对非 BTC 品种（ES/CL/GC/BRN/DX 应为 `FuturesContract`，QQQ/OKLO
/// 应为 `Equity`）套用会造成 instrument **身份**失真（不只是 ID 字符串——资产类/币种/
/// 精度全错），非本轮范围内可安全推广。`dataset.symbol` 非 BTC 时 fail-fast（见
/// [`require_btc_symbol`]），不静默借用 BTC instrument 跑一遍产出误导性输出
/// （价格值域正确但持仓/订单归属的品种是假的）。
pub fn run_theta_backtest(
    dataset: &Dataset,
    theta: &ThetaConfig,
) -> anyhow::Result<BacktestResult> {
    require_btc_symbol(&dataset.symbol)?;

    let instrument = btc_instrument();
    let instrument_id = instrument.id();
    let (step, aggregation) = aggregation_for(dataset.bar_seconds);
    let bar_type = btc_bar_type(instrument_id, step, aggregation);

    // 起始余额：与 in-crate CLI 的 initial_nav 量级一致（USDT 计价）。
    let mut engine = BacktestEngine::new(BacktestEngineConfig::default())?;
    engine.add_venue(
        SimulatedVenueConfig::builder()
            .venue(Venue::from("BINANCE"))
            .oms_type(OmsType::Netting)
            .account_type(AccountType::Cash)
            .book_type(BookType::L1_MBP)
            .starting_balances(vec![Money::from("1_000_000 USDT")])
            .build()?,
    )?;

    // ★顺序硬约束：先 add_instrument（add_data validate 校验 instrument 已存在）。
    engine.add_instrument(&instrument)?;

    // ThetaStrategy（StrategyConfig 默认 + S_Θ theta + bar_type + size_precision=6）。
    let strat_config = nautilus_trading::strategy::StrategyConfig::default();
    let strategy = ThetaStrategy::new(strat_config, theta.clone(), bar_type, 6);
    engine.add_strategy(strategy)?;

    // dataset bar → Data::Bar（dequantize）。
    let data: Vec<Data> = dataset
        .bars
        .iter()
        .map(|b| Data::Bar(to_nautilus_bar(b, bar_type, theta.tick.tick_size)))
        .collect();
    // add_data(data, client_id, validate=true, sort=true)：External + 已 add_instrument ⟹ 通过。
    engine.add_data(data, None, true, true)?;

    // run(start, end, run_config_id, streaming=false)。
    engine.run(None, None, None, false)?;
    Ok(engine.get_result())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_btc_symbol_gate_names_symbol_and_declares_btc_only() {
        let err = require_btc_symbol("OKLO").expect_err("OKLO 未接入，须 fail-fast");
        let msg = err.to_string();
        // ★#344 MED-1 修复：断言反引号包裹的精确插值形态 `OKLO`，而非裸子串 "OKLO"——
        // 后者即使 `{symbol}` 插值丢失也会命中错误文本里静态列举的 "QQQ/OKLO 需 Equity
        // instrument"，测试空转（照过不代表插值生效）。
        assert!(
            msg.contains("`OKLO`"),
            "错误须精确点名未接入品种（反引号包裹的插值形态），实际: {msg}"
        );
        assert!(
            msg.contains("BTC"),
            "错误须声明当前能力边界（仅支持 BTC），实际: {msg}"
        );
    }

    #[test]
    fn btc_symbol_case_insensitive_passes_gate() {
        require_btc_symbol("BTC").expect("BTC 须通过门控");
        require_btc_symbol("btc").expect("btc（小写）须通过门控");
        require_btc_symbol("Btc").expect("Btc（混合大小写）须通过门控");
    }

    #[test]
    fn every_catalogued_non_btc_symbol_is_gated() {
        // ★#344 LOW-2 修复：随 SYMBOLS 表迭代（同 crate 可见），而非硬编码 7 品种数组——
        // 品种表扩容时此测试自动延伸覆盖，不再需要手动同步维护列表。
        for (sym, _, _) in crate::theta_v0::backtest::data::SYMBOLS.iter() {
            if sym.eq_ignore_ascii_case("BTC") {
                continue;
            }
            let err = require_btc_symbol(sym).expect_err("非 BTC 品种须 fail-fast");
            assert!(
                err.to_string().contains(&format!("`{sym}`")),
                "错误须精确点名品种 `{sym}`（反引号包裹的插值形态，非静态列举误报），实际: {err}"
            );
        }
    }

    #[test]
    fn uncatalogued_symbol_is_named_via_interpolation_not_static_enumeration() {
        // FOO 不出现在错误文本的静态品种列举（ES/CL/GC/BRN/DX/QQQ/OKLO）中——只有
        // `{symbol}` 插值真正生效时此断言才会通过，是 #344 MED-1 空转的直接反测。
        let err = require_btc_symbol("FOO").expect_err("未接入品种须 fail-fast");
        assert!(
            err.to_string().contains("`FOO`"),
            "错误须通过 {{symbol}} 插值点名，非静态列举，实际: {err}"
        );
    }
}
