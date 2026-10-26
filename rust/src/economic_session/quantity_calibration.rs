//! #1715 I1-Q：逐重金额到冻结数量的定标（ADR 0049 Q4/Q3；ADR 0034 裁定一/四/五）。
//!
//! 金额与数量分维：`Q_amount` 只来自本重同签版派生金额（manifest.chongs[i].funding.Q），
//! `Q_units`、`tier_quantity`、`C_units` 是数量 quantum；q_ref 只作身份比对，不参与算术。
//! frozen_quote 只读锚上预封存快照中当时已知的报价，不读现价、不换锚兜底；signed_units 不依赖报价。
//! 本模块是纯函数：同一记录、锚与快照 ⟹ 同一结果，恢复/重试据此复现，不刷新定标。
use crate::session_protocol::{parse_canonical_i64, positive, required};
use serde_json::{json, Value};

pub(crate) const QUANTITY_CALIBRATION_POLICY_ID: &str = "#1610/quantity-calibration/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChongFunding {
    pub chong_id: String,
    pub profile_id: String,
    pub funding_version: String,
    pub symbol: String,
    pub operation_level: String,
    pub q_ref: String,
    pub q_amount: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnchorKind {
    ActivationRequest,
    ProfileSignature,
}
impl AnchorKind {
    fn policy_text(self) -> &'static str {
        match self {
            AnchorKind::ActivationRequest => "activation_request_asof",
            AnchorKind::ProfileSignature => "profile_signature_asof",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ratio {
    pub num: i64,
    pub den: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuoteSelector {
    pub source: String,
    pub market: String,
    pub instrument: String,
    pub price_field: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Signed<T> {
    Signed(T),
    AwaitingPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FrozenQuotePolicy {
    pub quote_time_policy: Signed<AnchorKind>,
    pub max_quote_age: Signed<i64>,
    pub currency_scale: Ratio,
    pub quantity_quantum: String,
    pub selector: QuoteSelector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QuantityBasis {
    SignedUnits { units: i64, signature_ref: String },
    FrozenQuote(FrozenQuotePolicy),
    AwaitingPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TierRatio {
    Signed { d: i64, w: Ratio },
    Unmeasured { d: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    pub profile_id: String,
    pub funding_version: String,
    pub symbol: String,
    pub operation_level: String,
    pub q_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CalibrationPolicy {
    pub version: String,
    pub supersedes: String,
    pub binding: Binding,
    pub basis: QuantityBasis,
    pub tiers: Vec<TierRatio>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub kind: AnchorKind,
    pub anchor_id: String,
    pub t_anchor: i64,
    pub snapshot_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Quote {
    pub quote_id: String,
    pub selector: QuoteSelector,
    pub t_occurred: i64,
    pub t_first_observed: i64,
    pub price: Ratio,
    pub original_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub snapshot_id: String,
    pub quotes: Vec<Quote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tier {
    pub d: i64,
    pub quantity: Signed<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedQuote {
    pub quote: Quote,
    pub anchor: Anchor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Calibration {
    pub chong_id: String,
    pub version: String,
    pub supersedes: String,
    pub binding: Binding,
    /// 已签定标方法的可审计投影（方法、签署引用、尺度、quantum、选择器）。
    pub basis: Value,
    pub q_amount: i64,
    pub q_units: i64,
    /// 余款 = residue_num / residue_den 个报价货币计量单位；留在资金账，不折成数量。
    pub residue: Option<Ratio>,
    pub selected: Option<SelectedQuote>,
    pub tiers: Vec<Tier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Readiness {
    Ready(Calibration),
    AwaitingPolicy(&'static str),
    AwaitingData(&'static str),
    NotComputable(&'static str),
}

/// 定标：失败返回 Err（身份冲突、非法输入、溢出）；未签与缺数据是 typed 未就绪，不激活。
pub(crate) fn calibrate(
    policy: &CalibrationPolicy,
    funding: &ChongFunding,
    anchor: Option<&Anchor>,
    snapshots: &[Snapshot],
) -> Result<Readiness, String> {
    check_binding(&policy.binding, funding)?;
    let (q_units, residue, selected) = match &policy.basis {
        QuantityBasis::AwaitingPolicy => return Ok(Readiness::AwaitingPolicy("quantity_basis")),
        QuantityBasis::SignedUnits { units, .. } => (*units, None, None),
        QuantityBasis::FrozenQuote(fq) => {
            let kind = match fq.quote_time_policy {
                Signed::Signed(k) => k,
                Signed::AwaitingPolicy => {
                    return Ok(Readiness::AwaitingPolicy("quote_time_policy"))
                }
            };
            let max_age = match fq.max_quote_age {
                Signed::Signed(a) => a,
                Signed::AwaitingPolicy => return Ok(Readiness::AwaitingPolicy("max_quote_age")),
            };
            let Some(anchor) = anchor else {
                return Ok(Readiness::AwaitingData("quote_anchor"));
            };
            if anchor.kind != kind {
                return Err(format!(
                    "IdentityConflict：锚类型与已签 quote_time_policy={} 不符；首次激活锚与签署锚不得混用",
                    kind.policy_text()
                ));
            }
            let Some(snapshot) = snapshots
                .iter()
                .find(|s| s.snapshot_id == anchor.snapshot_id)
            else {
                return Ok(Readiness::AwaitingData("quote_snapshot"));
            };
            let Some(quote) = select_quote(&fq.selector, max_age, anchor, snapshot)? else {
                return Ok(Readiness::AwaitingData("eligible_quote"));
            };
            let (units, residue) = to_units(funding.q_amount, &fq.currency_scale, &quote.price)?;
            (
                units,
                Some(residue),
                Some(SelectedQuote {
                    quote,
                    anchor: anchor.clone(),
                }),
            )
        }
    };
    if q_units == 0 {
        return Ok(Readiness::NotComputable("quantity_zero"));
    }
    let tiers = policy
        .tiers
        .iter()
        .map(|t| match t {
            TierRatio::Unmeasured { d } => Ok(Tier {
                d: *d,
                quantity: Signed::AwaitingPolicy,
            }),
            TierRatio::Signed { d, w } => Ok(Tier {
                d: *d,
                quantity: Signed::Signed(floor_mul(q_units, w)?),
            }),
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Readiness::Ready(Calibration {
        chong_id: funding.chong_id.clone(),
        version: policy.version.clone(),
        supersedes: policy.supersedes.clone(),
        binding: policy.binding.clone(),
        basis: basis_json(&policy.basis),
        q_amount: funding.q_amount,
        q_units,
        residue,
        selected,
        tiers,
    }))
}

fn check_binding(b: &Binding, f: &ChongFunding) -> Result<(), String> {
    for (what, signed, actual) in [
        ("profile", &b.profile_id, &f.profile_id),
        ("funding version", &b.funding_version, &f.funding_version),
        ("symbol", &b.symbol, &f.symbol),
        ("operation_level", &b.operation_level, &f.operation_level),
        ("q_ref", &b.q_ref, &f.q_ref),
    ] {
        if signed != actual {
            return Err(format!(
                "IdentityConflict：定标记录 {what}={signed} 与重 {} 的 {actual} 不符",
                f.chong_id
            ));
        }
    }
    Ok(())
}

/// 合格集合：选择器匹配、发生与首次观察都不晚于锚、年龄不超过已签上限；
/// 取（发生时间、首次观察时间、稳定 ID）字典序最大者；同 ID 内容不同即拒绝。
fn select_quote(
    selector: &QuoteSelector,
    max_age: i64,
    anchor: &Anchor,
    snapshot: &Snapshot,
) -> Result<Option<Quote>, String> {
    let mut by_id: std::collections::BTreeMap<&str, &Quote> = Default::default();
    for q in &snapshot.quotes {
        if let Some(prev) = by_id.insert(q.quote_id.as_str(), q) {
            if prev != q {
                return Err(format!(
                    "IdentityConflict：快照 {} 内报价 {} 同 ID 内容冲突，拒绝定标",
                    snapshot.snapshot_id, q.quote_id
                ));
            }
        }
    }
    let key = |x: &Quote| (x.t_occurred, x.t_first_observed, x.quote_id.clone());
    let mut best: Option<&Quote> = None;
    for q in by_id.values().copied() {
        let age = anchor.t_anchor.checked_sub(q.t_occurred);
        let eligible = q.selector == *selector
            && q.t_occurred <= anchor.t_anchor
            && q.t_first_observed <= anchor.t_anchor
            && age.is_some_and(|a| a <= max_age);
        if eligible && best.is_none_or(|b| key(q) > key(b)) {
            best = Some(q);
        }
    }
    Ok(best.cloned())
}

/// Q_units = ⌊Q_amount·scale·p_den / p_num⌋；先换货币尺度（须整除），不再乘杠杆。
pub(crate) fn to_units(amount: i64, scale: &Ratio, price: &Ratio) -> Result<(i64, Ratio), String> {
    let scaled = i128::from(amount) * i128::from(scale.num);
    if scaled % i128::from(scale.den) != 0 {
        return Err("Unsupported：货币尺度换算不整除，金额不可精确表示".into());
    }
    let scaled = scaled / i128::from(scale.den);
    let value = scaled
        .checked_mul(i128::from(price.den))
        .ok_or("InvalidDomain：定标乘法溢出")?;
    let units = i64::try_from(value / i128::from(price.num))
        .map_err(|_| "InvalidDomain：Q_units 超出 i64".to_string())?;
    let residue = i64::try_from(value % i128::from(price.num))
        .map_err(|_| "InvalidDomain：余款超出 i64".to_string())?;
    Ok((
        units,
        Ratio {
            num: residue,
            den: price.den,
        },
    ))
}

fn floor_mul(units: i64, w: &Ratio) -> Result<i64, String> {
    i64::try_from(i128::from(units) * i128::from(w.num) / i128::from(w.den))
        .map_err(|_| "InvalidDomain：tier_quantity 超出 i64".to_string())
}

impl Calibration {
    /// 同一选中报价下把金额容量换到数量域（跨维容量绑定同一定标记录，ADR 0049）。
    pub(crate) fn capacity_units(&self, c_amount: i64, scale: &Ratio) -> Result<i64, String> {
        match &self.selected {
            Some(s) => Ok(to_units(c_amount, scale, &s.quote.price)?.0),
            None => Err("Unsupported：signed_units 定标没有报价，不能把金额容量换成数量".into()),
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let ratio = |r: &Ratio| json!({"num": r.num.to_string(), "den": r.den.to_string()});
        json!({
            "policy_id": QUANTITY_CALIBRATION_POLICY_ID,
            "version": self.version,
            "supersedes": self.supersedes,
            "quantity_basis": self.basis,
            "chong_id": self.chong_id,
            "profile_id": self.binding.profile_id,
            "funding_version": self.binding.funding_version,
            "symbol": self.binding.symbol,
            "operation_level": self.binding.operation_level,
            "q_ref": self.binding.q_ref,
            "Q_amount": self.q_amount.to_string(),
            "Q_units": self.q_units.to_string(),
            "residue": self.residue.as_ref().map(ratio),
            "quote": self.selected.as_ref().map(|s| json!({
                "quote_id": s.quote.quote_id,
                "price": ratio(&s.quote.price),
                "original_ref": s.quote.original_ref,
                "anchor_id": s.anchor.anchor_id,
                "anchor_kind": s.anchor.kind.policy_text(),
                "t_anchor": s.anchor.t_anchor.to_string(),
                "snapshot_id": s.anchor.snapshot_id,
            })),
            "tiers": self.tiers.iter().map(|t| match t.quantity {
                Signed::Signed(q) => json!({"d": t.d.to_string(), "tier_quantity": q.to_string()}),
                Signed::AwaitingPolicy => json!({"d": t.d.to_string(), "tier_quantity": "AwaitingPolicy"}),
            }).collect::<Vec<_>>(),
        })
    }
}

fn basis_json(b: &QuantityBasis) -> Value {
    match b {
        QuantityBasis::SignedUnits { signature_ref, .. } => {
            json!({"kind": "signed_units", "signature_ref": signature_ref})
        }
        QuantityBasis::FrozenQuote(fq) => json!({
            "kind": "frozen_quote",
            "quote_time_policy": match fq.quote_time_policy {
                Signed::Signed(k) => k.policy_text(),
                Signed::AwaitingPolicy => "AwaitingPolicy",
            },
            "max_quote_age": match fq.max_quote_age {
                Signed::Signed(a) => a.to_string(),
                Signed::AwaitingPolicy => "AwaitingPolicy".to_string(),
            },
            "currency_scale": {"num": fq.currency_scale.num.to_string(), "den": fq.currency_scale.den.to_string()},
            "quantity_quantum": fq.quantity_quantum,
            "selector": {"source": fq.selector.source, "market": fq.selector.market,
                         "instrument": fq.selector.instrument, "price_field": fq.selector.price_field},
            "rounding": "floor",
        }),
        QuantityBasis::AwaitingPolicy => json!("AwaitingPolicy"),
    }
}

/// 拆股只按比例重基已冻结数量（书账里的定标投影）；报价、锚、快照与版本不变，不重新选价。
pub(crate) fn rebase_split(cal: &Value, num: i64, den: i64) -> Result<Value, String> {
    let rebase = |x: &Value, what: &str| -> Result<Value, String> {
        let q = parse_canonical_i64(
            x.as_str()
                .ok_or_else(|| format!("SchemaUnsupported：{what}须文本"))?,
            what,
        )?;
        let scaled = i128::from(q) * i128::from(num);
        if scaled % i128::from(den) != 0 {
            return Err("Unsupported：拆股比不整除冻结数量（ADR 0031 裁定五）".into());
        }
        Ok(json!(i64::try_from(scaled / i128::from(den))
            .map_err(|_| "InvalidDomain：拆股重基溢出".to_string())?
            .to_string()))
    };
    let mut out = cal.clone();
    out["Q_units"] = rebase(&cal["Q_units"], "Q_units")?;
    for t in out["tiers"]
        .as_array_mut()
        .ok_or("SchemaUnsupported：定标 tiers 须数组")?
    {
        if t["tier_quantity"] != "AwaitingPolicy" {
            t["tier_quantity"] = rebase(&t["tier_quantity"], "tier_quantity")?;
        }
    }
    let mut splits = cal
        .get("splits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    splits.push(json!({"num": num.to_string(), "den": den.to_string()}));
    out["splits"] = Value::Array(splits);
    Ok(out)
}

const WAIT_SLOTS: [&str; 7] = [
    "quantity_basis",
    "quote_time_policy",
    "max_quote_age",
    "quote_anchor",
    "quote_snapshot",
    "eligible_quote",
    "quantity_zero",
];
pub(crate) fn is_wait_slot(slot: &str) -> bool {
    WAIT_SLOTS.contains(&slot)
}

impl Readiness {
    pub(crate) fn ready(&self) -> Option<&Calibration> {
        match self {
            Readiness::Ready(c) => Some(c),
            _ => None,
        }
    }
    /// 未就绪原因：B 拒绝、E 等待共用同一文案。
    pub(crate) fn reason(&self) -> Option<String> {
        match self {
            Readiness::Ready(_) => None,
            Readiness::AwaitingPolicy(slot) => {
                Some(format!("AwaitingPolicy：{slot}；数量定标未签，不激活"))
            }
            Readiness::AwaitingData(slot) => Some(format!(
                "AwaitingData：{slot}；合规签署下缺合格报价数据，不读现价兜底"
            )),
            Readiness::NotComputable(reason) => Some(format!(
                "NotComputable：{reason}；数量为零不激活，余款留资金账"
            )),
        }
    }
    /// E 侧 typed 等待的 (kind, slot)。
    pub(crate) fn wait(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Readiness::Ready(_) => None,
            Readiness::AwaitingPolicy(s) => Some(("AwaitingPolicy", *s)),
            Readiness::AwaitingData(s) => Some(("AwaitingData", *s)),
            Readiness::NotComputable(s) => Some(("NotComputable", *s)),
        }
    }
}

// ---- 边界解析：manifest 冻结的 JSON → 类型化记录 ----

fn exact(v: &Value, keys: &[&str], what: &str) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("SchemaUnsupported：{what}须对象"))?;
    if keys.iter().any(|k| !o.contains_key(*k)) || o.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(format!("SchemaUnsupported：{what}字段不完整或多余"));
    }
    Ok(())
}
fn text(v: &Value, k: &str) -> Result<String, String> {
    let s = required(v, k)?;
    if s.is_empty() {
        return Err(format!("InvalidDomain：{k} 不得为空"));
    }
    Ok(s.to_owned())
}
fn int(v: &Value, k: &str) -> Result<i64, String> {
    parse_canonical_i64(required(v, k)?, k)
}
fn ratio(v: &Value, what: &str) -> Result<Ratio, String> {
    exact(v, &["num", "den"], what)?;
    Ok(Ratio {
        num: positive(required(v, "num")?, what)?,
        den: positive(required(v, "den")?, what)?,
    })
}
fn selector(v: &Value) -> Result<QuoteSelector, String> {
    Ok(QuoteSelector {
        source: text(v, "source")?,
        market: text(v, "market")?,
        instrument: text(v, "instrument")?,
        price_field: text(v, "price_field")?,
    })
}
fn awaiting(v: &Value) -> bool {
    v.as_str() == Some("AwaitingPolicy")
}

pub(crate) fn parse_policy(v: &Value) -> Result<CalibrationPolicy, String> {
    exact(
        v,
        &[
            "policy_id",
            "version",
            "supersedes",
            "binding",
            "quantity_basis",
            "tiers",
        ],
        "quantity-calibration ",
    )?;
    if v["policy_id"] != QUANTITY_CALIBRATION_POLICY_ID {
        return Err(format!(
            "IdentityConflict：数量定标政策族须为 {QUANTITY_CALIBRATION_POLICY_ID}"
        ));
    }
    let b = &v["binding"];
    exact(
        b,
        &[
            "profile_id",
            "funding_version",
            "symbol",
            "operation_level",
            "q_ref",
        ],
        "quantity-calibration binding ",
    )?;
    let binding = Binding {
        profile_id: text(b, "profile_id")?,
        funding_version: text(b, "funding_version")?,
        symbol: text(b, "symbol")?,
        operation_level: text(b, "operation_level")?,
        q_ref: text(b, "q_ref")?,
    };
    let basis = parse_basis(&v["quantity_basis"])?;
    let tiers = v["tiers"]
        .as_array()
        .ok_or("SchemaUnsupported：quantity-calibration tiers 须数组")?
        .iter()
        .map(|t| {
            exact(t, &["d", "w"], "tier ")?;
            let d = positive(required(t, "d")?, "tier.d")?;
            if awaiting(&t["w"]) || t["w"] == "Unmeasured" {
                Ok(TierRatio::Unmeasured { d })
            } else {
                Ok(TierRatio::Signed {
                    d,
                    w: ratio(&t["w"], "tier.w")?,
                })
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut seen = std::collections::BTreeSet::new();
    for t in &tiers {
        let d = match t {
            TierRatio::Signed { d, .. } | TierRatio::Unmeasured { d } => *d,
        };
        if !seen.insert(d) {
            return Err("IdentityConflict：tiers 深度重复".into());
        }
    }
    Ok(CalibrationPolicy {
        version: text(v, "version")?,
        supersedes: text(v, "supersedes")?,
        binding,
        basis,
        tiers,
    })
}

fn parse_basis(v: &Value) -> Result<QuantityBasis, String> {
    if awaiting(v) {
        return Ok(QuantityBasis::AwaitingPolicy);
    }
    match v["kind"].as_str() {
        Some("signed_units") => {
            exact(v, &["kind", "units", "signature_ref"], "signed_units ")?;
            Ok(QuantityBasis::SignedUnits {
                units: positive(required(v, "units")?, "signed_units.units")?,
                signature_ref: text(v, "signature_ref")?,
            })
        }
        Some("frozen_quote") => {
            exact(
                v,
                &[
                    "kind",
                    "quote_time_policy",
                    "max_quote_age",
                    "currency_scale",
                    "quantity_quantum",
                    "selector",
                    "rounding",
                ],
                "frozen_quote ",
            )?;
            if v["rounding"] != "floor" {
                return Err("InvalidDomain：frozen_quote 舍入只认 floor（ADR 0049 Q4）".into());
            }
            let quote_time_policy = match v["quote_time_policy"].as_str() {
                Some("activation_request_asof") => Signed::Signed(AnchorKind::ActivationRequest),
                Some("profile_signature_asof") => Signed::Signed(AnchorKind::ProfileSignature),
                Some("AwaitingPolicy") => Signed::AwaitingPolicy,
                _ => return Err("InvalidDomain：quote_time_policy 不在可选集".into()),
            };
            let max_quote_age = if awaiting(&v["max_quote_age"]) {
                Signed::AwaitingPolicy
            } else {
                Signed::Signed(positive(required(v, "max_quote_age")?, "max_quote_age")?)
            };
            Ok(QuantityBasis::FrozenQuote(FrozenQuotePolicy {
                quote_time_policy,
                max_quote_age,
                currency_scale: ratio(&v["currency_scale"], "currency_scale")?,
                quantity_quantum: text(v, "quantity_quantum")?,
                selector: selector(&v["selector"])?,
            }))
        }
        _ => Err(
            "InvalidDomain：quantity_basis 只认 signed_units、frozen_quote 或 AwaitingPolicy"
                .into(),
        ),
    }
}

pub(crate) fn parse_anchor(v: &Value) -> Result<Anchor, String> {
    exact(
        v,
        &["kind", "anchor_id", "t_anchor", "snapshot_id"],
        "quote anchor ",
    )?;
    let kind = match v["kind"].as_str() {
        Some("activation_request") => AnchorKind::ActivationRequest,
        Some("profile_signature") => AnchorKind::ProfileSignature,
        _ => {
            return Err("InvalidDomain：锚类型只认 activation_request 或 profile_signature".into())
        }
    };
    Ok(Anchor {
        kind,
        anchor_id: text(v, "anchor_id")?,
        t_anchor: int(v, "t_anchor")?,
        snapshot_id: text(v, "snapshot_id")?,
    })
}

pub(crate) fn parse_snapshot(v: &Value) -> Result<Snapshot, String> {
    exact(v, &["snapshot_id", "quotes"], "quote snapshot ")?;
    let quotes = v["quotes"]
        .as_array()
        .ok_or("SchemaUnsupported：快照 quotes 须数组")?
        .iter()
        .map(|q| {
            exact(
                q,
                &[
                    "quote_id",
                    "source",
                    "market",
                    "instrument",
                    "price_field",
                    "t_occurred",
                    "t_first_observed",
                    "price",
                    "original_ref",
                ],
                "quote ",
            )?;
            let price = &q["price"];
            exact(price, &["num", "den"], "quote price ")?;
            let p_num = int(price, "num")?;
            let p_den = int(price, "den")?;
            if p_num <= 0 || p_den <= 0 {
                return Err("InvalidDomain：报价须为正有理数".into());
            }
            Ok(Quote {
                quote_id: text(q, "quote_id")?,
                selector: selector(q)?,
                t_occurred: int(q, "t_occurred")?,
                t_first_observed: int(q, "t_first_observed")?,
                price: Ratio {
                    num: p_num,
                    den: p_den,
                },
                original_ref: text(q, "original_ref")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Snapshot {
        snapshot_id: text(v, "snapshot_id")?,
        quotes,
    })
}
