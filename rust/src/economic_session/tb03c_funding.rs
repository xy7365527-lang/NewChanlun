//! #1406 C8：`tb03c-book/2` 资金合同——ADR 0040 签署记录接入书账开局批。
//!
//! 只对 manifest 冻结 `book_revision = tb03c-book/2` 的新域生效；`/1` 与缺省域不读本模块的任何字段
//! （出现即拒，#799 无回落）。口径：
//! - manifest 顶层 `funding` 冻结 funding-record（ADR 0040 签署记录，ADR 0029:31 八字段形状 + 可选 scope，
//!   `cap` 承载 Q_total）与 `margin_leverage`（值 + 出处）；每个重 `chongs[i].funding` 冻结本重
//!   ⟨Q, policy-record⟩（ADR 0034 裁定一量档记录形状）。缺任一件 typed 拒绝，不默认填。
//! - 数值只从 manifest 读（ADR 0040 通则 Q5）；本文件只有政策族标识，没有签署数值。
//! - 开局批（ADR 0040 裁定四第 4 步；ADR 0031 裁定一）：恰一笔 principal_contribution，
//!   amount = ⌈Q / L⌉，authorization_ref = `<funding policy_id>@<profile_id>`，
//!   authorization_version = funding-record.version，currency = funding-record.currency。
//! - 开重复核 Q ≤ C(ℓ)（ADR 0034 裁定四；ADR 0040 裁定四第 5 步）；C(ℓ) = Unknown ⟹
//!   AwaitingPolicy{capacity, level:ℓ}，未知不是无限（E Pending 同口径）。原件 known_at 早于签署生效日
//!   ⟹ PolicyNotYetEffective，超过任一签署有效期 ⟹ PolicyExpired。
//! - 非开局追加批：ADR 0040 裁定四末段「改总投入……都签新版本」⟹ AwaitingPolicy{funding_revision}。
//! - q_rebase：「Q 变大不自动改现役 Q（输出 AwaitingPolicy{q_rebase, (S,ℓ)}）」。
use super::book::{rules, target, Rules};
use super::quantity_calibration::{self as qc, Readiness};
use crate::session_protocol::{parse_canonical_i64, positive, required};
use serde_json::{json, Value};

/// ADR 0040 签署记录的政策族标识（标识不是数值；版本号从记录本身读）。
pub(crate) const FUNDING_POLICY_ID: &str = "#1557/funding-authorization/v1";
pub(crate) const QUANTITY_TIER_POLICY_ID: &str = "#1355/quantity-tier-policy/v1";
const FUNDING_FIELDS: [&str; 9] = [
    "profile_id",
    "policy_id",
    "version",
    "supersedes",
    "cap",
    "currency",
    "authorizer",
    "authorized_at",
    "valid_through",
];
const POLICY_FIELDS: [&str; 13] = [
    "profile_id",
    "policy_id",
    "version",
    "supersedes",
    "symbol",
    "operation_level",
    "q_ref",
    "tiers",
    "capacities",
    "frequency_evidence_ref",
    "authorizer",
    "authorized_at",
    "valid_through",
];
const CAPACITY_FIELDS: [&str; 4] = ["level", "C", "source", "valid_through"];

pub(crate) struct Chong {
    id: String,
    symbol: String,
    op_level: String,
    level: String,
    q: i64,
    policy: Value,
    /// `/3` 才有：本重冻结数量定标（#1715 I1-Q）；`/2` 为 None，行为逐字不变。
    quantity: Option<Quantity>,
}
struct Quantity {
    funding: qc::ChongFunding,
    /// [现役初版, 获批 q_rebase 新版…]：后版 supersedes 前版；每版各指定一份选价前封存快照。
    versions: Vec<Version>,
}
struct Version {
    policy: qc::CalibrationPolicy,
    snapshot_id: String,
}
impl Quantity {
    /// q_rebase 原件的 `policy_ref` 只认已冻结的获批新版本（`<政策族>@<版本>`）；初版不能被重新激活。
    fn rebase_version(&self, rebase: &Value) -> Option<&Version> {
        self.versions[1..].iter().find(|v| {
            rebase["policy_ref"]
                == format!(
                    "{}@{}",
                    qc::QUANTITY_CALIBRATION_POLICY_ID,
                    v.policy.version
                )
                .as_str()
        })
    }
}
pub(crate) struct Contract {
    rules: Rules,
    record: Value,
    leverage: i64,
    chongs: Vec<Chong>,
    snapshots: Vec<qc::Snapshot>,
}

/// 字段集合恰为 `keys`（`scope` 可选，ADR 0040 签署记录 evidence_only 档）。
fn exact(v: &Value, keys: &[&str], optional: &[&str], what: &str) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("SchemaUnsupported：{what}须对象"))?;
    if keys.iter().any(|k| !o.contains_key(*k))
        || o.keys()
            .any(|k| !keys.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(format!("SchemaUnsupported：{what}字段不完整或多余"));
    }
    Ok(())
}
/// 签署日期文本 `YYYY-MM-DD`；同形文本的字典序即日期序。
fn date<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    let s = required(v, k)?;
    let b = s.as_bytes();
    let ok = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if !ok {
        return Err(format!("InvalidDomain：{k} 须为 YYYY-MM-DD"));
    }
    Ok(s)
}

/// #1621：将 Unix epoch 纳秒时间戳（整数字符串）转换为 YYYY-MM-DD 格式。
/// 用于与签署记录的 authorized_at/valid_through（YYYY-MM-DD）比较。
///
/// 时间来源：Config::clock() System 模式返回 Unix epoch 纳秒
/// (SystemTime::now().duration_since(std::time::UNIX_EPOCH).as_nanos())，
/// 参见 store.rs:314-317。
///
/// 实现：精确 Proleptic Gregorian 历法（基于 Howard Hinnant 的 civil_from_days 算法）。
/// 支持负数（1970-01-01 之前）、闰年、月底边界。
fn ns_to_date(ns_str: &str) -> Result<String, String> {
    const NS_PER_DAY: i64 = 86_400_000_000_000;

    let ns: i64 = ns_str
        .parse()
        .map_err(|_| format!("InvalidDomain：first_known_ns 须为整数，得 {ns_str}"))?;

    // 转换为天数（向下取整，负数正确处理）
    let days_since_epoch = if ns >= 0 {
        ns / NS_PER_DAY
    } else {
        (ns - NS_PER_DAY + 1) / NS_PER_DAY
    };

    // civil_from_days 算法（Howard Hinnant，Proleptic Gregorian）
    // 输入：Unix epoch (1970-01-01) 后的天数
    // 输出：(year, month, day)
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };

    Ok(format!("{:04}-{:02}-{:02}", y, m, d))
}

fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    let s = required(v, k)?;
    if s.is_empty() {
        return Err(format!("InvalidDomain：{k} 不得为空"));
    }
    Ok(s)
}
/// manifest 的 operation_level（非负整数文本 n）在签署记录里写作 `Ln`。
pub(crate) fn level_name(op_level: &str) -> String {
    format!("L{op_level}")
}
fn has_funding(manifest: &Value) -> bool {
    manifest.get("funding").is_some()
        || manifest["chongs"]
            .as_array()
            .is_some_and(|a| a.iter().any(|c| c.get("funding").is_some()))
}

/// `/3` 逐重定标记录：金额只取本重 `funding.Q`（不读总 cap）；身份与币种在加载边界核，
/// 锚由激活时的持久请求或版本签署派生，所以定标结果在动作处计算（`Contract::readiness`）。
fn parse_quantity(
    cf: &Value,
    record: &Value,
    funding: qc::ChongFunding,
) -> Result<Quantity, String> {
    let mut versions = vec![Version {
        policy: qc::parse_policy(&cf["quantity_calibration"])?,
        snapshot_id: text(cf, "quote_snapshot")?.to_owned(),
    }];
    if let Some(rebases) = cf.get("quantity_rebases") {
        for r in rebases
            .as_array()
            .ok_or("SchemaUnsupported：quantity_rebases 须数组")?
        {
            exact(
                r,
                &["quantity_calibration", "quote_snapshot"],
                &[],
                "q_rebase 定标版本 ",
            )?;
            versions.push(Version {
                policy: qc::parse_policy(&r["quantity_calibration"])?,
                snapshot_id: text(r, "quote_snapshot")?.to_owned(),
            });
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, v) in versions.iter().enumerate() {
        if !seen.insert(v.policy.version.as_str()) {
            return Err("IdentityConflict：数量定标版本重复".into());
        }
        if i > 0 && v.policy.supersedes != versions[i - 1].policy.version {
            return Err(format!(
                "IdentityConflict：q_rebase 定标版本 {} 须直接继承 {}",
                v.policy.version,
                versions[i - 1].policy.version
            ));
        }
        qc::check_binding(&v.policy.binding, &funding)?;
        if let qc::QuantityBasis::FrozenQuote(fq) = &v.policy.basis {
            if fq.currency_scale.from != required(record, "currency")? {
                return Err(
                    "IdentityConflict：定标 currency_scale.from 须等于 funding-record 币种".into(),
                );
            }
        }
    }
    Ok(Quantity { funding, versions })
}

pub(super) fn validate_config(manifest: &Value) -> Result<(), String> {
    Contract::of(manifest).map(|_| ())
}

impl Chong {
    pub(crate) fn symbol(&self) -> &str {
        &self.symbol
    }
    pub(crate) fn level(&self) -> &str {
        &self.level
    }
    /// policy-record 里本级（ℓ）的容量条目。
    fn capacity(&self) -> Option<&Value> {
        self.policy["capacities"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["level"] == self.level.as_str()))
    }
    /// 已签的 C(ℓ)；缺条目或 Unknown ⟹ None（未知不是无限，ADR 0034 裁定四）。
    fn signed_capacity(&self) -> Option<&str> {
        self.capacity()
            .and_then(|x| x["C"].as_str())
            .filter(|c| *c != "Unknown")
    }
    fn signed_cap(&self) -> Result<i64, String> {
        match self.signed_capacity() {
            Some(v) => positive(v, "C(ℓ)"),
            None => Err(format!(
                "AwaitingPolicy：capacity level={}；C(ℓ) 未签署（Unknown 不是无限），不建收据",
                self.level
            )),
        }
    }
}

impl Contract {
    /// `/2` ⟹ 解析并核验冻结的资金三件；`/1` 与缺省 ⟹ None（且不得夹带资金字段）。
    pub(crate) fn of(manifest: &Value) -> Result<Option<Self>, String> {
        let revision = rules(manifest)?;
        if !matches!(revision, Rules::Tb03cV2 | Rules::Tb03cV3) {
            if has_funding(manifest) {
                return Err(
                    "SchemaUnsupported：资金合同只属 tb03c-book/2 域；本域版本不读它（#799 无回落）"
                        .into(),
                );
            }
            return Ok(None);
        }
        let f = manifest
            .get("funding")
            .filter(|v| v.is_object())
            .ok_or("MissingDependency：tb03c-book/2 缺资金签署 funding（ADR 0040 签署记录）")?;
        let record = f
            .get("funding_record")
            .filter(|v| v.is_object())
            .ok_or("MissingDependency：tb03c-book/2 缺 funding-record（ADR 0040 签署记录）")?;
        let leverage = f.get("margin_leverage").filter(|v| v.is_object()).ok_or(
            "MissingDependency：tb03c-book/2 缺 margin_leverage（值+出处，ADR 0040 裁定三）",
        )?;
        let v3 = revision == Rules::Tb03cV3;
        if v3 {
            exact(
                f,
                &["funding_record", "margin_leverage", "quote_snapshots"],
                &[],
                "funding ",
            )?;
        } else {
            exact(f, &["funding_record", "margin_leverage"], &[], "funding ")?;
        }
        exact(record, &FUNDING_FIELDS, &["scope"], "funding-record ")?;
        if record["policy_id"] != FUNDING_POLICY_ID {
            return Err(
                "IdentityConflict：funding-record 政策族须为 #1557/funding-authorization/v1".into(),
            );
        }
        for k in [
            "profile_id",
            "version",
            "supersedes",
            "currency",
            "authorizer",
        ] {
            text(record, k)?;
        }
        positive(required(record, "cap")?, "funding cap")?;
        date(record, "authorized_at")?;
        date(record, "valid_through")?;
        exact(leverage, &["value", "source"], &[], "margin_leverage ")?;
        let l = positive(required(leverage, "value")?, "margin_leverage")?;
        text(leverage, "source")?;
        let q_ref = format!(
            "{}@{}",
            required(record, "policy_id")?,
            required(record, "profile_id")?
        );
        let snapshots = if v3 {
            let snaps = f["quote_snapshots"]
                .as_array()
                .ok_or("SchemaUnsupported：quote_snapshots 须数组（选价前封存）")?
                .iter()
                .map(qc::parse_snapshot)
                .collect::<Result<Vec<_>, String>>()?;
            let mut ids = std::collections::BTreeSet::new();
            if snaps.iter().any(|s| !ids.insert(s.snapshot_id.clone())) {
                return Err("IdentityConflict：quote_snapshots 快照 ID 重复".into());
            }
            snaps
        } else {
            Vec::new()
        };
        let mut chongs = Vec::new();
        let mut total = 0i64;
        for c in manifest["chongs"]
            .as_array()
            .ok_or("InvalidDomain：重路由为空")?
        {
            let id = required(c, "chong_id")?;
            let cf = c.get("funding").filter(|v| v.is_object()).ok_or_else(|| {
                format!("MissingDependency：tb03c-book/2 重 {id} 缺 ⟨Q, policy-record⟩")
            })?;
            if v3 {
                exact(
                    cf,
                    &[
                        "Q",
                        "policy_record",
                        "quantity_calibration",
                        "quote_snapshot",
                    ],
                    &["quantity_rebases"],
                    "重资金 ",
                )?;
            } else {
                exact(cf, &["Q", "policy_record"], &[], "重资金 ")?;
            }
            let q = positive(required(cf, "Q")?, "Q")?;
            total = total.checked_add(q).ok_or("InvalidDomain：ΣQ 溢出")?;
            let p = &cf["policy_record"];
            exact(p, &POLICY_FIELDS, &["scope"], "policy-record ")?;
            if p["policy_id"] != QUANTITY_TIER_POLICY_ID {
                return Err(
                    "IdentityConflict：policy-record 政策族须为 #1355/quantity-tier-policy/v1"
                        .into(),
                );
            }
            for k in [
                "version",
                "supersedes",
                "authorizer",
                "frequency_evidence_ref",
            ] {
                text(p, k)?;
            }
            date(p, "authorized_at")?;
            date(p, "valid_through")?;
            let symbol = required(c, "symbol")?;
            let op_level = required(c, "operation_level")?;
            let level = level_name(op_level);
            if p["profile_id"] != record["profile_id"]
                || p["symbol"] != symbol
                || p["operation_level"] != level.as_str()
            {
                return Err(format!(
                    "IdentityConflict：重 {id} 的 policy-record 不属本 profile 的 (S,ℓ)"
                ));
            }
            if p["q_ref"] != q_ref.as_str() {
                return Err(format!(
                    "IdentityConflict：重 {id} 的 q_ref 不指向本 funding-record（ADR 0040 裁定四第 5 步）"
                ));
            }
            let caps = p["capacities"]
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or("SchemaUnsupported：policy-record capacities 须非空数组")?;
            let mut levels = std::collections::BTreeSet::new();
            for cap in caps {
                exact(cap, &CAPACITY_FIELDS, &[], "capacity ")?;
                if !levels.insert(text(cap, "level")?) {
                    return Err("IdentityConflict：capacities 级别重复".into());
                }
                if cap["C"] != "Unknown" {
                    positive(required(cap, "C")?, "C(ℓ)")?;
                }
                text(cap, "source")?;
                date(cap, "valid_through")?;
            }
            let quantity = if v3 {
                Some(parse_quantity(
                    cf,
                    record,
                    qc::ChongFunding {
                        chong_id: id.to_owned(),
                        profile_id: required(record, "profile_id")?.to_owned(),
                        funding_version: required(record, "version")?.to_owned(),
                        symbol: symbol.to_owned(),
                        operation_level: level.clone(),
                        q_ref: q_ref.clone(),
                        q_amount: q,
                    },
                )?)
            } else {
                None
            };
            chongs.push(Chong {
                id: id.to_owned(),
                symbol: symbol.to_owned(),
                op_level: op_level.to_owned(),
                level,
                q,
                policy: p.clone(),
                quantity,
            });
        }
        if total > positive(required(record, "cap")?, "funding cap")? {
            return Err("InvalidDomain：各重 Q 之和超过 funding-record 总投入".into());
        }
        Ok(Some(Self {
            rules: revision,
            record: record.clone(),
            leverage: l,
            chongs,
            snapshots,
        }))
    }

    /// 本合同所属书账版本（`/2` 或 `/3`）。
    pub(crate) fn rules(&self) -> Rules {
        self.rules
    }
    pub(crate) fn chong(&self, id: &str) -> Result<&Chong, String> {
        self.chongs
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("InvalidDomain：资金合同无重 {id}"))
    }
    /// 本重某定标版本对持久请求 `record`（E 已提交事实）的就绪判定；E 等待与 B 激活同一函数。
    fn readiness(&self, c: &Chong, v: &Version, record: &Value) -> Result<Readiness, String> {
        let q = c
            .quantity
            .as_ref()
            .ok_or("StorageUnavailable：/3 重缺数量定标")?;
        qc::calibrate(
            &v.policy,
            &q.funding,
            &qc::ActionFact::of(record)?,
            &v.snapshot_id,
            &self.snapshots,
        )
    }

    /// 新激活的金额域条件（ADR 0040 裁定四第 5 步与 :56/:59；ADR 0034 裁定四）：可审计 B 观察时刻落在
    /// funding-record、policy-record 与本级容量条目的签署窗口内，且已签 C_amount(ℓ) ≥ 本重 Q_amount。
    /// 普通开局、`/3` 转入目标与获批 q_rebase 都是新激活，共用本条；已开重的后续事实不经此门。返回 C_amount。
    fn activation_terms(&self, c: &Chong, record: &Value) -> Result<i64, String> {
        // #1621：known_at 绑定可审计观察时间（record["b_observation"]["first_known_ns"]），不再使用原件自报日期。
        // 跨域不变式：S/E/B/X 时刻先后由同一观察时间源锁定（与 tb03c_transfer.rs:367 转移目标口径一致）。
        let first_known_ns = record["b_observation"]
            .get("first_known_ns")
            .and_then(|v| v.as_str())
            .ok_or("MissingDependency：b_observation 缺 first_known_ns")?;
        let known_at = ns_to_date(first_known_ns)?;
        // ADR 0040:56/:59：批次只能引用已生效的签署（「只作用于生效后」）——下界。
        for (what, from) in [
            ("funding-record", self.record["authorized_at"].as_str()),
            ("policy-record", c.policy["authorized_at"].as_str()),
        ] {
            if from.is_some_and(|f| known_at.as_str() < f) {
                return Err(format!(
                    "PolicyNotYetEffective：原件 known_at {known_at} 早于 {what} 签署生效日"
                ));
            }
        }
        for (what, through) in [
            ("funding-record", self.record["valid_through"].as_str()),
            ("policy-record", c.policy["valid_through"].as_str()),
            (
                "capacity",
                c.capacity().and_then(|x| x["valid_through"].as_str()),
            ),
        ] {
            if through.is_some_and(|t| known_at.as_str() > t) {
                return Err(format!(
                    "PolicyExpired：原件 known_at {known_at} 超过 {what} 有效期"
                ));
            }
        }
        let cap = c.signed_cap()?;
        if c.q > cap {
            return Err(format!(
                "InvalidDomain：Q {} 超过已签 C({}) {cap}（ADR 0034 裁定四）",
                c.q, c.level
            ));
        }
        Ok(cap)
    }

    /// 名义资金金额是 funding-record 币种的名义整数（ADR 0040:34），书账金额须与之同一尺度：原件
    /// `units.money` 以该币种本身计、quantum 1；同一整数换个金额 quantum 就是另一名义值（独审 26AF R1）。
    fn money_scale(&self, units: &Value) -> Result<(), String> {
        let money = &units["money"];
        let currency = &self.record["currency"];
        if money["currency"] != *currency || money["unit"] != *currency || money["quantum"] != "1" {
            return Err(format!(
                "IdentityConflict：原件金额 {}（unit {}，quantum {}）不是 funding-record 币种 {currency} 的名义整数（quantum 1，ADR 0040:34），资金与书账不同尺度",
                money["currency"], money["unit"], money["quantum"]
            ));
        }
        Ok(())
    }

    /// `/3` 新激活闸（#1715 I1-Q）：普通开局、转入目标与获批 q_rebase 共用同一判定——持久请求派生锚 →
    /// 定标就绪（与 E 等待同一函数）→ 原件金额与数量量纲 → fixed_q = Q_units → 金额域签署窗口与
    /// Q_amount ≤ C_amount → 同维 Q_units ≤ C_units。返回 (定标投影, C_amount, C_units)。
    fn activate(
        &self,
        c: &Chong,
        v: &Version,
        fixed_q: i64,
        units: &Value,
        record: &Value,
    ) -> Result<(Value, i64, Option<i64>), String> {
        let readiness = self.readiness(c, v, record)?;
        let Some(cal) = readiness.ready() else {
            return Err(readiness.reason().unwrap_or_default());
        };
        self.money_scale(units)?;
        cal.check_units(units)?;
        if fixed_q != cal.q_units {
            return Err(format!(
                "IdentityConflict：fixed_q 须等于冻结 Q_units {}（ADR 0049 Q4）",
                cal.q_units
            ));
        }
        let cap = self.activation_terms(c, record)?;
        let c_units = cal.capacity_units(cap)?;
        if let Some(cu) = c_units {
            if cal.q_units > cu {
                return Err(format!(
                    "InvalidDomain：Q_units {} 超过同维 C_units({}) {cu}",
                    cal.q_units, c.level
                ));
            }
        }
        Ok((cal.to_json(), cap, c_units))
    }

    /// 转入目标开局（`/3`）：本金来源仍是转移（ADR 0034 裁定六，不核开局批），数量走同一激活闸。
    /// `/2` 返回 None，书账形状逐字不变。
    pub(crate) fn check_transfer_opening(
        &self,
        event: &Value,
        record: &Value,
        units: &Value,
    ) -> Result<Option<Value>, String> {
        let c = self.chong(target(event)?)?;
        let Some(q) = &c.quantity else {
            return Ok(None);
        };
        let o = &event["transfer_opening"];
        if o["symbol"] != c.symbol.as_str() || o["op_level"] != c.op_level.as_str() {
            return Err("IdentityConflict：转入开局 (S,ℓ) 与资金合同冻结的重不符".into());
        }
        let fixed_q = positive(required(o, "fixed_q")?, "transfer fixed_q")?;
        let (cal, cap, c_units) = self.activate(c, &q.versions[0], fixed_q, units, record)?;
        let mut capacity = json!({"level": c.level, "C": cap.to_string()});
        if let Some(cu) = c_units {
            capacity["C_units"] = json!(cu.to_string());
        }
        Ok(Some(
            json!({"funding_version": self.record["version"], "quantity_calibration": cal, "capacity": capacity}),
        ))
    }

    /// q_rebase（`/3`）：只有 manifest 已冻结的获批新版本、且直接继承书账现役定标版本时才重定标；
    /// 锚取本次持久 q_rebase 请求或新版本签署。返回新的 fixed_q 与书账资金投影。
    pub(crate) fn check_q_rebase(
        &self,
        event: &Value,
        record: &Value,
        units: &Value,
        book: &Value,
    ) -> Result<(String, Value), String> {
        let c = self.chong(target(event)?)?;
        let unsigned = || {
            format!(
                "AwaitingPolicy：q_rebase (S,ℓ)=({},{})；现役 Q 不自动改写，须签新版本",
                c.symbol, c.level
            )
        };
        let Some(v) = c
            .quantity
            .as_ref()
            .and_then(|q| q.rebase_version(&event["q_rebase"]))
        else {
            return Err(unsigned());
        };
        let funding = book
            .get("funding")
            .filter(|f| f.get("quantity_calibration").is_some())
            .ok_or("AwaitingDependency：q_rebase 须在已冻结定标的开局之后")?;
        if funding["quantity_calibration"]["version"] != v.policy.supersedes.as_str() {
            return Err(format!(
                "IdentityConflict：q_rebase 新版本 {} 不直接继承书账现役定标版本",
                v.policy.version
            ));
        }
        let new_q = positive(required(&event["q_rebase"], "new_q")?, "q_rebase.new_q")?;
        let (cal, cap, c_units) = self.activate(c, v, new_q, units, record)?;
        let mut out = funding.clone();
        let mut superseded = funding
            .get("superseded_calibrations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        superseded.push(funding["quantity_calibration"].clone());
        out["superseded_calibrations"] = Value::Array(superseded);
        out["quantity_calibration"] = cal;
        out["capacity"]["C"] = json!(cap.to_string());
        match c_units {
            Some(cu) => out["capacity"]["C_units"] = json!(cu.to_string()),
            None => {
                if let Some(m) = out["capacity"].as_object_mut() {
                    m.remove("C_units");
                }
            }
        }
        Ok((new_q.to_string(), out))
    }

    fn authorization_ref(&self) -> String {
        format!(
            "{}@{}",
            self.record["policy_id"].as_str().unwrap_or(""),
            self.record["profile_id"].as_str().unwrap_or("")
        )
    }

    /// 开局批核验（ADR 0040 裁定四第 4/5 步）。通过即返回写入书账的资金出处；失败不改任何状态。
    /// `record["b_observation"]["first_known_ns"]` 为可审计观察时间（#1621），原件 `known_at` 不再用于时效判断。
    pub(crate) fn check_opening(
        &self,
        event: &Value,
        record: &Value,
        units: &Value,
    ) -> Result<Value, String> {
        let c = self.chong(target(event)?)?;
        let id = &event["book_identity"];
        if id["symbol"] != c.symbol.as_str() || id["op_level"] != c.op_level.as_str() {
            return Err("IdentityConflict：开局 (S,ℓ) 与资金合同冻结的重不符".into());
        }
        let batches: Vec<&Value> = event["known_cost_components"]
            .as_array()
            .ok_or("SchemaUnsupported：known_cost_components须数组")?
            .iter()
            .filter(|x| x["kind"] == "principal_contribution")
            .collect();
        let batch =
            match batches.as_slice() {
                [one] => *one,
                [] => return Err(
                    "MissingDependency：tb03c-book/2 开局须带本重开局批（ADR 0040 裁定四第 4 步）"
                        .into(),
                ),
                _ => return Err("IdentityConflict：tb03c-book/2 开局只收一笔开局批".into()),
            };
        if batch["authorization_ref"] != self.authorization_ref().as_str() {
            return Err(
                "IdentityConflict：开局批 authorization_ref 不指向冻结的 funding-record".into(),
            );
        }
        if batch["authorization_version"] != self.record["version"] {
            return Err(
                "IdentityConflict：开局批 authorization_version 与冻结的 funding-record 版本不符"
                    .into(),
            );
        }
        if batch["currency"] != self.record["currency"] {
            return Err("InvalidDomain：开局批币种与 funding-record 不符".into());
        }
        let (cap, quantity) = match &c.quantity {
            None => (self.activation_terms(c, record)?, None),
            Some(q) => {
                let (cal, cap, c_units) = self.activate(
                    c,
                    &q.versions[0],
                    positive(required(id, "fixed_q")?, "fixed_q")?,
                    units,
                    record,
                )?;
                (cap, Some((cal, c_units)))
            }
        };
        let expected =
            c.q.checked_add(self.leverage - 1)
                .ok_or("InvalidDomain：⌈Q/L⌉ 溢出")?
                / self.leverage;
        if parse_canonical_i64(required(batch, "canonical_amount")?, "canonical_amount")?
            != expected
        {
            return Err(format!(
                "IdentityConflict：开局批金额 ≠ ⌈Q/L⌉ = {expected}（ADR 0040 裁定四第 4 步）"
            ));
        }
        let mut out = json!({"authorization_ref":self.authorization_ref(),"funding_version":self.record["version"],
            "Q":c.q.to_string(),"margin_leverage":self.leverage.to_string(),"opening_batch":expected.to_string(),
            "policy_record":{"policy_id":c.policy["policy_id"],"version":c.policy["version"],"q_ref":c.policy["q_ref"]},
            "capacity":{"level":c.level,"C":cap.to_string()}});
        if let Some((cal, c_units)) = quantity {
            // 数量域容量与 Q_units 同维比较，且绑定同一冻结报价（ADR 0034 裁定四；ADR 0049）。
            if let Some(cu) = c_units {
                out["capacity"]["C_units"] = json!(cu.to_string());
            }
            out["quantity_calibration"] = cal;
        }
        Ok(out)
    }
}

/// `/2` 下 E 侧必须显式等待的签署缺口；`/1` 与缺省返回 None（原文案逐字不变）。
/// `record` 是 E 已提交事实：`/3` 的定标锚由它（fact_key、first_known_ns）派生，与 B 激活同一判定。
pub(super) fn pending_reason(manifest: &Value, record: &Value) -> Result<Option<Value>, String> {
    if !matches!(rules(manifest)?, Rules::Tb03cV2 | Rules::Tb03cV3) {
        return Ok(None);
    }
    let event = &record["event"];
    let rebase = event.get("q_rebase").is_some();
    // None = 不适用（非 `/3`、非开局/转入/q_rebase，或 q_rebase 无获批新版本）；Some(None) = 已就绪。
    type Wait = Option<(&'static str, &'static str)>;
    let quantity_wait = || -> Result<Option<Wait>, String> {
        let opening =
            event.get("observed_existing_book").is_some() || super::tb03c_transfer::is_in(event);
        if !opening && !rebase {
            return Ok(None);
        }
        let Some(contract) = Contract::of(manifest)? else {
            return Ok(None);
        };
        let Some(c) = target(event).ok().and_then(|id| contract.chong(id).ok()) else {
            return Ok(None);
        };
        let Some(q) = &c.quantity else {
            return Ok(None);
        };
        let version = if rebase {
            match q.rebase_version(&event["q_rebase"]) {
                Some(v) => v,
                None => return Ok(None),
            }
        } else {
            &q.versions[0]
        };
        Ok(Some(contract.readiness(c, version, record)?.wait()))
    };
    let opening_without_capacity = || -> Result<bool, String> {
        if event.get("observed_existing_book").is_none() {
            return Ok(false);
        }
        let Some(contract) = Contract::of(manifest)? else {
            return Ok(false);
        };
        Ok(target(event)
            .ok()
            .and_then(|id| contract.chong(id).ok())
            .is_some_and(|c| c.signed_capacity().is_none()))
    };
    let quantity = quantity_wait()?;
    let mut kind = "AwaitingPolicy";
    let slot = if rebase {
        match quantity {
            // #1715：获批新版本已就绪的 q_rebase 不挂签署等待。
            Some(None) => return Ok(None),
            Some(Some((k, slot))) => {
                kind = k;
                slot
            }
            None => "q_rebase",
        }
    } else if is_appended_contribution(event) {
        "funding_revision"
    } else if let Some(Some((k, slot))) = quantity {
        // #1715：`/3` 定标未签或缺合格报价 ⟹ E 侧 typed 等待，与 B 拒绝同口径。
        kind = k;
        slot
    } else if opening_without_capacity()? {
        // 评审 M3 / 根裁定 OQ-3：C(ℓ) 未签署是 E 侧 typed 等待，与 B 拒绝同口径。
        "capacity"
    } else {
        return Ok(None);
    };
    let route = target(event).ok().and_then(|id| {
        manifest["chongs"]
            .as_array()
            .and_then(|a| a.iter().find(|c| c["chong_id"] == id))
    });
    let (symbol, level) = match route {
        Some(r) => (
            r["symbol"].clone(),
            r["operation_level"]
                .as_str()
                .map_or(Value::Null, |l| json!(level_name(l))),
        ),
        None => (Value::Null, Value::Null),
    };
    Ok(Some(
        json!({"kind":kind,"slot":slot,"symbol":symbol,"level":level}),
    ))
}

/// 非开局原件夹带本金批 = 追加投入；`/2` 下须先签新版本（ADR 0040 裁定四末段）。
pub(super) fn is_appended_contribution(event: &Value) -> bool {
    event.get("observed_existing_book").is_none()
        && !super::tb03c_transfer::is_in(event)
        && event["known_cost_components"]
            .as_array()
            .is_some_and(|a| a.iter().any(|c| c["kind"] == "principal_contribution"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1621：ns_to_date() 精确对拍——与 Python 标准库生成的 golden 值逐项校验。
    /// 覆盖：Unix epoch、负数、月底、闰年、政策边界。
    #[test]
    fn ns_to_date_parity_with_python_stdlib() {
        // Golden 值由 tb03c_funding_oracle_date.py 生成（Python datetime 标准库）
        const ORACLE: &str = include_str!("tb03c_funding_oracle_date.json");
        let oracle: Value = serde_json::from_str(ORACLE).unwrap();
        let cases = oracle["test_cases"].as_array().unwrap();

        for case in cases {
            let desc = case["description"].as_str().unwrap();
            let ns_str = case["ns"].as_str().unwrap();
            let expected = case["expected_date"].as_str().unwrap();

            // 调用 Rust 实现
            let result = ns_to_date(ns_str);
            assert!(
                result.is_ok(),
                "{desc}: ns={ns_str} 应转换成功，实际错误 {:?}",
                result.unwrap_err()
            );
            let actual = result.unwrap();
            assert_eq!(
                actual, expected,
                "{desc}: ns={ns_str} 应得 {expected}，实际得 {actual}"
            );
        }

        // 额外负控：无效输入
        assert!(ns_to_date("not_a_number").is_err());
        assert!(ns_to_date("").is_err());
    }
}
