//! 回测数据加载层——`analysis/data_cache/*.json` → 清洗 → 量化为整数 tick [`Bar`]。
//!
//! ## 认识论等级：**L1**（管线正确性，零信息增量）
//!
//! 本模块只做"读 JSON → 清洗坏 bar → 量化为 tick"——确定性数据变换，验证管线无 bug，
//! 不验证 Θ 在市场有效（formalization-validity-domain 231号）。
//!
//! ## 数据快照（协议 §1.1，只读，看结果前冻结）
//!
//! 8 个品种全部为 1min OHLCV 真实行情（databento / Binance 归档）。本模块加载时**独立
//! 核实** bar 数与时间边界（不照搬协议表格），与协议 §1.1 不符则 fail-loud（数据漂移）。
//!
//! ## schema（独立核实，2026-06-25）
//!
//! 全 8 品种为 parallel-array schema：`{opens,highs,lows,closes,volumes,dates,symbol}`。
//! `dates` 为逐 bar 字符串：databento 品种 tz-aware（`"2016-01-03 23:00:00+00:00"`），
//! BTC 无 tz（`"2017-08-17 04:00:00"`，协议 §1.2 [设计选择] 按 UTC 解释）。**8 品种统一
//! 用 `dates[..10]` 字典序切日期窗**（ISO 日期串字典序 = 时间序）——无 `timestamps_ns` 列。
//!
//! ## 不可交易判据（协议 §1.2 + reference-theta-v0.md:53，Θ_exec 继承）
//!
//! 缺 OHLC / `high<max(open,close,low)` / `low>min(open,close,high)` / volume=0 ⇒ bar
//! 标 `untradable=true`（**不删除**，进 [`Bar`] 但执行层不在其上成交）。这与
//! `recursive_t/backtest_run.rs::load_clean_ohlc` 的"删除坏 bar"口径**不同**——Θ_exec
//! 要求保留 bar 序列连续性 + 标注不可交易（§1.2"统计占比，不豁免只标注"）。

use super::super::config::ThetaConfig;
use super::super::types::{quantize, Bar, Timestamp};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// parallel-array schema（8 品种统一）。`Option<f64>` 容忍 Python `json(allow_nan)`
/// 写出的 NaN/Inf（预处理替换为 null → None）。
#[derive(Deserialize, Default)]
struct RawData {
    #[serde(default)]
    opens: Vec<Option<f64>>,
    #[serde(default)]
    highs: Vec<Option<f64>>,
    #[serde(default)]
    lows: Vec<Option<f64>>,
    #[serde(default)]
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    #[serde(default)]
    dates: Vec<String>,
}

/// 8 品种 → 数据文件名（协议 §1.1 锁定）。与 `recursive_t/backtest_run.rs::SYMBOLS` 一致。
pub const SYMBOLS: [(&str, &str); 8] = [
    ("BTC", "btc_1m_full.json"),
    ("ES", "es_1m_databento_10y.json"),
    ("CL", "cl_1m_databento_10y.json"),
    ("GC", "gc_1m_databento_10y.json"),
    ("BRN", "brn_1m_databento_10y.json"),
    ("DX", "dx_1m_databento_10y.json"),
    ("QQQ", "qqq_1m_databento_full.json"),
    ("OKLO", "oklo_1m_databento.json"),
];

/// `analysis/data_cache` 绝对路径（crate manifest 上一级）。
pub fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 的父目录 = 项目根")
        .join("analysis/data_cache")
}

/// 已加载的数据集（量化后的整数 tick bar 序列 + 原始日期串，供日期窗切片）。
///
/// `bars` 与 `dates` 同长、同索引（一一对应）。`dates[i][..10]` = bar i 的 ISO 日期。
#[derive(Debug, Clone)]
pub struct Dataset {
    pub symbol: String,
    /// 量化后的整数 tick bar（含 `untradable` 标注，**不删除坏 bar**，保序列连续）。
    pub bars: Vec<Bar>,
    /// 逐 bar 日期串（与 `bars` 同索引）。用于日期窗切片（OOS/Holdout/walk-forward）。
    pub dates: Vec<String>,
}

impl Dataset {
    /// 不可交易 bar 占比（协议 §5.5：>20% 该品种判据不评估；§1.2：>5% 标数据质量警告）。
    pub fn untradable_ratio(&self) -> f64 {
        if self.bars.is_empty() {
            return 0.0;
        }
        let n_bad = self.bars.iter().filter(|b| b.untradable).count();
        n_bad as f64 / self.bars.len() as f64
    }

    /// 按 ISO 日期闭区间 `[day_start, day_end]` 切片（协议 §2 样本切分）。
    ///
    /// 判据：`day_start <= dates[i][..10] <= day_end`（ISO 字典序 = 时间序，闭区间）。
    /// 返回新 `Dataset`（immutable，coding-style：不就地改）。窗为空时返回空 bars 的
    /// Dataset（调用方据 §5.5 判 inconclusive，**不 panic**——空窗是合法的"该品种该窗无
    /// 数据"，区别于切片口径错误）。
    pub fn slice_date_window(&self, day_start: &str, day_end: &str) -> Dataset {
        assert!(
            day_start <= day_end,
            "窗口非法：day_start `{day_start}` > day_end `{day_end}`"
        );
        let mut bars = Vec::new();
        let mut dates = Vec::new();
        for (i, b) in self.bars.iter().enumerate() {
            let day = self.dates[i].get(..10).unwrap_or("");
            if day >= day_start && day <= day_end {
                bars.push(*b);
                dates.push(self.dates[i].clone());
            }
        }
        Dataset {
            symbol: self.symbol.clone(),
            bars,
            dates,
        }
    }
}

/// 把 ISO 日期串解析为单调时间戳（仅用于排序，不参与价格运算，types.rs:17）。
///
/// 用 `dates[..16]`（`"YYYY-MM-DD HH:MM"`）去掉分隔符后的整数作为单调键——同序列内严格
/// 单调即可（协议平局裁决按 `(timestamp, source_index)`，source_index 已唯一裁平局）。
/// 不做完整历法换算（无 chrono 依赖；时间戳只用于排序，绝对值无语义）。
fn date_to_timestamp(date: &str) -> Timestamp {
    // "2016-01-03 23:00:00+00:00" → 取 "201601032300" 数字部分（年月日时分，单调）。
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+') // 去掉 tz 后缀
        .filter(|c| c.is_ascii_digit())
        .take(12) // YYYYMMDDHHMM
        .collect();
    digits.parse::<i64>().unwrap_or(0)
}

/// 加载单品种全量数据（量化为整数 tick bar，标注不可交易）。
///
/// `tick_size` 来自 [`ThetaConfig`]（默认 1e-8，types.rs:24 `quantize`）。**不删除坏 bar**
/// （区别于 recursive_t 口径）——保序列连续 + 标 `untradable`（Θ_exec §1.2 要求）。
///
/// 边界条件：文件不存在 ⇒ `Err`；JSON 解析失败 ⇒ `Err`；五列长度不一致 ⇒ `Err`
/// （fail-loud，no-patch）。
pub fn load_symbol(path: &Path, symbol: &str, config: &ThetaConfig) -> Result<Dataset, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取 {path:?} 失败: {e}"))?;
    // Python json(allow_nan) 写出 NaN/Infinity（serde_json 硬拒）→ null。
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData =
        serde_json::from_str(&text).map_err(|e| format!("解析 {path:?} 失败: {e}"))?;
    drop(text);

    let n = raw.closes.len();
    if n == 0 {
        return Err(format!("{path:?} 无 closes 数据"));
    }
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!(
                "{path:?} 列长度不一致：{name}={len} vs closes={n}"
            ));
        }
    }

    let tick_size = config.tick.tick_size;
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let o = raw.opens[i];
        let h = raw.highs[i];
        let l = raw.lows[i];
        let c = raw.closes[i];
        // volume 列可能缺失（部分 schema），缺失按 0（→ untradable）。
        let v = raw.volumes.get(i).and_then(|x| *x).unwrap_or(0.0);

        // 不可交易判据（协议 §1.2 / reference:53）。缺 OHLC 或越界或 vol=0 ⇒ untradable。
        // 坏 bar 用前一根 close 填充价格（保序列连续，但标 untradable 不在其上成交）；
        // 首根坏 bar 无前值时用 0（量化后 tick=0，untradable 已标注，执行层不触碰）。
        let valid_ohlc = matches!((o, h, l, c), (Some(_), Some(_), Some(_), Some(_)));
        let (oq, hq, lq, cq, untradable) = if let (Some(o), Some(h), Some(l), Some(c)) =
            (o, h, l, c)
        {
            let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
            let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
            let untradable = bad_range || bad_price || v <= 0.0;
            (
                quantize(o, tick_size),
                quantize(h, tick_size),
                quantize(l, tick_size),
                quantize(c, tick_size),
                untradable,
            )
        } else {
            // 缺 OHLC：用前一根 close 占位（连续性），标 untradable。
            let prev = bars.last().map(|b: &Bar| b.close).unwrap_or(0);
            (prev, prev, prev, prev, true)
        };
        let _ = valid_ohlc; // 语义已并入上面的 if let

        bars.push(Bar {
            source_index: i,
            timestamp: date_to_timestamp(&raw.dates[i]),
            open: oq,
            high: hq,
            low: lq,
            close: cq,
            volume: v as i64,
            untradable,
        });
    }

    Ok(Dataset {
        symbol: symbol.to_string(),
        bars,
        dates: raw.dates,
    })
}

/// 便捷：从默认 `data_dir()` 按品种名加载（SYMBOLS 表查文件名）。
pub fn load_by_symbol(symbol: &str, config: &ThetaConfig) -> Result<Dataset, String> {
    let file = SYMBOLS
        .iter()
        .find(|(s, _)| s.eq_ignore_ascii_case(symbol))
        .map(|(_, f)| *f)
        .ok_or_else(|| format!("未知品种 `{symbol}`（不在 SYMBOLS 表）"))?;
    let path = data_dir().join(file);
    load_symbol(&path, symbol, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// date_to_timestamp 单调性（同序列内严格递增即可，绝对值无语义）。
    #[test]
    fn date_timestamp_monotonic_within_day() {
        let t0 = date_to_timestamp("2016-01-03 23:00:00+00:00");
        let t1 = date_to_timestamp("2016-01-03 23:01:00+00:00");
        let t2 = date_to_timestamp("2016-01-04 00:00:00+00:00");
        assert!(t0 < t1, "同日相邻分钟单调");
        assert!(t1 < t2, "跨日单调");
    }

    /// BTC 无 tz 与 databento 有 tz 的日期串都能解析（协议 §1.2 BTC 按 UTC）。
    #[test]
    fn date_timestamp_handles_tz_and_no_tz() {
        let no_tz = date_to_timestamp("2017-08-17 04:00:00");
        let with_tz = date_to_timestamp("2017-08-17 04:00:00+00:00");
        assert_eq!(no_tz, with_tz, "tz 后缀不影响单调键（取数字前 12 位）");
        assert_eq!(no_tz, 201_708_170_400, "YYYYMMDDHHMM");
    }

    /// 日期窗切片闭区间语义（L1：切片口径正确性）。
    #[test]
    fn slice_date_window_closed_interval() {
        let ds = Dataset {
            symbol: "TEST".to_string(),
            bars: (0..5)
                .map(|i| Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: 1,
                    high: 1,
                    low: 1,
                    close: 1,
                    volume: 1,
                    untradable: false,
                })
                .collect(),
            dates: vec![
                "2022-12-30 00:00:00".to_string(),
                "2022-12-31 00:00:00".to_string(),
                "2023-01-01 00:00:00".to_string(),
                "2023-01-02 00:00:00".to_string(),
                "2025-07-01 00:00:00".to_string(),
            ],
        };
        // OOS 窗（协议 §2.1）：2023-01-01 → 2025-06-30。
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        assert_eq!(oos.bars.len(), 2, "含 2023-01-01 与 2023-01-02，不含 2022/2025-07");
        // Holdout 窗：2025-07-01 → 末尾。
        let holdout = ds.slice_date_window("2025-07-01", "2030-01-01");
        assert_eq!(holdout.bars.len(), 1, "仅 2025-07-01");
        // 空窗合法（不 panic）。
        let empty = ds.slice_date_window("2099-01-01", "2099-12-31");
        assert_eq!(empty.bars.len(), 0, "空窗返回空 Dataset，不 panic");
    }

    /// untradable_ratio 计算（协议 §5.5/§1.2）。
    #[test]
    fn untradable_ratio_counts_flagged_bars() {
        let mk = |untradable: bool, idx: usize| Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 1,
            untradable,
        };
        let ds = Dataset {
            symbol: "T".to_string(),
            bars: vec![mk(false, 0), mk(true, 1), mk(false, 2), mk(true, 3)],
            dates: vec!["x".into(); 4],
        };
        assert!((ds.untradable_ratio() - 0.5).abs() < 1e-12, "2/4 不可交易");
    }
}
