use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MODEL_ID: &str = "history-sim/bar-batch-known-frontier/1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrderKind {
    Limit { price: i64 },
    Market { exec_index: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Order {
    pub command_id: String,
    pub family_id: String,
    pub chong_id: String,
    pub revision: i64,
    pub side: Side,
    pub quantity: i64,
    pub kind: OrderKind,
    pub eligibility_ref: String,
    pub projection_manifest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    pub index: i64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
}
impl Bar {
    fn validate(&self) -> Result<(), String> {
        if self.index < 0
            || self.low <= 0
            || self.low > self.open
            || self.low > self.close
            || self.high < self.open
            || self.high < self.close
        {
            return Err("InvalidDomain：bar 索引或整数 OHLC 非法".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Capacity {
    None,
    Scenario {
        reference: String,
        buy: i64,
        sell: i64,
    },
    Input {
        reference: String,
        buy: i64,
        sell: i64,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    Floor,
    Ceiling,
    Exact,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    pub numerator: i64,
    pub denominator: i64,
    pub rounding: Rounding,
}
impl Rate {
    fn charge(&self, amount: i64) -> Result<i64, String> {
        if self.numerator < 0 || self.denominator <= 0 || amount < 0 {
            return Err("InvalidDomain：费用率不合法".into());
        }
        let n = i128::from(amount) * i128::from(self.numerator);
        let d = i128::from(self.denominator);
        let q = match self.rounding {
            Rounding::Floor => n / d,
            Rounding::Ceiling => (n + d - 1) / d,
            Rounding::Exact if n % d == 0 => n / d,
            Rounding::Exact => return Err("Unsupported：具名费用合同要求整除".into()),
        };
        i64::try_from(q).map_err(|_| "InvalidDomain：费用金额溢出".into())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeeRole {
    Maker,
    Taker,
}
impl FeeRole {
    fn name(self) -> &'static str {
        match self {
            Self::Maker => "maker",
            Self::Taker => "taker",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeeProfile {
    pub reference: String,
    pub role_source: String,
    pub limit_role: FeeRole,
    pub market_role: FeeRole,
    pub maker: Rate,
    pub taker: Rate,
    pub market_slippage_ticks: i64,
    pub market_slippage_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub model_id: String,
    pub revision: String,
    pub symbol: String,
    pub quantity_unit: String,
    pub currency: String,
    pub tick_value: i64,
    pub fee_profile: FeeProfile,
    pub rd11_policy: String,
    pub rd12_policy: String,
    pub projection_kind: String,
    pub seed: String,
}
impl Model {
    pub fn validate(&self) -> Result<(), String> {
        if self.model_id != MODEL_ID || self.seed != "none" {
            return Err("Unsupported：未知模型或非确定性 seed".into());
        }
        if self.rd12_policy != "per-leg-source-attribution/1" {
            return Err("Unsupported：per_leg 模拟只支持 ADR0032 逐腿来源归属政策".into());
        }
        if self.projection_kind != "per_leg" {
            return Err("Unsupported：本模型实现尚无 netted 执行投影，不回落 per_leg".into());
        }
        for s in [
            &self.revision,
            &self.symbol,
            &self.quantity_unit,
            &self.currency,
            &self.rd11_policy,
            &self.rd12_policy,
            &self.fee_profile.reference,
            &self.fee_profile.role_source,
            &self.fee_profile.market_slippage_source,
        ] {
            if s.is_empty() {
                return Err("AwaitingPolicy：版本或具名成本来源缺失".into());
            }
        }
        if self.fee_profile.market_slippage_ticks < 0 || self.tick_value <= 0 {
            return Err("InvalidDomain：滑点 tick 不得为负".into());
        }
        self.fee_profile.maker.charge(0)?;
        self.fee_profile.taker.charge(0)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Submit {
        order: Order,
    },
    Cancel {
        family_id: String,
        command_id: String,
        eligibility_ref: String,
    },
    Replace {
        order: Order,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bust {
    pub exec_id: String,
    pub quantity: i64,
    pub scenario_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub request_id: String,
    pub known_frontier_cut: String,
    pub bar: Bar,
    pub capacity: Capacity,
    pub close_changes: Vec<Change>,
    pub scenario_busts: Vec<Bust>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkingOrder {
    pub order: Order,
    pub filled: i64,
    pub active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub last_bar: Option<i64>,
    pub families: BTreeMap<String, WorkingOrder>,
    pub commands: BTreeSet<String>,
    pub capacity_state: super::capacity::CapacityState,
    pub last_close_changes: Vec<Change>,
    pub last_request_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub exec_id: String,
    pub command_id: String,
    pub family_id: String,
    pub chong_id: String,
    pub revision: i64,
    pub side: Side,
    pub quantity: i64,
    pub price: i64,
    pub amount: i64,
    pub fee: i64,
    pub role: String,
    pub slot: String,
    pub projection_manifest: String,
    pub capacity_epoch: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResultBatch {
    pub fills: Vec<Fill>,
    pub intra_batch_order: String,
    pub l0_fill_count: usize,
    pub next_state: State,
    pub unfilled_market: BTreeMap<String, i64>,
    pub capacity_transitions: Vec<serde_json::Value>,
    pub capacity_evidence: Vec<serde_json::Value>,
}

fn valid_order(order: &Order, bar: i64) -> Result<(), String> {
    if [
        &order.command_id,
        &order.family_id,
        &order.chong_id,
        &order.eligibility_ref,
        &order.projection_manifest,
    ]
    .iter()
    .any(|s| s.is_empty())
        || order.quantity <= 0
        || order.revision <= 0
    {
        return Err("InvalidDomain：订单身份、资格引用、数量或版本非法".into());
    }
    match order.kind {
        OrderKind::Limit { price } if price <= 0 => {
            Err("InvalidDomain：限价须为正整数 tick".into())
        }
        OrderKind::Market { exec_index } if exec_index <= bar => {
            Err("ModelOrderUnspecified：收盘新单不得当根或过去执行".into())
        }
        _ => Ok(()),
    }
}

/// 状态只记录 X 的执行进度，不拥有成交持仓账。收盘变化在成交批完成后才作用。
pub fn advance(model: &Model, state: &State, batch: &Batch) -> Result<ResultBatch, String> {
    model.validate()?;
    batch.bar.validate()?;
    if batch.request_id.is_empty() || batch.known_frontier_cut.is_empty() {
        return Err("InvalidDomain：批次身份与已知前沿不得为空".into());
    }
    if let Some(previous) = state.last_bar {
        if previous.checked_add(1) != Some(batch.bar.index) {
            return Err("ModelOrderUnspecified：bar 必须连续，不能跨过潜在成交".into());
        }
    }
    match &batch.capacity {
        Capacity::None => (),
        Capacity::Input {
            reference,
            buy,
            sell,
        }
        | Capacity::Scenario {
            reference,
            buy,
            sell,
        } => {
            if reference.is_empty() || *buy < 0 || *sell < 0 {
                return Err("InvalidDomain：容量原件缺失或数量非法".into());
            }
        }
    }
    let mut next = state.clone();
    let mut candidates = Vec::new();
    let b = &batch.bar;
    for (family, working) in &state.families {
        if !working.active || working.filled == working.order.quantity {
            continue;
        }
        let order = &working.order;
        if working.filled < 0 || working.filled > order.quantity || order.family_id != *family {
            return Err("StorageUnavailable：订单族或累计成交状态不合法".into());
        }
        let px = match order.kind {
            OrderKind::Limit { price } => match order.side {
                Side::Buy if b.low < price => Some((
                    b.open.min(price),
                    "limit_interval",
                    model.fee_profile.limit_role.name(),
                )),
                Side::Sell if b.high > price => Some((
                    b.open.max(price),
                    "limit_interval",
                    model.fee_profile.limit_role.name(),
                )),
                _ => None,
            },
            OrderKind::Market { exec_index } if exec_index == b.index => {
                let slip = model.fee_profile.market_slippage_ticks;
                let price = match order.side {
                    Side::Buy => b.close.checked_add(slip),
                    Side::Sell => b.close.checked_sub(slip),
                }
                .filter(|p| *p > 0)
                .ok_or("InvalidDomain：市价滑点溢出或非正价格")?;
                Some((price, "close", model.fee_profile.market_role.name()))
            }
            OrderKind::Market { exec_index } if exec_index < b.index => {
                return Err("ModelOrderUnspecified：市价执行槽已过去".into())
            }
            _ => None,
        };
        if let Some((px, slot, role)) = px {
            let q = order
                .quantity
                .checked_sub(working.filled)
                .filter(|q| *q > 0)
                .ok_or("StorageUnavailable：累计成交超过额度")?;
            candidates.push((family.clone(), q, px, slot, role));
        }
    }
    // 排列只规范化输出及冻结配额并列键，不声明批内时间顺序。
    candidates.sort_by_key(|(family, _, _, slot, _)| (*slot == "close", family.clone()));
    let mut active_chongs = BTreeSet::new();
    for (family, ..) in &candidates {
        if !active_chongs.insert(&state.families[family].order.chong_id) {
            return Err("ModelOrderUnspecified：同重同批不允许多个动作".into());
        }
    }
    let (allocated, capacity_state, mut capacity_transitions, capacity_evidence) =
        super::capacity::allocate(model, state, batch, &candidates)?;
    next.capacity_state = capacity_state;
    let mut fills = Vec::new();
    for ((family, _, price, slot, role), quantity) in candidates.into_iter().zip(allocated) {
        if quantity == 0 {
            continue;
        }
        let w = next.families.get_mut(&family).unwrap();
        let amount = quantity
            .checked_mul(price)
            .and_then(|a| a.checked_mul(model.tick_value))
            .ok_or("InvalidDomain：成交金额溢出")?;
        let rate = if role == "maker" {
            &model.fee_profile.maker
        } else {
            &model.fee_profile.taker
        };
        let fee = rate.charge(amount)?;
        let exec_id = format!("{}:{}:{}", b.index, family, w.order.revision);
        fills.push(Fill {
            exec_id,
            command_id: w.order.command_id.clone(),
            family_id: family,
            chong_id: w.order.chong_id.clone(),
            revision: w.order.revision,
            side: w.order.side,
            quantity,
            price,
            amount,
            fee,
            role: role.into(),
            slot: slot.into(),
            projection_manifest: w.order.projection_manifest.clone(),
            capacity_epoch: capacity_evidence
                .iter()
                .find(|e| {
                    e["legs"].as_array().unwrap().iter().any(|l| {
                        l["family_id"] == w.order.family_id
                            && l["delta"].as_i64().is_some_and(|q| q > 0)
                    })
                })
                .and_then(|e| e["capacity_epoch"].as_str())
                .map(str::to_owned),
        });
        w.filled = w
            .filled
            .checked_add(quantity)
            .ok_or("InvalidDomain：累计成交溢出")?;
    }
    super::capacity::close_completed(&mut next, batch, &mut capacity_transitions);
    let mut unfilled_market = BTreeMap::new();
    for (family, working) in &mut next.families {
        if matches!(working.order.kind, OrderKind::Market { exec_index } if exec_index == b.index) {
            let remaining = working.order.quantity - working.filled;
            if remaining > 0 {
                unfilled_market.insert(family.clone(), remaining);
            }
            working.active = false;
        }
    }
    let mut close_changes = batch.close_changes.clone();
    close_changes.sort_by(|a, b| change_id(a).cmp(change_id(b)));
    let mut changed_families = BTreeSet::new();
    for change in &close_changes {
        let family = match change {
            Change::Submit { order } | Change::Replace { order } => &order.family_id,
            Change::Cancel { family_id, .. } => family_id,
        };
        if !changed_families.insert(family) {
            return Err("ModelOrderUnspecified：同close同family多个变化".into());
        }
        match change {
            Change::Submit { order } => {
                valid_order(order, b.index)?;
                if next.families.contains_key(&order.family_id)
                    || !next.commands.insert(order.command_id.clone())
                {
                    return Err("IdentityConflict：重复 family 或 command".into());
                }
                next.families.insert(
                    order.family_id.clone(),
                    WorkingOrder {
                        order: order.clone(),
                        filled: 0,
                        active: true,
                    },
                );
            }
            Change::Replace { order } => {
                valid_order(order, b.index)?;
                if !next.commands.insert(order.command_id.clone()) {
                    return Err("IdentityConflict：重复 command".into());
                }
                let w = next
                    .families
                    .get_mut(&order.family_id)
                    .ok_or("MissingDependency：修改缺原订单")?;
                if !w.active
                    || order.revision <= w.order.revision
                    || order.chong_id != w.order.chong_id
                    || order.side != w.order.side
                    || order.quantity != w.order.quantity
                {
                    return Err(
                        "IdentityConflict：修改必须同 family/方向/冻结额度且版本递增".into(),
                    );
                }
                w.order = order.clone();
            }
            Change::Cancel {
                family_id,
                command_id,
                eligibility_ref,
            } => {
                if eligibility_ref.is_empty()
                    || command_id.is_empty()
                    || !next.commands.insert(command_id.clone())
                {
                    return Err("IdentityConflict：取消缺资格或重复 command".into());
                }
                next.families
                    .get_mut(family_id)
                    .ok_or("MissingDependency：取消缺原订单")?
                    .active = false;
            }
        }
    }
    next.last_bar = Some(b.index);
    next.last_close_changes = close_changes;
    next.last_request_id = Some(batch.request_id.clone());
    Ok(ResultBatch {
        l0_fill_count: if matches!(batch.capacity, Capacity::None) {
            fills.len()
        } else {
            0
        },
        fills,
        intra_batch_order: "Unknown".into(),
        next_state: next,
        unfilled_market,
        capacity_transitions,
        capacity_evidence,
    })
}

fn change_id(c: &Change) -> &str {
    match c {
        Change::Submit { order } | Change::Replace { order } => &order.command_id,
        Change::Cancel { command_id, .. } => command_id,
    }
}
impl Batch {
    pub(crate) fn normalize(&mut self) {
        self.close_changes
            .sort_by(|a, b| change_id(a).cmp(change_id(b)));
    }
}
