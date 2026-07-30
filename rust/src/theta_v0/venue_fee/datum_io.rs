//! datum 文件 IO（serde_json + sha2 + fs）——[`venue_fee`](super) 的落盘侧。
//!
//! 从 `venue_fee.rs` 拆出（#374 LOW-D）：宿主文件 804 行越 `coding-style.md` 的 800 行硬上限，
//! 而本模块职责单一（读文件 + sha256 校验 + JSON→[`VenueFeeBook`](super::VenueFeeBook) 构造）
//! 且已 `cfg` 隔离 ⟹ 沿既有接缝切开，逐字搬迁、语义零改。

use super::{FeeUnit, PerShareFees, VenueFeeBook, VenueFeeSchedule};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// datum 落盘目录（与 `backtest::data::data_dir` 同一约定：`analysis/data_cache`）。
pub fn datum_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 的父目录 = 项目根")
        .join("analysis/data_cache")
}

/// datum schema 版本（#62 §4 列式约定）。**只接受 1**——未知版本 fail-loud，
/// 禁按旧字段表静默解析新 schema。
pub const DATUM_SCHEMA_VERSION: u32 = 1;

#[derive(Deserialize)]
struct RawBook {
    schema_version: u32,
    venue: String,
    source_url: String,
    retrieved_at_utc: String,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    symbol: String,
    tier: String,
    unit: String,
    // per-notional 档
    maker_bps: Option<f64>,
    taker_bps: Option<f64>,
    // per-share 档
    commission_per_share_usd: Option<f64>,
    min_commission_usd: Option<f64>,
    max_commission_frac_of_notional: Option<f64>,
    clearing_per_share_usd: Option<f64>,
    cat_per_share_usd: Option<f64>,
    sell_sec_fee_frac: Option<f64>,
    sell_taf_per_share_usd: Option<f64>,
    sell_taf_cap_usd: Option<f64>,
    passthru_exchange_frac_of_commission: Option<f64>,
    passthru_finra_frac_of_commission: Option<f64>,
}

fn need(v: Option<f64>, field: &str, ctx: &str) -> Result<f64, String> {
    let x = v.ok_or_else(|| format!("{ctx}: 缺字段 {field}"))?;
    if !x.is_finite() || x < 0.0 {
        return Err(format!("{ctx}: 字段 {field} 非有限非负（{x}）"));
    }
    Ok(x)
}

/// sha256 小写 hex。
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// 读 datum 文件 + **sha256 版本校验**（sidecar `<path>.sha256`，`shasum -a 256` 格式）。
///
/// fail-loud：文件缺失 / sidecar 缺失 / 哈希不符 / JSON 非法 / 字段缺失或非有限非负 /
/// 未知 `unit` / 条目为空 / (symbol,tier) 重复 ⟹ `Err`。**禁静默退化**（未校验的费率表
/// 进生产 = 口径标签谎报 L2）。
pub fn load_datum(json_path: &Path) -> Result<VenueFeeBook, String> {
    let bytes = std::fs::read(json_path)
        .map_err(|e| format!("venue fee datum 读失败 {}: {e}", json_path.display()))?;
    let actual = sha256_hex(&bytes);

    let sidecar = PathBuf::from(format!("{}.sha256", json_path.display()));
    let sidecar_txt = std::fs::read_to_string(&sidecar).map_err(|e| {
        format!(
            "venue fee datum sha256 sidecar 读失败 {}: {e}",
            sidecar.display()
        )
    })?;
    let expected = sidecar_txt
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("sha256 sidecar 空 {}", sidecar.display()))?
        .to_ascii_lowercase();
    if expected != actual {
        return Err(format!(
            "venue fee datum 哈希不符 {}: sidecar={expected} 实算={actual}（datum 漂移或被改写）",
            json_path.display()
        ));
    }

    let raw: RawBook = serde_json::from_slice(&bytes)
        .map_err(|e| format!("venue fee datum JSON 非法 {}: {e}", json_path.display()))?;
    build_book(raw, actual)
}

fn build_book(raw: RawBook, datum_sha256: String) -> Result<VenueFeeBook, String> {
    if raw.schema_version != DATUM_SCHEMA_VERSION {
        return Err(format!(
            "venue fee datum {}: schema_version={} 不受支持（本实装只认 {}）",
            raw.venue, raw.schema_version, DATUM_SCHEMA_VERSION
        ));
    }
    if raw.entries.is_empty() {
        return Err(format!("venue fee datum {}: entries 空", raw.venue));
    }
    let mut entries: Vec<VenueFeeSchedule> = Vec::with_capacity(raw.entries.len());
    for e in &raw.entries {
        let ctx = format!("venue fee datum {} 条目 {}/{}", raw.venue, e.symbol, e.tier);
        if entries
            .iter()
            .any(|p| p.symbol == e.symbol && p.tier == e.tier)
        {
            return Err(format!("{ctx}: (symbol, tier) 重复"));
        }
        let unit = match e.unit.as_str() {
            "notional" => FeeUnit::Notional {
                maker_bps: need(e.maker_bps, "maker_bps", &ctx)?,
                taker_bps: need(e.taker_bps, "taker_bps", &ctx)?,
            },
            "per_share" => {
                let max_frac = need(
                    e.max_commission_frac_of_notional,
                    "max_commission_frac_of_notional",
                    &ctx,
                )?;
                let taf_cap = need(e.sell_taf_cap_usd, "sell_taf_cap_usd", &ctx)?;
                if max_frac <= 0.0 {
                    return Err(format!("{ctx}: max_commission_frac_of_notional 必须 >0"));
                }
                if taf_cap <= 0.0 {
                    return Err(format!("{ctx}: sell_taf_cap_usd 必须 >0"));
                }
                FeeUnit::PerShare(PerShareFees {
                    commission_per_share_usd: need(
                        e.commission_per_share_usd,
                        "commission_per_share_usd",
                        &ctx,
                    )?,
                    min_commission_usd: need(e.min_commission_usd, "min_commission_usd", &ctx)?,
                    max_commission_frac_of_notional: max_frac,
                    clearing_per_share_usd: need(
                        e.clearing_per_share_usd,
                        "clearing_per_share_usd",
                        &ctx,
                    )?,
                    cat_per_share_usd: need(e.cat_per_share_usd, "cat_per_share_usd", &ctx)?,
                    sell_sec_fee_frac: need(e.sell_sec_fee_frac, "sell_sec_fee_frac", &ctx)?,
                    sell_taf_per_share_usd: need(
                        e.sell_taf_per_share_usd,
                        "sell_taf_per_share_usd",
                        &ctx,
                    )?,
                    sell_taf_cap_usd: taf_cap,
                    passthru_exchange_frac_of_commission: need(
                        e.passthru_exchange_frac_of_commission,
                        "passthru_exchange_frac_of_commission",
                        &ctx,
                    )?,
                    passthru_finra_frac_of_commission: need(
                        e.passthru_finra_frac_of_commission,
                        "passthru_finra_frac_of_commission",
                        &ctx,
                    )?,
                })
            }
            other => return Err(format!("{ctx}: 未知计费单位 unit={other}")),
        };
        entries.push(VenueFeeSchedule {
            venue: raw.venue.clone(),
            symbol: e.symbol.clone(),
            tier: e.tier.clone(),
            unit,
            datum_sha256: datum_sha256.clone(),
        });
    }
    Ok(VenueFeeBook {
        venue: raw.venue,
        source_url: raw.source_url,
        retrieved_at_utc: raw.retrieved_at_utc,
        datum_sha256,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NIST 向量：sha256("abc") / sha256("")——证明哈希实现口径 = 标准 SHA-256
    /// （⟹ sidecar 可用 `shasum -a 256 -c` 独立复核，非自造校验和）。
    #[test]
    fn sha256_matches_nist_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // ── #374 LOW-G：`build_book` 的 4 条 Err 分支固化（防篡改链的 fail-loud 语义）──
    //
    // 已由别处覆盖的 3 条走文件层（哈希不符 / 缺 sidecar / 未知 schema_version，见 `venue_fee`
    // 模块测试）；本组走**已读入的 RawBook**，直击 `build_book` 内的构造期校验，
    // 与哈希无关 ⟹ 不需要落临时文件。**认识论 L0**（纯解析器契约，零市场信息）。

    /// 合法 per-notional 底板 JSON；各用例只改动其中一处，其余保持合法 ⟹ 报错必来自被改处。
    fn base_json(entries: &str) -> String {
        format!(
            r#"{{"schema_version":1,"venue":"SYNTH","source_url":"https://example.invalid",
               "retrieved_at_utc":"2026-07-27T00:00:00Z","entries":[{entries}]}}"#
        )
    }

    fn parse(json: &str) -> Result<VenueFeeBook, String> {
        let raw: RawBook = serde_json::from_str(json).expect("底板 JSON 本身须合法");
        build_book(raw, "0".repeat(64))
    }

    const OK_ENTRY: &str =
        r#"{"symbol":"X","tier":"T","unit":"notional","maker_bps":1.0,"taker_bps":2.0}"#;

    /// 底板本身可解析（否则下面 4 条的 Err 可能来自底板而非被测分支）。
    #[test]
    fn base_datum_parses() {
        let book = parse(&base_json(OK_ENTRY)).expect("底板须解析成功");
        assert_eq!(book.entries.len(), 1);
    }

    /// ① 未知 `unit`：禁按已知字段表猜测新计费单位。
    #[test]
    fn unknown_unit_is_rejected() {
        let e =
            r#"{"symbol":"X","tier":"T","unit":"per_contract","maker_bps":1.0,"taker_bps":2.0}"#;
        let err = parse(&base_json(e)).expect_err("未知 unit 须 Err");
        assert!(err.contains("未知计费单位"), "错误须点名 unit：{err}");
    }

    /// ② `(symbol, tier)` 重复：`resolve` 取首条 ⟹ 重复条目会让"用哪份费率"取决于文件顺序。
    #[test]
    fn duplicate_symbol_tier_is_rejected() {
        let err = parse(&base_json(&format!("{OK_ENTRY},{OK_ENTRY}"))).expect_err("重复须 Err");
        assert!(err.contains("重复"), "错误须点名重复：{err}");
    }

    /// ③ `entries` 空：空簿会让 `resolve` 全部落空 ⟹ 静默退回未标定档（口径谎报）。
    #[test]
    fn empty_entries_is_rejected() {
        let err = parse(&base_json("")).expect_err("空 entries 须 Err");
        assert!(err.contains("entries 空"), "错误须点名空簿：{err}");
    }

    /// ④ `need()` 两支：缺字段 / 负值。**非有限值不从 JSON 侧可达**——serde_json 拒收
    /// `Infinity`/`NaN` 字面量与超范围数值（下面第三段断言即固化这一前提），故 `need()` 的
    /// `is_finite` 分支是对**非 JSON 构造路径**的兜底，照实登记，不假装被本测试覆盖。
    #[test]
    fn missing_or_negative_field_is_rejected() {
        let missing = r#"{"symbol":"X","tier":"T","unit":"notional","maker_bps":1.0}"#;
        let err = parse(&base_json(missing)).expect_err("缺 taker_bps 须 Err");
        assert!(
            err.contains("缺字段 taker_bps"),
            "错误须点名缺失字段：{err}"
        );

        let negative =
            r#"{"symbol":"X","tier":"T","unit":"notional","maker_bps":1.0,"taker_bps":-2.0}"#;
        let err = parse(&base_json(negative)).expect_err("负费率须 Err");
        assert!(err.contains("非有限非负"), "错误须点名非法取值：{err}");

        let nonfinite =
            r#"{"symbol":"X","tier":"T","unit":"notional","maker_bps":1.0,"taker_bps":1e400}"#;
        assert!(
            serde_json::from_str::<RawBook>(&base_json(nonfinite)).is_err(),
            "超 f64 范围的数值须在 JSON 解析层即被拒（非有限值不进 build_book）"
        );
    }
}
