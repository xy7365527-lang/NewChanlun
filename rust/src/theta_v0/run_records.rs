//! #1602（#1586 审计 C-06/C-08/C-17/C-18，ADR 0041）：场所 profile、Θ_risk 与数据 profile 的版本化记录。
//!
//! 四份记录随源码入库并编进二进制（`include_str!`），格式与 [`super::policy_records`]（#1603）相同：
//!
//! - `venue-profile-P0`：ADR 0057 的 `replay_binance_spot_1m`（`P0` 为其 profile_id 别名）：P0 确定性回放
//!   场所（ADR 0028 P0 槽位、ADR 0044 P0 签署记录）的 `tick_size` 与四项执行成本
//!   （`commission_bps`/`slippage_bps`/`tax_bps`/`entry_delay_bars`）。生产与验收回放按品种或 profile id
//!   从 [`PRODUCTION_VENUE_PROFILES`] 取；P2 未启用（ADR 0028「P2 处置」），表里没有 P2。
//! - `venue-profile-synthetic-tb02a`：ADR 0057 的 `production_synthetic_ohlc_integer_tb02a_v1`，#1640 S 结构
//!   写者的整数 tick 输入约定（`not_product_default`）。记录头 `symbol: null`、`usage: "s_structure_writer"`
//!   （#1774 B1+B2）：symbol 路选不到；通用 profile 选择器拒绝；只有 [`production_venue_for_s_writer`] 绑定。
//! - `venue-profile-diagnostic-fixture`：测试夹具与诊断运行用，数值等于改前 `impl Default` 常数，
//!   `admissible: false`——用它跑出的输出不参与准入。
//! - `theta-risk`：Θ_risk 与声部参数（rho/beta/gamma/kappa/default_lot/chi_z_alpha/两个 cap 开关、
//!   `max_depth`/`w_grade`）。`depth_weights` 属 C-07（审计判 E），不在本记录。
//! - `data-profile`：#922 坏 tick 闸阈值 `bad_tick_max_dev`。
//!
//! ## 缺值行为
//!
//! 场所 profile 缺 `tick_size` ⟹ 槽位 `venue_profile`；缺任一成本项 ⟹ 槽位 `cost_inputs`；品种在生产表
//! 里没有 profile ⟹ 两个槽位都报。其余记录各占一个槽位。所有缺口一次收齐到 [`RunPolicyError`]，
//! 调用方停止、不产读数；本模块**没有**备用值，也不回退旧常数。

use serde_json::Value;

use super::policy_records::{
    self as pr, PolicyError, PolicyRef, ACTIVE_MACD_FORCE_JSON, ACTIVE_TOWER_LEVEL_BUDGET_JSON,
};

/// P0 场所 profile 记录 id（ADR 0057 `replay_binance_spot_1m`，profile_id 别名 `P0`）。
pub const VENUE_PROFILE_P0_ID: &str = "venue-profile-P0";
/// S 结构写者 synthetic 场所 profile 记录 id（ADR 0057 `production_synthetic_ohlc_integer_tb02a_v1`）。
pub const VENUE_PROFILE_SYNTHETIC_TB02A_ID: &str = "venue-profile-synthetic-tb02a";
/// synthetic 记录头 `profile_id`；S 生产入口以此绑定（ADR 0057 受影响代码清单 driver.rs 行）。
pub const SYNTHETIC_TB02A_PROFILE_ID: &str = "production_synthetic_ohlc_integer_tb02a_v1";
/// 诊断夹具场所 profile 记录 id（不参与准入）。
pub const VENUE_PROFILE_DIAGNOSTIC_FIXTURE_ID: &str = "venue-profile-diagnostic-fixture";
/// 品种在生产表里找不到场所 profile 时报错所用的 id。
pub const VENUE_PROFILE_ID: &str = "venue-profile";
/// Θ_risk 记录 id（C-08）。
pub const THETA_RISK_ID: &str = "theta-risk";
/// 数据 profile 记录 id（C-18）。
pub const DATA_PROFILE_ID: &str = "data-profile";

/// P0 场所 profile（编进二进制）。
pub const VENUE_PROFILE_P0_JSON: &str = include_str!("policy_records/venue-profile-P0.json");
/// S 结构写者 synthetic 场所 profile（编进二进制）。
pub const VENUE_PROFILE_SYNTHETIC_TB02A_JSON: &str =
    include_str!("policy_records/venue-profile-synthetic-tb02a.json");
/// 诊断夹具场所 profile（编进二进制）。
pub const VENUE_PROFILE_DIAGNOSTIC_FIXTURE_JSON: &str =
    include_str!("policy_records/venue-profile-diagnostic-fixture.json");
/// 当前生效的 Θ_risk 记录（编进二进制）。
pub const ACTIVE_THETA_RISK_JSON: &str = include_str!("policy_records/theta-risk.json");
/// 当前生效的数据 profile 记录（编进二进制）。
pub const ACTIVE_DATA_PROFILE_JSON: &str = include_str!("policy_records/data-profile.json");

/// 生产场所 profile 表：`(记录 id, 记录文本)`。按记录头 `symbol` / `profile_id` 匹配，再按记录头
/// `usage` 与入口核对（[`VenueUsage`]）；一次运行只绑定其中一份（ADR 0057 裁定三）。
pub const PRODUCTION_VENUE_PROFILES: &[(&str, &str)] = &[
    (VENUE_PROFILE_P0_ID, VENUE_PROFILE_P0_JSON),
    (
        VENUE_PROFILE_SYNTHETIC_TB02A_ID,
        VENUE_PROFILE_SYNTHETIC_TB02A_JSON,
    ),
];

/// 槽位名：场所 profile（`tick_size`、记录本身）。
pub const SLOT_VENUE_PROFILE: &str = "venue_profile";
/// 槽位名：四项执行成本。
pub const SLOT_COST_INPUTS: &str = "cost_inputs";
/// 槽位名：Θ_risk 记录。
pub const SLOT_THETA_RISK: &str = "theta_risk";
/// 槽位名：数据 profile 记录。
pub const SLOT_DATA_PROFILE: &str = "data_profile";
/// 槽位名：级别塔资源预算记录（#1603）。
pub const SLOT_TOWER_LEVEL_BUDGET: &str = "tower_level_budget";
/// 槽位名：塔底构造记录（#1626）。
pub const SLOT_TOWER_BASE: &str = "tower_base";
/// 槽位名：MACD 力度记录（#1603）。
pub const SLOT_MACD_FORCE: &str = "macd_force";
/// 槽位名：特征序列扫描预算与线段构造规则记录（#1604）。
pub const SLOT_PARSE_RECORDS: &str = "parse_records";

/// 组装运行配置时收齐的全部缺口。调用方停止，不换值重试。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunPolicyError {
    /// 缺口所在槽位（去重、保序）。
    pub slots: Vec<&'static str>,
    /// 逐条原因。
    pub causes: Vec<PolicyError>,
}

impl RunPolicyError {
    fn add(&mut self, slots: &[&'static str], cause: PolicyError) {
        for s in slots {
            if !self.slots.contains(s) {
                self.slots.push(s);
            }
        }
        self.causes.push(cause);
    }

    fn take<T>(&mut self, slot: &'static str, r: Result<T, PolicyError>) -> Option<T> {
        r.map_err(|e| self.add(&[slot], e)).ok()
    }

    pub fn is_empty(&self) -> bool {
        self.causes.is_empty()
    }

    /// 全部原因都是缺记录/未签（无非法值）。
    pub fn is_awaiting_policy(&self) -> bool {
        !self.is_empty()
            && self
                .causes
                .iter()
                .all(|c| !matches!(c, PolicyError::Invalid { .. }))
    }

    pub(crate) fn merge(&mut self, other: RunPolicyError) {
        for s in other.slots {
            if !self.slots.contains(&s) {
                self.slots.push(s);
            }
        }
        self.causes.extend(other.causes);
    }
}

impl std::fmt::Display for RunPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = if self.is_awaiting_policy() {
            "AwaitingPolicy"
        } else {
            "InvalidPolicy"
        };
        write!(f, "{kind}{{{}}}：", self.slots.join(", "))?;
        let causes: Vec<String> = self.causes.iter().map(ToString::to_string).collect();
        f.write_str(&causes.join("；"))
    }
}

impl std::error::Error for RunPolicyError {}

/// 记录头 `usage`：场所 profile 允许的选择入口（#1774 B2）。缺省即 [`VenueUsage::General`]，
/// `usage` 不是旧记录的新前提。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueUsage {
    /// 回放 / 验收 / Python 的通用入口（按 symbol 或 profile id）。
    General,
    /// 只供 S 结构写者的专用入口 [`production_venue_for_s_writer`]。
    SStructureWriter,
}

impl VenueUsage {
    /// 记录头 `usage` 的写法；[`VenueUsage::General`] 对应缺省（不写）。
    pub fn header_value(self) -> Option<&'static str> {
        match self {
            VenueUsage::General => None,
            VenueUsage::SStructureWriter => Some("s_structure_writer"),
        }
    }
}

impl std::fmt::Display for VenueUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.header_value().unwrap_or("general"))
    }
}

/// 场所 profile 的解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct VenueProfile {
    pub record: PolicyRef,
    /// 记录头 `profile_id`（如 `P0`、`diagnostic-fixture`）。
    pub profile_id: String,
    /// 记录头 `symbol`；诊断夹具与 synthetic 为 `None`。
    pub symbol: Option<String>,
    /// 记录头 `usage`；缺省 [`VenueUsage::General`]。
    pub usage: VenueUsage,
    /// 记录头 `admissible`：`false` ⟹ 用它跑出的输出不参与准入。
    pub admissible: bool,
    pub tick_size: f64,
    pub commission_bps: f64,
    pub slippage_bps: f64,
    pub tax_bps: f64,
    pub entry_delay_bars: u32,
}

/// Θ_risk 记录的解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ThetaRisk {
    pub record: PolicyRef,
    pub rho: f64,
    pub beta: f64,
    pub gamma: f64,
    pub kappa: f64,
    pub default_lot: u32,
    pub chi_z_alpha: f64,
    pub enforce_gross_cap: bool,
    pub enforce_level_cap: bool,
    pub max_depth: u32,
    pub w_grade: [f64; 2],
}

/// 数据 profile 记录的解析结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataProfile {
    pub record: PolicyRef,
    pub bad_tick_max_dev: f64,
}

fn require(id: &'static str, name: &str, v: f64, ok: bool, rule: &str) -> Result<f64, PolicyError> {
    if ok {
        Ok(v)
    } else {
        Err(pr::invalid(id, name, format!("须 {rule}，得 {v}")))
    }
}

fn positive(id: &'static str, name: &str, v: f64) -> Result<f64, PolicyError> {
    require(id, name, v, v > 0.0, "> 0")
}

fn non_negative(id: &'static str, name: &str, v: f64) -> Result<f64, PolicyError> {
    require(id, name, v, v >= 0.0, "≥ 0")
}

/// 记录头里场所专有的四项。
struct VenueHeader {
    profile_id: String,
    symbol: Option<String>,
    usage: VenueUsage,
    admissible: bool,
}

/// 记录头里场所专有的四项：`profile_id`（必填字符串）、`symbol`（字符串或 null）、`usage`（可选，
/// 缺省 general）、`admissible`（必填布尔）。
fn venue_header(id: &'static str, text: &str) -> Result<VenueHeader, PolicyError> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| pr::invalid(id, "<record>", format!("不是合法 JSON：{e}")))?;
    let profile_id = root
        .get("profile_id")
        .and_then(Value::as_str)
        .ok_or_else(|| pr::invalid(id, "profile_id", "缺 profile_id 字符串"))?
        .to_string();
    let symbol = match root.get("symbol") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(v) => {
            return Err(pr::invalid(
                id,
                "symbol",
                format!("须为字符串或 null，得 {v}"),
            ))
        }
    };
    let usage = match root.get("usage") {
        None | Some(Value::Null) => VenueUsage::General,
        Some(Value::String(s)) if s == "s_structure_writer" => VenueUsage::SStructureWriter,
        Some(v) => {
            return Err(pr::invalid(
                id,
                "usage",
                format!("须缺省或为 \"s_structure_writer\"，得 {v}"),
            ))
        }
    };
    let admissible = root
        .get("admissible")
        .and_then(Value::as_bool)
        .ok_or_else(|| pr::invalid(id, "admissible", "缺 admissible 布尔值"))?;
    Ok(VenueHeader {
        profile_id,
        symbol,
        usage,
        admissible,
    })
}

/// 解析一份场所 profile。缺 `tick_size` 记在 `venue_profile` 槽，缺成本项记在 `cost_inputs` 槽，
/// 一次收齐全部缺口。
pub fn parse_venue_profile(
    id: &'static str,
    text: Option<&str>,
) -> Result<VenueProfile, RunPolicyError> {
    let mut err = RunPolicyError::default();
    let (record, fields) = match pr::parse_header(id, text) {
        Ok(x) => x,
        Err(e) => {
            err.add(&[SLOT_VENUE_PROFILE, SLOT_COST_INPUTS], e);
            return Err(err);
        }
    };
    let header = err.take(
        SLOT_VENUE_PROFILE,
        venue_header(id, text.unwrap_or_default()),
    );
    let tick_size = err.take(
        SLOT_VENUE_PROFILE,
        pr::read_f64(id, &fields, "tick_size").and_then(|v| positive(id, "tick_size", v)),
    );
    let mut cost = |name: &str| {
        err.take(
            SLOT_COST_INPUTS,
            pr::read_f64(id, &fields, name).and_then(|v| non_negative(id, name, v)),
        )
    };
    let commission_bps = cost("commission_bps");
    let slippage_bps = cost("slippage_bps");
    let tax_bps = cost("tax_bps");
    let entry_delay_bars = err.take(
        SLOT_COST_INPUTS,
        pr::read_u32(id, &fields, "entry_delay_bars"),
    );
    match (
        header,
        tick_size,
        commission_bps,
        slippage_bps,
        tax_bps,
        entry_delay_bars,
    ) {
        (
            Some(VenueHeader {
                profile_id,
                symbol,
                usage,
                admissible,
            }),
            Some(tick_size),
            Some(commission_bps),
            Some(slippage_bps),
            Some(tax_bps),
            Some(entry_delay_bars),
        ) if err.is_empty() => Ok(VenueProfile {
            record,
            profile_id,
            symbol,
            usage,
            admissible,
            tick_size,
            commission_bps,
            slippage_bps,
            tax_bps,
            entry_delay_bars,
        }),
        _ => Err(err),
    }
}

/// 解析 Θ_risk 记录。
pub fn parse_theta_risk(text: Option<&str>) -> Result<ThetaRisk, PolicyError> {
    let id = THETA_RISK_ID;
    let (record, fields) = pr::parse_header(id, text)?;
    let f = |name: &str| pr::read_f64(id, &fields, name);
    let rho = positive(id, "rho", f("rho")?)?;
    let beta = non_negative(id, "beta", f("beta")?)?;
    let gamma = positive(id, "gamma", f("gamma")?)?;
    let kappa = non_negative(id, "kappa", f("kappa")?)?;
    let default_lot = pr::read_u32(id, &fields, "default_lot")?;
    if default_lot == 0 {
        return Err(pr::invalid(id, "default_lot", "须 ≥ 1"));
    }
    let chi_z_alpha = non_negative(id, "chi_z_alpha", f("chi_z_alpha")?)?;
    let enforce_gross_cap = pr::read_bool(id, &fields, "enforce_gross_cap")?;
    let enforce_level_cap = pr::read_bool(id, &fields, "enforce_level_cap")?;
    let max_depth = pr::read_u32(id, &fields, "max_depth")?;
    if max_depth == 0 {
        return Err(pr::invalid(id, "max_depth", "须 ≥ 1"));
    }
    let w_grade = pr::read_f64_array::<2>(id, &fields, "w_grade")?;
    for w in w_grade {
        non_negative(id, "w_grade", w)?;
    }
    Ok(ThetaRisk {
        record,
        rho,
        beta,
        gamma,
        kappa,
        default_lot,
        chi_z_alpha,
        enforce_gross_cap,
        enforce_level_cap,
        max_depth,
        w_grade,
    })
}

/// 解析数据 profile 记录。
pub fn parse_data_profile(text: Option<&str>) -> Result<DataProfile, PolicyError> {
    let id = DATA_PROFILE_ID;
    let (record, fields) = pr::parse_header(id, text)?;
    let bad_tick_max_dev = positive(
        id,
        "bad_tick_max_dev",
        pr::read_f64(id, &fields, "bad_tick_max_dev")?,
    )?;
    Ok(DataProfile {
        record,
        bad_tick_max_dev,
    })
}

/// 诊断夹具场所 profile（`admissible: false`）。
pub fn diagnostic_fixture_venue() -> Result<VenueProfile, RunPolicyError> {
    parse_venue_profile(
        VENUE_PROFILE_DIAGNOSTIC_FIXTURE_ID,
        Some(VENUE_PROFILE_DIAGNOSTIC_FIXTURE_JSON),
    )
}

/// 按记录头某字段在生产表里找场所 profile；找不到 ⟹ `venue_profile` 与 `cost_inputs` 两槽都报缺记录。
fn production_venue_where(
    usage: VenueUsage,
    matches: impl Fn(&str, Option<&str>) -> bool,
) -> Result<VenueProfile, RunPolicyError> {
    production_venue_from_records(PRODUCTION_VENUE_PROFILES, usage, matches)
}

/// 从具名生产记录表选择场所。匹配表的记录头损坏是 `InvalidPolicy`，不得吞成“没有记录”。
/// 记录头 `usage` 与入口 `usage` 不符 ⟹ `InvalidPolicy{venue_profile}`，不产配置（#1774 B2）；
/// 该核对在读取任何字段值之前，与签署状态无关。
pub fn production_venue_from_records(
    records: &[(&'static str, &str)],
    usage: VenueUsage,
    matches: impl Fn(&str, Option<&str>) -> bool,
) -> Result<VenueProfile, RunPolicyError> {
    for &(id, text) in records {
        match venue_header(id, text) {
            Ok(header) => {
                if !matches(&header.profile_id, header.symbol.as_deref()) {
                    continue;
                }
                if header.usage != usage {
                    let mut err = RunPolicyError::default();
                    err.add(
                        &[SLOT_VENUE_PROFILE],
                        pr::invalid(
                            id,
                            "usage",
                            format!(
                                "usage 不符：记录 usage={}，本入口只接受 usage={usage}",
                                header.usage
                            ),
                        ),
                    );
                    return Err(err);
                }
                return parse_venue_profile(id, Some(text));
            }
            Err(error) => {
                let mut err = RunPolicyError::default();
                err.add(&[SLOT_VENUE_PROFILE, SLOT_COST_INPUTS], error);
                return Err(err);
            }
        }
    }
    Err(no_venue_profile())
}

/// 没有场所 profile：`venue_profile` 与 `cost_inputs` 两槽都报缺记录。
pub fn no_venue_profile() -> RunPolicyError {
    let mut err = RunPolicyError::default();
    err.add(
        &[SLOT_VENUE_PROFILE, SLOT_COST_INPUTS],
        PolicyError::RecordMissing {
            id: VENUE_PROFILE_ID,
        },
    );
    err
}

/// 生产场所 profile（通用入口）：按品种（大小写不敏感）匹配记录头 `symbol`；只接受 `usage` 缺省的记录。
pub fn production_venue_for_symbol(symbol: &str) -> Result<VenueProfile, RunPolicyError> {
    production_venue_where(VenueUsage::General, |_, s| {
        s.is_some_and(|s| s.eq_ignore_ascii_case(symbol))
    })
}

/// 生产场所 profile（通用入口）：按记录头 `profile_id` 匹配（如 `P0`）；只接受 `usage` 缺省的记录，
/// `usage: s_structure_writer` 的记录在此报 `InvalidPolicy{venue_profile}`（#1774 B2）。
pub fn production_venue_for_profile(profile_id: &str) -> Result<VenueProfile, RunPolicyError> {
    production_venue_where(VenueUsage::General, |p, _| p == profile_id)
}

/// S 结构写者专用入口（#1774 B2）：按记录头 `profile_id` 匹配，只接受 `usage: s_structure_writer`
/// 的记录；通用记录（如 replay `P0`）在此同样报 `InvalidPolicy{venue_profile}`。
pub fn production_venue_for_s_writer(profile_id: &str) -> Result<VenueProfile, RunPolicyError> {
    production_venue_where(VenueUsage::SStructureWriter, |p, _| p == profile_id)
}

/// 当前生效的 Θ_risk 记录。
pub fn active_theta_risk() -> Result<ThetaRisk, PolicyError> {
    parse_theta_risk(Some(ACTIVE_THETA_RISK_JSON))
}

/// 当前生效的数据 profile 记录。
pub fn active_data_profile() -> Result<DataProfile, PolicyError> {
    parse_data_profile(Some(ACTIVE_DATA_PROFILE_JSON))
}

/// 一次运行所需的全部记录（场所 profile 由调用方给出，其余取当前生效版本）。
#[derive(Debug, Clone, PartialEq)]
pub struct RunRecordSet {
    pub venue: VenueProfile,
    pub risk: ThetaRisk,
    pub data: DataProfile,
    pub tower: pr::TowerLevelBudget,
    pub level: super::config::LevelConfig,
    pub macd: super::config::MacdConfig,
    pub parse: super::config::ParseConfig,
}

/// 收齐一次运行的全部记录；任何缺口都进同一个 [`RunPolicyError`]。
pub fn collect_run_records(
    venue: Result<VenueProfile, RunPolicyError>,
    risk_text: Option<&str>,
    data_text: Option<&str>,
) -> Result<RunRecordSet, RunPolicyError> {
    let mut err = RunPolicyError::default();
    let venue = venue.map_err(|e| err.merge(e)).ok();
    let risk = err.take(SLOT_THETA_RISK, parse_theta_risk(risk_text));
    let data = err.take(SLOT_DATA_PROFILE, parse_data_profile(data_text));
    let tower = err.take(
        SLOT_TOWER_LEVEL_BUDGET,
        pr::parse_tower_level_budget(Some(ACTIVE_TOWER_LEVEL_BUDGET_JSON)),
    );
    let tower_base = err.take(SLOT_TOWER_BASE, pr::active_tower_base());
    let level = match (tower, tower_base) {
        (Some(budget), Some(base)) => {
            err.take(SLOT_TOWER_BASE, pr::try_level_config_from(budget, base, 3))
        }
        _ => None,
    };
    let macd = err.take(
        SLOT_MACD_FORCE,
        pr::parse_macd_force(Some(ACTIVE_MACD_FORCE_JSON)),
    );
    let parse = err.take(
        SLOT_PARSE_RECORDS,
        super::config::ParseConfig::from_active_records(),
    );
    match (venue, risk, data, tower, level, macd, parse) {
        (
            Some(venue),
            Some(risk),
            Some(data),
            Some(tower),
            Some(level),
            Some(macd),
            Some(parse),
        ) => Ok(RunRecordSet {
            venue,
            risk,
            data,
            tower,
            level,
            macd,
            parse,
        }),
        _ => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::policy_records::PolicyVersion;

    fn venue_text(fields: &str) -> String {
        format!(
            r#"{{"id":"venue-profile-P0","version":2,"profile_id":"P0","symbol":"BTC","admissible":true,"fields":{{{fields}}}}}"#
        )
    }

    const FULL_FIELDS: &str = r#""tick_size":{"value":0.01},"commission_bps":{"value":10},"slippage_bps":{"value":3},"tax_bps":{"value":0},"entry_delay_bars":{"value":2}"#;

    #[test]
    fn active_records_parse_with_version_1() {
        let v = diagnostic_fixture_venue().expect("夹具场所 profile 可解析");
        assert_eq!(v.record.label(), "venue-profile-diagnostic-fixture/1");
        assert!(!v.admissible, "诊断夹具不参与准入");
        assert_eq!(v.symbol, None);
        let r = active_theta_risk().expect("Θ_risk 记录可解析");
        assert_eq!(r.record.label(), "theta-risk/1");
        let d = active_data_profile().expect("数据 profile 可解析");
        assert_eq!(d.record.label(), "data-profile/1");
    }

    /// 等值对拍（#804）：v1 记录的值与改前 `impl Default` 常数逐位相同。
    #[test]
    fn v1_records_bit_equal_previous_constants() {
        let v = diagnostic_fixture_venue().unwrap();
        assert_eq!(v.tick_size.to_bits(), 1e-8_f64.to_bits());
        assert_eq!(v.commission_bps.to_bits(), 1.0_f64.to_bits());
        assert_eq!(v.slippage_bps.to_bits(), 2.0_f64.to_bits());
        assert_eq!(v.tax_bps.to_bits(), 0.0_f64.to_bits());
        assert_eq!(v.entry_delay_bars, 1);
        let r = active_theta_risk().unwrap();
        assert_eq!(r.rho.to_bits(), 0.005_f64.to_bits());
        assert_eq!(r.beta.to_bits(), 0.5_f64.to_bits());
        assert_eq!(r.gamma.to_bits(), 1.0_f64.to_bits());
        assert_eq!(r.kappa.to_bits(), 2.0_f64.to_bits());
        assert_eq!(r.default_lot, 1);
        assert_eq!(r.chi_z_alpha.to_bits(), 0.0_f64.to_bits());
        assert!(!r.enforce_gross_cap && !r.enforce_level_cap);
        assert_eq!(r.max_depth, 3);
        assert_eq!(r.w_grade.map(f64::to_bits), [1.0_f64.to_bits(); 2]);
        let d = active_data_profile().unwrap();
        assert_eq!(d.bad_tick_max_dev.to_bits(), 0.80_f64.to_bits());
    }

    /// #1774 测试夹具：synthetic v1（ADR 0057 裁定三只签 tick，三项成本 AwaitingPolicy）；不在生产表。
    const SYNTHETIC_UNSIGNED_V1: &str =
        include_str!("../../tests/fixtures/venue-profile-synthetic-tb02a-unsigned-v1.json");

    fn awaiting_fields(e: &RunPolicyError) -> Vec<(&'static str, &str)> {
        e.causes
            .iter()
            .map(|c| match c {
                PolicyError::AwaitingPolicy { id, field } => (*id, field.as_str()),
                other => panic!("非 AwaitingPolicy：{other}"),
            })
            .collect()
    }

    /// #1774 AC3/AC5（B1 负控 + 旧用法兼容）：replay 记录 tick 已签（ADR 0057 R1），BTC 与 `P0` 两路
    /// 都选中 replay 记录并因三项成本未签停在 AwaitingPolicy{cost_inputs}；缺字段恰为三项。
    #[test]
    fn production_p0_awaits_cost_inputs_only() {
        let e = production_venue_for_symbol("BTC").expect_err("P0 三项成本未签，必须停");
        assert_eq!(e.slots, vec![SLOT_COST_INPUTS]);
        assert!(e.is_awaiting_policy());
        assert_eq!(
            awaiting_fields(&e),
            vec![
                (VENUE_PROFILE_P0_ID, "slippage_bps"),
                (VENUE_PROFILE_P0_ID, "tax_bps"),
                (VENUE_PROFILE_P0_ID, "entry_delay_bars")
            ]
        );
        assert!(e.to_string().starts_with("AwaitingPolicy{cost_inputs}"));
        let by_profile = production_venue_for_profile("P0").unwrap_err();
        assert_eq!(by_profile.slots, e.slots);
        assert_eq!(by_profile.causes, e.causes);
    }

    /// #1774 AC2 第一层：S 专用选择器返回签齐的 synthetic 记录（三值 0/0/0 为模型假设、tick 1）。
    #[test]
    fn s_writer_selector_returns_signed_synthetic_profile() {
        let v =
            production_venue_for_s_writer(SYNTHETIC_TB02A_PROFILE_ID).expect("synthetic v2 已签");
        assert_eq!(v.record.label(), "venue-profile-synthetic-tb02a/2");
        assert_eq!(v.profile_id, SYNTHETIC_TB02A_PROFILE_ID);
        assert_eq!(v.symbol, None, "B1：symbol 路选不到");
        assert_eq!(v.usage, VenueUsage::SStructureWriter);
        assert!(v.admissible);
        assert_eq!(v.tick_size.to_bits(), 1.0_f64.to_bits());
        assert_eq!(
            (v.slippage_bps, v.tax_bps, v.entry_delay_bars),
            (0.0, 0.0, 0)
        );
        assert_eq!(v.commission_bps, 10.0, "ADR 0057 裁定一继承");
    }

    /// #1774 AC3/AC4（B1+B2 负控）：symbol 路选不到 synthetic；通用 profile 路遇 usage 记录报
    /// InvalidPolicy{venue_profile} 且原因为 usage 不符；S 专用入口同样不接 replay 记录。
    #[test]
    fn general_selectors_reject_s_writer_usage_and_vice_versa() {
        for &(id, text) in PRODUCTION_VENUE_PROFILES {
            let v = parse_venue_profile(id, Some(text)).ok();
            let header_symbol = serde_json::from_str::<Value>(text).unwrap()["symbol"].clone();
            if id == VENUE_PROFILE_SYNTHETIC_TB02A_ID {
                assert!(header_symbol.is_null(), "synthetic 记录头 symbol 须为 null");
                assert_eq!(v.unwrap().usage, VenueUsage::SStructureWriter);
            } else {
                assert!(header_symbol.is_string(), "{id}");
            }
        }
        let e = production_venue_for_profile(SYNTHETIC_TB02A_PROFILE_ID).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE]);
        assert!(!e.is_awaiting_policy());
        assert!(
            e.to_string().starts_with("InvalidPolicy{venue_profile}"),
            "{e}"
        );
        assert!(matches!(
            &e.causes[..],
            [PolicyError::Invalid { id: VENUE_PROFILE_SYNTHETIC_TB02A_ID, field, reason }]
                if field == "usage" && reason.contains("usage 不符")
        ));
        let e = production_venue_for_s_writer("P0").unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE]);
        assert!(matches!(
            &e.causes[..],
            [PolicyError::Invalid { id: VENUE_PROFILE_P0_ID, field, reason }]
                if field == "usage" && reason.contains("usage 不符")
        ));
        assert_eq!(
            production_venue_for_s_writer("nope").unwrap_err().slots,
            vec![SLOT_VENUE_PROFILE, SLOT_COST_INPUTS]
        );
    }

    /// #1774 AC6（旧已签记录正控）：无 `usage` 头的完整已签记录经同一选择函数按 symbol 与 profile 各选
    /// 一次都成功、原值读回，组装后 admissible；`usage` 不是旧记录的新前提。
    #[test]
    fn signed_record_without_usage_selects_by_symbol_and_profile() {
        let venue = r#"{"id":"venue-profile-P0","version":3,"profile_id":"P0","symbol":"BTC","admissible":true,
            "fields":{"tick_size":{"value":0.01},"commission_bps":{"value":10},"slippage_bps":{"value":4},
            "tax_bps":{"value":0.5},"entry_delay_bars":{"value":2}}}"#;
        let table: &[(&'static str, &str)] = &[(VENUE_PROFILE_P0_ID, venue)];
        let by_symbol = production_venue_from_records(table, VenueUsage::General, |_, s| {
            s.is_some_and(|s| s.eq_ignore_ascii_case("btc"))
        })
        .expect("按 symbol 选中");
        let by_profile =
            production_venue_from_records(table, VenueUsage::General, |p, _| p == "P0")
                .expect("按 profile 选中");
        assert_eq!(by_symbol, by_profile);
        assert_eq!(by_symbol.usage, VenueUsage::General);
        assert_eq!(
            (
                by_symbol.tick_size,
                by_symbol.commission_bps,
                by_symbol.slippage_bps,
                by_symbol.tax_bps,
                by_symbol.entry_delay_bars
            ),
            (0.01, 10.0, 4.0, 0.5, 2)
        );
        let c = super::super::config::ThetaConfig::from_records(
            Ok(by_symbol),
            Some(ACTIVE_THETA_RISK_JSON),
            Some(ACTIVE_DATA_PROFILE_JSON),
        )
        .unwrap();
        assert!(c.admissible());
        assert!(c.record_labels().contains("venue-profile-P0/3"));
    }

    /// #1774 AC7（未签负控）：synthetic v1 夹具经 S 专用入口的同一选择函数仍停在 AwaitingPolicy{cost_inputs}。
    #[test]
    fn unsigned_synthetic_fixture_awaits_cost_inputs() {
        let e = production_venue_from_records(
            &[(VENUE_PROFILE_SYNTHETIC_TB02A_ID, SYNTHETIC_UNSIGNED_V1)],
            VenueUsage::SStructureWriter,
            |p, _| p == SYNTHETIC_TB02A_PROFILE_ID,
        )
        .expect_err("三项成本未签，必须停");
        assert_eq!(e.slots, vec![SLOT_COST_INPUTS]);
        assert!(e.is_awaiting_policy());
        assert_eq!(
            awaiting_fields(&e),
            vec![
                (VENUE_PROFILE_SYNTHETIC_TB02A_ID, "slippage_bps"),
                (VENUE_PROFILE_SYNTHETIC_TB02A_ID, "tax_bps"),
                (VENUE_PROFILE_SYNTHETIC_TB02A_ID, "entry_delay_bars")
            ]
        );
    }

    /// 记录头 `usage` 取未知值 ⟹ InvalidPolicy{venue_profile}，不吞成 general。
    #[test]
    fn unknown_usage_header_is_invalid() {
        let text = venue_text(FULL_FIELDS)
            .replace(r#""symbol":"BTC","#, r#""symbol":"BTC","usage":"replay","#);
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&text)).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE]);
        assert!(!e.is_awaiting_policy());
        assert!(e.causes.iter().any(|c| matches!(
            c,
            PolicyError::Invalid { field, .. } if field == "usage"
        )));
    }

    /// 生产表里没有 profile 的品种 ⟹ 缺记录，两槽都报。
    #[test]
    fn malformed_production_header_is_invalid_not_missing() {
        let malformed = venue_text(FULL_FIELDS).replace(r#""admissible":true,"#, "");
        let error = production_venue_from_records(
            &[(VENUE_PROFILE_P0_ID, malformed.as_str())],
            VenueUsage::General,
            |_, _| true,
        )
        .unwrap_err();
        assert!(!error.is_awaiting_policy());
        assert!(error
            .to_string()
            .starts_with("InvalidPolicy{venue_profile, cost_inputs}"));
        assert!(error.causes.iter().any(|cause| matches!(
            cause,
            PolicyError::Invalid { id: VENUE_PROFILE_P0_ID, field, reason }
                if field == "admissible" && reason.contains("缺 admissible")
        )));
    }

    #[test]
    fn unknown_symbol_has_no_production_profile() {
        for sym in ["ES", "OKLO", "6E", "nope"] {
            let e = production_venue_for_symbol(sym).unwrap_err();
            assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE, SLOT_COST_INPUTS]);
            assert_eq!(
                e.causes,
                vec![PolicyError::RecordMissing {
                    id: VENUE_PROFILE_ID
                }]
            );
        }
        assert!(production_venue_for_profile("P2").is_err(), "P2 未启用");
        assert!(
            production_venue_for_profile("diagnostic-fixture").is_err(),
            "夹具不在生产表"
        );
    }

    /// 签齐的场所 profile 按记录值与版本读出（变异体：改回常数 ⟹ 本测试红）。
    #[test]
    fn signed_venue_profile_reads_record_values() {
        let text = venue_text(FULL_FIELDS);
        let v = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&text)).unwrap();
        assert_eq!(v.record.version, PolicyVersion::Signed(2));
        assert_eq!(
            (
                v.tick_size,
                v.commission_bps,
                v.slippage_bps,
                v.tax_bps,
                v.entry_delay_bars
            ),
            (0.01, 10.0, 3.0, 0.0, 2)
        );
        assert!(v.admissible);
    }

    /// 负控：缺记录 / 缺字段 / 非法值。
    #[test]
    fn negative_controls_missing_absent_invalid() {
        // 缺记录。
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, None).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE, SLOT_COST_INPUTS]);
        assert_eq!(
            parse_theta_risk(None),
            Err(PolicyError::RecordMissing { id: THETA_RISK_ID })
        );
        assert_eq!(
            parse_data_profile(None),
            Err(PolicyError::RecordMissing {
                id: DATA_PROFILE_ID
            })
        );
        // 只缺 tick_size ⟹ 只报 venue_profile。
        let no_tick = venue_text(&FULL_FIELDS.replace(r#""tick_size":{"value":0.01},"#, ""));
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&no_tick)).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE]);
        // 只缺 commission ⟹ 只报 cost_inputs。
        let no_comm = venue_text(&FULL_FIELDS.replace(r#""commission_bps":{"value":10},"#, ""));
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&no_comm)).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_COST_INPUTS]);
        assert!(e.is_awaiting_policy());
        // 缺 Θ_risk 字段。
        let risk_no_rho = ACTIVE_THETA_RISK_JSON.replacen("\"rho\"", "\"rho_removed\"", 1);
        assert_eq!(
            parse_theta_risk(Some(&risk_no_rho)),
            Err(PolicyError::AwaitingPolicy {
                id: THETA_RISK_ID,
                field: "rho".into()
            })
        );
        // 非法值：tick ≤ 0、负成本、rho ≤ 0、阈值 ≤ 0、max_depth 0、w_grade 长度错。
        let bad_tick = venue_text(&FULL_FIELDS.replace("0.01", "0"));
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&bad_tick)).unwrap_err();
        assert!(!e.is_awaiting_policy());
        assert!(e.to_string().starts_with("InvalidPolicy{venue_profile}"));
        let neg_slip = venue_text(&FULL_FIELDS.replace(r#"{"value":3}"#, r#"{"value":-3}"#));
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&neg_slip)).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_COST_INPUTS]);
        assert!(!e.is_awaiting_policy());
        let bad_rho = r#"{"id":"theta-risk","version":1,"fields":{"rho":{"value":0}}}"#;
        assert!(matches!(
            parse_theta_risk(Some(bad_rho)),
            Err(PolicyError::Invalid { .. })
        ));
        let bad_depth = ACTIVE_THETA_RISK_JSON.replacen("\"value\": 3", "\"value\": 0", 1);
        assert!(matches!(
            parse_theta_risk(Some(&bad_depth)),
            Err(PolicyError::Invalid { .. })
        ));
        let bad_wg = ACTIVE_THETA_RISK_JSON.replacen("[1.0, 1.0]", "[1.0]", 1);
        assert!(matches!(
            parse_theta_risk(Some(&bad_wg)),
            Err(PolicyError::Invalid { .. })
        ));
        let bad_dev =
            r#"{"id":"data-profile","version":1,"fields":{"bad_tick_max_dev":{"value":-0.8}}}"#;
        assert!(matches!(
            parse_data_profile(Some(bad_dev)),
            Err(PolicyError::Invalid { .. })
        ));
        let no_admissible = venue_text(FULL_FIELDS).replace(r#""admissible":true,"#, "");
        let e = parse_venue_profile(VENUE_PROFILE_P0_ID, Some(&no_admissible)).unwrap_err();
        assert_eq!(e.slots, vec![SLOT_VENUE_PROFILE]);
    }

    /// 收齐：场所缺口与其余记录缺口同时出现时一次全报。
    #[test]
    fn collect_reports_every_slot_at_once() {
        let e = collect_run_records(production_venue_for_symbol("BTC"), None, None).unwrap_err();
        assert_eq!(
            e.slots,
            vec![SLOT_COST_INPUTS, SLOT_THETA_RISK, SLOT_DATA_PROFILE]
        );
        assert!(e.is_awaiting_policy());
        let ok = collect_run_records(
            diagnostic_fixture_venue(),
            Some(ACTIVE_THETA_RISK_JSON),
            Some(ACTIVE_DATA_PROFILE_JSON),
        )
        .unwrap();
        assert_eq!(ok.tower.record.label(), "tower-level-budget/1");
        assert_eq!(ok.level.budget, ok.tower.record);
        assert_eq!(
            ok.level,
            super::super::config::LevelConfig::from_active_record().unwrap()
        );
        assert_eq!(ok.level.tower_base_policy.label(), "tower-base/1");
    }
}
