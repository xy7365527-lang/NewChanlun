//! #1601（#1586 审计 C-01/C-02/C-22，ADR 0040 / ADR 0041）：资金签署记录与起始操作级别的读取点。
//!
//! 回放总投入 `Q_total`、NT 生产路径的下单基数与旧经济臂的操作级别都只从 ADR 0040「签署记录」读取：
//!
//! - funding-record（[`funding_record/funding-record.json`]）：每个 profile 一条，`cap` 承载 `Q_total`
//!   （ADR 0040:100）；P0 100,000、P1-hl-testnet 500、P0-acceptance-5m 5,000,000（`scope: evidence_only`，
//!   只出证据、不进生产、不下单，ADR 0040:34）。
//! - derivation-input（[`funding_record/chong-derivation-input.json`]）：`l_start` 起始操作级别。
//! - 旧线 ARM 次级别确认过期窗口不在 ADR 0040 里，落旧线 policy 记录 `arm-sub-expiry`
//!   （[`crate::theta_v0::policy_records::active_arm_sub_expiry`]）。
//!
//! 记录文件是 ADR 0040 签署块的逐字转录，随源码入库并编进二进制（`include_str!`）；
//! 转录漂移由 `tests::active_records_match_adr_0040` 对照 ADR 正文锁住。数值只写在记录里，
//! Rust 代码不持有资金常数。改值 ＝ 在 ADR 签新版本（`supersedes` 指向旧版本）并同步转录。
//!
//! ## ℓ ↔ ladder 换算（C-22）
//!
//! 旧经济臂的 ladder 坐标：0=bar 1=bi 2=segment 3=move(L1) 4..=recL2..（`trading::types::ladder_name`）。
//! ADR 0040 的 ℓ 是走势级别坐标，L0 ＝ 塔底线段层（ADR 0040 裁定二，`qujiantao.md:516`）。
//! 故 `ladder = FIRST_BSP_LADDER + ℓ`：L0 ↔ ladder 2（segment），L1 ↔ ladder 3（move(L1)）。
//! 改前常数 `DEFAULT_OP_LADDER = 3` 对应 `l_start = L1`；ADR 0040 三个 profile 的 v1 都签 L0 ⟹
//! 读生效记录时旧臂操作级别由 3 变 2（不等值，见 #1601 交付说明）。
//!
//! ## 缺值行为（ADR 0041 裁定二推论、#799）
//!
//! 记录文本缺失、profile 无记录、字段缺失或写作 `null` / `"AwaitingPolicy"` ⟹
//! [`FundingError::AwaitingPolicy`]；值非法 ⟹ [`FundingError::Invalid`]。调用方必须停止，
//! 本模块没有备用值，不回退任何旧常数。签署有效期（`valid_through`）不在此核：回放没有可审计的
//! 当前日期，过期核验归开局批（`economic_session::tb03c_funding`，ADR 0040 裁定四第 4/5 步）。

use serde_json::{Map, Value};

use crate::theta_v0::policy_records::{self, ArmSubExpiry, PolicyError};
use crate::trading::types::{FIRST_BSP_LADDER, MAX_LADDER};

/// funding-record 槽位名（`AwaitingPolicy{funding_record}`）。
pub const FUNDING_RECORD_SLOT: &str = "funding_record";
/// derivation-input 槽位名（`AwaitingPolicy{derivation_input}`）。
pub const DERIVATION_INPUT_SLOT: &str = "derivation_input";
/// ADR 0040 funding-record 政策族（标识不是数值；版本号从记录本身读）。
pub const FUNDING_POLICY_ID: &str = "#1557/funding-authorization/v1";
/// ADR 0040 derivation-input 政策族。
pub const DERIVATION_POLICY_ID: &str = "#1558/chong-derivation-input/v1";
/// ADR 0040 `scope: evidence_only`：验收专用回放，不进生产、不下单。
pub const EVIDENCE_ONLY: &str = "evidence_only";

/// 当前生效的 funding-record（编进二进制）。
pub const ACTIVE_FUNDING_RECORD_JSON: &str = include_str!("funding_record/funding-record.json");
/// 当前生效的 derivation-input（编进二进制）。
pub const ACTIVE_DERIVATION_INPUT_JSON: &str =
    include_str!("funding_record/chong-derivation-input.json");

/// 读取签署记录失败。调用方必须停止，不得换值重试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FundingError {
    /// 记录文本缺失、profile 无记录（`field = None`），或字段未签（`field = Some(..)`）。
    AwaitingPolicy {
        slot: &'static str,
        profile: String,
        field: Option<String>,
    },
    /// 记录或字段值非法。
    Invalid {
        slot: &'static str,
        profile: String,
        field: String,
        reason: String,
    },
}

impl std::fmt::Display for FundingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundingError::AwaitingPolicy {
                slot,
                profile,
                field: None,
            } => write!(
                f,
                "AwaitingPolicy{{{slot}, profile={profile}}}：无签署记录，停止（不回退任何备用值）"
            ),
            FundingError::AwaitingPolicy {
                slot,
                profile,
                field: Some(field),
            } => write!(
                f,
                "AwaitingPolicy{{{slot}.{field}, profile={profile}}}：字段未签，停止（不回退任何备用值）"
            ),
            FundingError::Invalid {
                slot,
                profile,
                field,
                reason,
            } => write!(f, "InvalidPolicy{{{slot}.{field}, profile={profile}}}：{reason}"),
        }
    }
}

impl std::error::Error for FundingError {}

/// 按 profile 从记录文本里取当前生效的那一条：同 profile 中未被他条 `supersedes` 指向的版本，须恰一条。
fn select_active(
    slot: &'static str,
    expected_id: &str,
    text: Option<&str>,
    profile: &str,
) -> Result<Map<String, Value>, FundingError> {
    let awaiting = || FundingError::AwaitingPolicy {
        slot,
        profile: profile.to_string(),
        field: None,
    };
    let invalid = |field: &str, reason: String| FundingError::Invalid {
        slot,
        profile: profile.to_string(),
        field: field.to_string(),
        reason,
    };
    let text = text.ok_or_else(awaiting)?;
    let root: Value = serde_json::from_str(text)
        .map_err(|e| invalid("<record>", format!("不是合法 JSON：{e}")))?;
    if root.get("id").and_then(Value::as_str) != Some(expected_id) {
        return Err(invalid("id", format!("记录文件 id 须为 `{expected_id}`")));
    }
    let records = root
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("records", "缺 records 数组".into()))?;
    let mine: Vec<&Map<String, Value>> = records
        .iter()
        .filter_map(Value::as_object)
        .filter(|r| r.get("profile_id").and_then(Value::as_str) == Some(profile))
        .collect();
    let superseded: Vec<&str> = mine
        .iter()
        .filter_map(|r| r.get("supersedes").and_then(Value::as_str))
        .collect();
    let live: Vec<&&Map<String, Value>> = mine
        .iter()
        .filter(|r| {
            r.get("version")
                .and_then(Value::as_str)
                .is_none_or(|v| !superseded.contains(&v))
        })
        .collect();
    match live.as_slice() {
        [] => Err(awaiting()),
        [one] => Ok((**one).clone()),
        _ => Err(invalid(
            "version",
            format!(
                "同一 profile 有 {} 条未被取代的版本，生效版本不唯一",
                live.len()
            ),
        )),
    }
}

/// 读一个已签文本字段：缺失、`null`、`"AwaitingPolicy"` ⟹ 未签；空串或非字符串 ⟹ 非法。
fn signed_text(
    slot: &'static str,
    profile: &str,
    rec: &Map<String, Value>,
    field: &str,
) -> Result<String, FundingError> {
    match rec.get(field) {
        None | Some(Value::Null) => Err(FundingError::AwaitingPolicy {
            slot,
            profile: profile.to_string(),
            field: Some(field.to_string()),
        }),
        Some(Value::String(s)) if s == "AwaitingPolicy" => Err(FundingError::AwaitingPolicy {
            slot,
            profile: profile.to_string(),
            field: Some(field.to_string()),
        }),
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(v) => Err(FundingError::Invalid {
            slot,
            profile: profile.to_string(),
            field: field.to_string(),
            reason: format!("须为非空文本，得 {v}"),
        }),
    }
}

/// 读记录头公共字段并核政策族与 scope。返回 (policy_id, version, supersedes, scope)。
fn header(
    slot: &'static str,
    expected_policy: &str,
    profile: &str,
    rec: &Map<String, Value>,
) -> Result<(String, String, String, Option<String>), FundingError> {
    let policy_id = signed_text(slot, profile, rec, "policy_id")?;
    if policy_id != expected_policy {
        return Err(FundingError::Invalid {
            slot,
            profile: profile.to_string(),
            field: "policy_id".into(),
            reason: format!("政策族须为 {expected_policy}，得 {policy_id}"),
        });
    }
    let version = signed_text(slot, profile, rec, "version")?;
    let supersedes = signed_text(slot, profile, rec, "supersedes")?;
    for k in ["authorizer", "authorized_at", "valid_through"] {
        signed_text(slot, profile, rec, k)?;
    }
    let scope = match rec.get("scope") {
        None => None,
        Some(Value::String(s)) if s == EVIDENCE_ONLY => Some(s.clone()),
        Some(v) => {
            return Err(FundingError::Invalid {
                slot,
                profile: profile.to_string(),
                field: "scope".into(),
                reason: format!("scope 只认 {EVIDENCE_ONLY}，得 {v}"),
            })
        }
    };
    Ok((policy_id, version, supersedes, scope))
}

/// ADR 0040 funding-record 当前生效版本（一 profile 一条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingRecord {
    pub profile_id: String,
    pub policy_id: String,
    pub version: String,
    pub supersedes: String,
    pub scope: Option<String>,
    /// 总投入 `Q_total`（记录字段 `cap`，USDC 名义整数，ADR 0040:100）。
    pub q_total: i64,
    pub currency: String,
}

impl FundingRecord {
    /// 从记录文本按 profile 解析当前生效版本。
    pub fn parse(text: Option<&str>, profile: &str) -> Result<Self, FundingError> {
        let slot = FUNDING_RECORD_SLOT;
        let rec = select_active(slot, "funding-record", text, profile)?;
        let (policy_id, version, supersedes, scope) =
            header(slot, FUNDING_POLICY_ID, profile, &rec)?;
        let currency = signed_text(slot, profile, &rec, "currency")?;
        let q_total = match rec.get("cap") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s == "AwaitingPolicy" => None,
            Some(v) => {
                Some(
                    v.as_i64()
                        .filter(|&q| q > 0)
                        .ok_or_else(|| FundingError::Invalid {
                            slot,
                            profile: profile.to_string(),
                            field: "cap".into(),
                            reason: format!("总投入须为正整数，得 {v}"),
                        })?,
                )
            }
        }
        .ok_or_else(|| FundingError::AwaitingPolicy {
            slot,
            profile: profile.to_string(),
            field: Some("cap".into()),
        })?;
        Ok(Self {
            profile_id: profile.to_string(),
            policy_id,
            version,
            supersedes,
            scope,
            q_total,
            currency,
        })
    }

    /// 当前生效记录（编进二进制）里该 profile 的 funding-record。
    pub fn active(profile: &str) -> Result<Self, FundingError> {
        Self::parse(Some(ACTIVE_FUNDING_RECORD_JSON), profile)
    }

    /// `Q_total` 的浮点读数（旧经济臂与 NT 下单基数是 f64 域）。
    pub fn q_total_f64(&self) -> f64 {
        self.q_total as f64
    }

    /// 输出戳：`<policy_id>@<profile_id>/<version>`，如 `#1557/funding-authorization/v1@P0/v1`。
    pub fn label(&self) -> String {
        format!("{}@{}/{}", self.policy_id, self.profile_id, self.version)
    }

    pub fn is_evidence_only(&self) -> bool {
        self.scope.as_deref() == Some(EVIDENCE_ONLY)
    }

    /// 生产下单路径的准入：`evidence_only` 记录只出证据、不下单（ADR 0040:34）⟹ 拒收。
    pub fn require_production(self) -> Result<Self, FundingError> {
        if self.is_evidence_only() {
            return Err(FundingError::Invalid {
                slot: FUNDING_RECORD_SLOT,
                profile: self.profile_id.clone(),
                field: "scope".into(),
                reason: "evidence_only 记录只出证据、不进生产、不下单（ADR 0040:34）".into(),
            });
        }
        Ok(self)
    }

    /// NT 生产下单路径的唯一准入口：无记录 ⟹ `AwaitingPolicy{funding_record}`；
    /// `evidence_only` ⟹ `Invalid{scope}`。两种情况都不构造策略、不下单。
    pub fn admit_production(rec: Option<Self>) -> Result<Self, FundingError> {
        rec.ok_or_else(|| FundingError::AwaitingPolicy {
            slot: FUNDING_RECORD_SLOT,
            profile: "<未提供 funding-record>".into(),
            field: None,
        })?
        .require_production()
    }

    /// 按 profile 读生产下单用的 funding-record（未给 profile ⟹ `AwaitingPolicy{funding_record}`）。
    pub fn production(profile: Option<&str>) -> Result<Self, FundingError> {
        let profile = profile.ok_or_else(|| FundingError::AwaitingPolicy {
            slot: FUNDING_RECORD_SLOT,
            profile: "<未指定 funding_profile>".into(),
            field: None,
        })?;
        Self::admit_production(Some(Self::active(profile)?))
    }
}

/// ADR 0040 derivation-input 当前生效版本（本模块只取 `l_start`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationInput {
    pub profile_id: String,
    pub policy_id: String,
    pub version: String,
    pub scope: Option<String>,
    /// 起始操作级别 ℓ（`Ln` 的 n）。
    pub l_start: u32,
}

impl DerivationInput {
    pub fn parse(text: Option<&str>, profile: &str) -> Result<Self, FundingError> {
        let slot = DERIVATION_INPUT_SLOT;
        let rec = select_active(slot, "chong-derivation-input", text, profile)?;
        let (policy_id, version, _supersedes, scope) =
            header(slot, DERIVATION_POLICY_ID, profile, &rec)?;
        let raw = signed_text(slot, profile, &rec, "l_start")?;
        let l_start = raw
            .strip_prefix('L')
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|&n| FIRST_BSP_LADDER + (n as usize) < MAX_LADDER)
            .ok_or_else(|| FundingError::Invalid {
                slot,
                profile: profile.to_string(),
                field: "l_start".into(),
                reason: format!(
                    "须为 Ln 且 {FIRST_BSP_LADDER}+n < {MAX_LADDER}（ladder 承载上界），得 {raw}"
                ),
            })?;
        Ok(Self {
            profile_id: profile.to_string(),
            policy_id,
            version,
            scope,
            l_start,
        })
    }

    pub fn active(profile: &str) -> Result<Self, FundingError> {
        Self::parse(Some(ACTIVE_DERIVATION_INPUT_JSON), profile)
    }

    /// 旧经济臂 ladder 坐标：`FIRST_BSP_LADDER + ℓ`（见模块头换算）。
    pub fn op_ladder(&self) -> usize {
        FIRST_BSP_LADDER + self.l_start as usize
    }

    pub fn label(&self) -> String {
        format!("{}@{}/{}", self.policy_id, self.profile_id, self.version)
    }
}

/// 回放读取记录失败（资金签署记录或旧线 policy 记录）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayRecordError {
    Funding(FundingError),
    Policy(PolicyError),
}

impl std::fmt::Display for ReplayRecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayRecordError::Funding(e) => e.fmt(f),
            ReplayRecordError::Policy(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ReplayRecordError {}

impl From<FundingError> for ReplayRecordError {
    fn from(e: FundingError) -> Self {
        ReplayRecordError::Funding(e)
    }
}

impl From<PolicyError> for ReplayRecordError {
    fn from(e: PolicyError) -> Self {
        ReplayRecordError::Policy(e)
    }
}

/// 旧经济臂一次回放所读的全部记录：总投入、起始操作级别、ARM 过期窗口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecords {
    pub funding: FundingRecord,
    pub derivation: DerivationInput,
    pub sub_expiry: ArmSubExpiry,
}

impl ReplayRecords {
    /// 当前生效记录里该 profile 的回放记录。
    pub fn active(profile: &str) -> Result<Self, ReplayRecordError> {
        Ok(Self {
            funding: FundingRecord::active(profile)?,
            derivation: DerivationInput::active(profile)?,
            sub_expiry: policy_records::active_arm_sub_expiry()?,
        })
    }

    /// 回放总投入（旧经济臂的期初现金池）。
    pub fn q_total(&self) -> f64 {
        self.funding.q_total_f64()
    }

    /// 操作绑定级别（ladder 坐标）。
    pub fn op_ladder(&self) -> usize {
        self.derivation.op_ladder()
    }

    /// ARM 后次级别确认过期 bar 数。
    pub fn sub_expiry(&self) -> i64 {
        i64::from(self.sub_expiry.bars)
    }

    /// 输出戳：三份记录的 id 与版本。
    pub fn label(&self) -> String {
        format!(
            "{}; {}; {}",
            self.funding.label(),
            self.derivation.label(),
            self.sub_expiry.record.label()
        )
    }

    /// 测试夹具：与改前常数等值的记录（Q_total 100,000 ＝ 旧 `INITIAL_CAPITAL`；l_start L1 ＝ 旧
    /// `DEFAULT_OP_LADDER` 3；ARM 过期窗口取生效记录 60 ＝ 旧 `SUB_EXPIRY`）。走同一解析器，
    /// profile 名写明是夹具，不进任何生产路径（#1601 等值对拍）。
    #[cfg(test)]
    pub fn legacy_parity_fixture() -> Self {
        const PROFILE: &str = "TEST-legacy-parity";
        let funding = r##"{"id":"funding-record","records":[{"profile_id":"TEST-legacy-parity","policy_id":"#1557/funding-authorization/v1","version":"test-fixture","supersedes":"none","cap":100000,"currency":"USDC","authorizer":"测试夹具（#1601 等值对拍）","authorized_at":"2026-10-22","valid_through":"2026-10-22"}]}"##;
        let derivation = r##"{"id":"chong-derivation-input","records":[{"profile_id":"TEST-legacy-parity","policy_id":"#1558/chong-derivation-input/v1","version":"test-fixture","supersedes":"none","symbol":"BTC","l_start":"L1","authorizer":"测试夹具（#1601 等值对拍）","authorized_at":"2026-10-22","valid_through":"2026-10-22"}]}"##;
        Self {
            funding: FundingRecord::parse(Some(funding), PROFILE).expect("夹具 funding-record"),
            derivation: DerivationInput::parse(Some(derivation), PROFILE)
                .expect("夹具 derivation-input"),
            sub_expiry: policy_records::active_arm_sub_expiry().expect("生效 arm-sub-expiry"),
        }
    }
}

/// PyO3 入口共用：按调用方给的 profile 读回放记录。未给 profile ⟹ `AwaitingPolicy{funding_record}`
/// 报错（不回退任何默认 profile 或资金值，#799 / ADR 0041）。
pub fn py_replay_records(profile: Option<&str>) -> pyo3::PyResult<ReplayRecords> {
    let profile = py_profile(profile)?;
    ReplayRecords::active(profile)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// PyO3 入口共用：按 profile 读 funding-record（只需 `Q_total` 的引擎）。
pub fn py_funding_record(profile: Option<&str>) -> pyo3::PyResult<FundingRecord> {
    let profile = py_profile(profile)?;
    FundingRecord::active(profile)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

fn py_profile(profile: Option<&str>) -> pyo3::PyResult<&str> {
    profile.ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(
            FundingError::AwaitingPolicy {
                slot: FUNDING_RECORD_SLOT,
                profile: "<未指定 funding_profile>".into(),
                field: None,
            }
            .to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_profiles_carry_signed_q_total_and_version() {
        for (profile, q) in [
            ("P0", 100_000),
            ("P1-hl-testnet", 500),
            ("P0-acceptance-5m", 5_000_000),
        ] {
            let r = FundingRecord::active(profile).expect("生效 funding-record");
            assert_eq!(r.q_total, q, "{profile}");
            assert_eq!(r.currency, "USDC");
            assert_eq!(
                r.label(),
                format!("#1557/funding-authorization/v1@{profile}/v1")
            );
            let d = DerivationInput::active(profile).expect("生效 derivation-input");
            assert_eq!(d.l_start, 0, "{profile} l_start = L0");
            assert_eq!(d.op_ladder(), 2, "L0 ↔ ladder 2（segment）");
        }
        assert!(FundingRecord::active("P0-acceptance-5m")
            .unwrap()
            .is_evidence_only());
        assert!(!FundingRecord::active("P0").unwrap().is_evidence_only());
    }

    /// 夹具与 `trading::types` 的 cfg(test) 夹具常量同值（等值对拍的前提）。
    #[test]
    fn legacy_fixture_matches_fixture_constants() {
        use crate::trading::types::{FIXTURE_OP_LADDER, FIXTURE_Q_TOTAL, FIXTURE_SUB_EXPIRY};
        let fx = ReplayRecords::legacy_parity_fixture();
        assert_eq!(fx.q_total(), FIXTURE_Q_TOTAL);
        assert_eq!(fx.op_ladder(), FIXTURE_OP_LADDER);
        assert_eq!(fx.sub_expiry(), FIXTURE_SUB_EXPIRY);
    }

    #[test]
    fn replay_records_label_names_every_record_and_version() {
        let r = ReplayRecords::active("P0").unwrap();
        assert_eq!(r.q_total(), 100_000.0);
        assert_eq!(r.op_ladder(), 2);
        assert_eq!(r.sub_expiry(), 60);
        assert_eq!(
            r.label(),
            "#1557/funding-authorization/v1@P0/v1; #1558/chong-derivation-input/v1@P0/v1; arm-sub-expiry/1"
        );
        let fx = ReplayRecords::legacy_parity_fixture();
        assert_eq!(
            (fx.q_total(), fx.op_ladder(), fx.sub_expiry()),
            (100_000.0, 3, 60)
        );
    }

    /// 负控：缺记录、缺 profile、缺字段、未签、非法值、生效版本不唯一、evidence_only 进生产。
    #[test]
    fn negative_controls_missing_unsigned_invalid() {
        assert_eq!(
            FundingRecord::parse(None, "P0"),
            Err(FundingError::AwaitingPolicy {
                slot: FUNDING_RECORD_SLOT,
                profile: "P0".into(),
                field: None
            })
        );
        assert_eq!(
            FundingRecord::active("P9-unsigned"),
            Err(FundingError::AwaitingPolicy {
                slot: FUNDING_RECORD_SLOT,
                profile: "P9-unsigned".into(),
                field: None
            })
        );
        assert!(matches!(
            ReplayRecords::active("P9-unsigned"),
            Err(ReplayRecordError::Funding(
                FundingError::AwaitingPolicy { .. }
            ))
        ));
        let rec = |cap: &str| {
            format!(
                r##"{{"id":"funding-record","records":[{{"profile_id":"X","policy_id":"#1557/funding-authorization/v1","version":"v1","supersedes":"none",{cap}"currency":"USDC","authorizer":"a","authorized_at":"2026-10-20","valid_through":"2027-10-20"}}]}}"##
            )
        };
        let no_cap = rec("");
        assert_eq!(
            FundingRecord::parse(Some(&no_cap), "X"),
            Err(FundingError::AwaitingPolicy {
                slot: FUNDING_RECORD_SLOT,
                profile: "X".into(),
                field: Some("cap".into())
            })
        );
        for unsigned in [r#""cap":null,"#, r#""cap":"AwaitingPolicy","#] {
            assert!(matches!(
                FundingRecord::parse(Some(&rec(unsigned)), "X"),
                Err(FundingError::AwaitingPolicy { field: Some(_), .. })
            ));
        }
        for bad in [
            r#""cap":0,"#,
            r#""cap":-5,"#,
            r#""cap":1.5,"#,
            r#""cap":"100000","#,
        ] {
            assert!(
                matches!(
                    FundingRecord::parse(Some(&rec(bad)), "X"),
                    Err(FundingError::Invalid { .. })
                ),
                "{bad}"
            );
        }
        let ok = FundingRecord::parse(Some(&rec(r#""cap":7,"#)), "X").unwrap();
        assert_eq!(ok.q_total, 7);
        // 两条互不取代的版本 ⟹ 生效版本不唯一。
        let two = r##"{"id":"funding-record","records":[
            {"profile_id":"X","policy_id":"#1557/funding-authorization/v1","version":"v1","supersedes":"none","cap":1,"currency":"USDC","authorizer":"a","authorized_at":"2026-10-20","valid_through":"2027-10-20"},
            {"profile_id":"X","policy_id":"#1557/funding-authorization/v1","version":"v2","supersedes":"none","cap":2,"currency":"USDC","authorizer":"a","authorized_at":"2026-10-21","valid_through":"2027-10-20"}]}"##;
        assert!(matches!(
            FundingRecord::parse(Some(two), "X"),
            Err(FundingError::Invalid { .. })
        ));
        // v2 supersedes v1 ⟹ 读 v2。
        let chain = two.replace(
            r#""version":"v2","supersedes":"none""#,
            r#""version":"v2","supersedes":"v1""#,
        );
        let v2 = FundingRecord::parse(Some(&chain), "X").unwrap();
        assert_eq!((v2.version.as_str(), v2.q_total), ("v2", 2));
        // 非法 l_start。
        let d = |l: &str| {
            format!(
                r##"{{"id":"chong-derivation-input","records":[{{"profile_id":"X","policy_id":"#1558/chong-derivation-input/v1","version":"v1","supersedes":"none","l_start":{l},"authorizer":"a","authorized_at":"2026-10-20","valid_through":"2027-10-20"}}]}}"##
            )
        };
        for bad in [r#""0""#, r#""L-1""#, r#""L9""#, "3"] {
            assert!(
                matches!(
                    DerivationInput::parse(Some(&d(bad)), "X"),
                    Err(FundingError::Invalid { .. })
                ),
                "{bad}"
            );
        }
        assert!(matches!(
            DerivationInput::parse(Some(&d("null")), "X"),
            Err(FundingError::AwaitingPolicy { .. })
        ));
        assert_eq!(
            DerivationInput::parse(Some(&d(r#""L1""#)), "X")
                .unwrap()
                .op_ladder(),
            3
        );
        // evidence_only 不进生产。
        let e = FundingRecord::active("P0-acceptance-5m")
            .unwrap()
            .require_production();
        assert!(matches!(e, Err(FundingError::Invalid { ref field, .. }) if field == "scope"));
        assert!(FundingRecord::active("P0")
            .unwrap()
            .require_production()
            .is_ok());
    }

    /// 解析 ADR 0040 的 ```<tag> 签署块：空行分条，每条 `key: value`。
    fn adr_block(adr: &str, tag: &str) -> Vec<Vec<(String, String)>> {
        let open = format!("```{tag}\n");
        let start = adr.find(&open).expect("ADR 0040 签署块") + open.len();
        let body = &adr[start..start + adr[start..].find("```").expect("块尾")];
        body.split("\n\n")
            .map(|rec| {
                rec.lines()
                    .filter_map(|l| l.split_once(": "))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .collect()
            })
            .collect()
    }

    fn json_text(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// 转录漂移锁：JSON 记录的每个字段与 ADR 0040 签署块逐字一致，profile 集合相同。
    #[test]
    fn active_records_match_adr_0040() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/adr/0040-funding-signature-and-chong-derivation.md");
        let adr = std::fs::read_to_string(&path).expect("读 ADR 0040");
        for (tag, json) in [
            ("funding-record", ACTIVE_FUNDING_RECORD_JSON),
            ("derivation-input", ACTIVE_DERIVATION_INPUT_JSON),
        ] {
            let blocks = adr_block(&adr, tag);
            let root: Value = serde_json::from_str(json).unwrap();
            let records = root["records"].as_array().unwrap();
            assert_eq!(records.len(), blocks.len(), "{tag} 条数");
            for (rec, block) in records.iter().zip(&blocks) {
                let obj = rec.as_object().unwrap();
                for (k, v) in obj {
                    let adr_v = block
                        .iter()
                        .find(|(bk, _)| bk == k)
                        .map(|(_, bv)| bv.as_str())
                        .unwrap_or_else(|| panic!("{tag} 字段 {k} 不在 ADR 块里"));
                    assert_eq!(json_text(v), adr_v, "{tag} {k}");
                }
                if tag == "funding-record" {
                    assert_eq!(obj.len(), block.len(), "funding-record 字段集与 ADR 一致");
                }
            }
        }
    }

    /// #1601 源码守卫：生产段（各文件首个 `#[cfg(test)] mod` 之前、去掉 `//` 注释）不得再出现
    /// 资金/级别常数名或 1e6 占位。把读取改回常数的变异体在此变红。
    #[test]
    fn production_sources_hold_no_funding_constants() {
        const SOURCES: &[(&str, &str)] = &[
            ("trading/types.rs", include_str!("trading/types.rs")),
            ("trading/runner.rs", include_str!("trading/runner.rs")),
            (
                "trading/positional.rs",
                include_str!("trading/positional.rs"),
            ),
            (
                "trading/recursive_position.rs",
                include_str!("trading/recursive_position.rs"),
            ),
            (
                "trading/unified_necessity.rs",
                include_str!("trading/unified_necessity.rs"),
            ),
            (
                "trading/axiom_voice.rs",
                include_str!("trading/axiom_voice.rs"),
            ),
            (
                "trading/unified_recursive.rs",
                include_str!("trading/unified_recursive.rs"),
            ),
            (
                "trading/isolated_fugue.rs",
                include_str!("trading/isolated_fugue.rs"),
            ),
            (
                "trading/recursive_nested_fugue.rs",
                include_str!("trading/recursive_nested_fugue.rs"),
            ),
            (
                "trading/dual_voice.rs",
                include_str!("trading/dual_voice.rs"),
            ),
            (
                "trading/unified_voice.rs",
                include_str!("trading/unified_voice.rs"),
            ),
            (
                "trading/nested_fugue.rs",
                include_str!("trading/nested_fugue.rs"),
            ),
            (
                "trading/nested_interval_fugue.rs",
                include_str!("trading/nested_interval_fugue.rs"),
            ),
            (
                "trading/positioning_chain_fugue.rs",
                include_str!("trading/positioning_chain_fugue.rs"),
            ),
            (
                "trading/positional_fusion.rs",
                include_str!("trading/positional_fusion.rs"),
            ),
            ("spiral/engine.rs", include_str!("spiral/engine.rs")),
            ("spiral/ffi.rs", include_str!("spiral/ffi.rs")),
            ("spiral/params.rs", include_str!("spiral/params.rs")),
            ("fugue_v3/operate.rs", include_str!("fugue_v3/operate.rs")),
            ("fugue_v3/engine.rs", include_str!("fugue_v3/engine.rs")),
            ("fugue_v3/ffi.rs", include_str!("fugue_v3/ffi.rs")),
            ("fugue_v3/mod.rs", include_str!("fugue_v3/mod.rs")),
            (
                "recursive_t/t_engine.rs",
                include_str!("recursive_t/t_engine.rs"),
            ),
            (
                "recursive_t/stream.rs",
                include_str!("recursive_t/stream.rs"),
            ),
            (
                "recursive_t/rec_stream.rs",
                include_str!("recursive_t/rec_stream.rs"),
            ),
            ("recursive_t/ffi.rs", include_str!("recursive_t/ffi.rs")),
            ("theta_v0/ffi.rs", include_str!("theta_v0/ffi.rs")),
            (
                "theta_v0/nautilus/theta_strategy.rs",
                include_str!("theta_v0/nautilus/theta_strategy.rs"),
            ),
            (
                "theta_v0/nautilus/theta_pi_strategy.rs",
                include_str!("theta_v0/nautilus/theta_pi_strategy.rs"),
            ),
            (
                "theta_v0/nautilus/backtest_engine.rs",
                include_str!("theta_v0/nautilus/backtest_engine.rs"),
            ),
            ("lib.rs", include_str!("lib.rs")),
        ];
        const FORBIDDEN: &[&str] = &[
            "INITIAL_CAPITAL",
            "DEFAULT_OP_LADDER",
            "SUB_EXPIRY",
            "INITIAL_NAV_PLACEHOLDER",
            "1_000_000",
            "1.0e6",
            "1e6",
        ];
        let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
        let mut hits = Vec::new();
        for (path, text) in SOURCES {
            let prod = text.split("\n#[cfg(test)]\nmod ").next().unwrap();
            for (n, line) in prod.lines().enumerate() {
                let code = line.split("//").next().unwrap();
                for tok in FORBIDDEN {
                    for (i, _) in code.match_indices(tok) {
                        let before = code[..i].chars().next_back();
                        let after = code[i + tok.len()..].chars().next();
                        if !before.is_some_and(is_ident) && !after.is_some_and(is_ident) {
                            hits.push(format!("{path}:{}: {tok}", n + 1));
                        }
                    }
                }
            }
        }
        assert!(hits.is_empty(), "生产段残留资金/级别常数：{hits:#?}");
    }
}
