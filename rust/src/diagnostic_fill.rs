//! #1718：独立诊断限价成交，不产生正式成交或持仓。
use crate::theta_v0::config::FillRule;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MODEL_ID: &str = "adr0018-integer-bar/1";
pub const REQUEST_SCHEMA: &str = "diagnostic-fill-request/1";
pub const WARNING: &str =
    "touch是乐观撮合，存在过拟合代价；全部诊断输出均不得用于生产准入或正式验收";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Order {
    pub id: String,
    pub side: Side,
    pub quantity: u64,
    pub limit_ticks: i64,
    pub submitted_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    pub opened_at: u64,
    pub event_time: u64,
    pub known_at: u64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub model_id: String,
    pub record_version: String,
    pub mode: String,
    pub fill_rule: FillRule,
    pub market_snapshot: String,
    pub as_of: u64,
    pub orders: Vec<Order>,
    pub bars: Vec<Bar>,
}

impl Request {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != REQUEST_SCHEMA || self.model_id != MODEL_ID {
            return Err("SchemaUnsupported：诊断模型或请求版本不受支持".into());
        }
        if self.mode != "diagnostic" {
            return Err("DiagnosticOnly：此模型及其结果不进入生产准入或正式验收".into());
        }
        self.fill_rule.validate()?;
        if self.record_version.is_empty() || self.market_snapshot.is_empty() {
            return Err("InvalidDomain：缺具名记录版本或行情快照".into());
        }
        let mut ids = BTreeSet::new();
        for order in &self.orders {
            if order.id.is_empty()
                || order.quantity == 0
                || order.submitted_at > self.as_of
                || !ids.insert(&order.id)
            {
                return Err(
                    "InvalidDomain：订单身份须唯一、数量为正且提交时间不越过已知前沿".into(),
                );
            }
        }
        let mut previous = None;
        for bar in &self.bars {
            if bar.low > bar.open
                || bar.low > bar.close
                || bar.high < bar.open
                || bar.high < bar.close
                || bar.low > bar.high
                || bar.opened_at >= bar.event_time
                || bar.event_time > bar.known_at
                || bar.known_at > self.as_of
                || previous
                    .is_some_and(|(event, known)| bar.opened_at < event || bar.known_at < known)
            {
                return Err("InvalidDomain：行情不是该快照已知前沿的有序有效OHLC".into());
            }
            previous = Some((bar.event_time, bar.known_at));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub order_id: String,
    pub side: Side,
    pub quantity: u64,
    pub bar_index: usize,
    pub event_time: u64,
    pub known_at: u64,
    pub price_ticks: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub mode: String,
    pub model_id: String,
    pub record_version: String,
    pub fill_rule: FillRule,
    pub market_snapshot: String,
    pub as_of: u64,
    pub processed_bars: usize,
    pub fills: Vec<Fill>,
    pub pending_order_ids: Vec<String>,
    pub warning: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    request: Request,
    next_bar: usize,
    fills: Vec<Fill>,
}

pub struct Replay {
    checkpoint: Checkpoint,
}

fn matched(order: &Order, bar: &Bar, rule: FillRule) -> Option<i64> {
    if order.submitted_at > bar.opened_at {
        return None;
    }
    let distance = match order.side {
        Side::Buy => i128::from(order.limit_ticks) - i128::from(bar.low),
        Side::Sell => i128::from(bar.high) - i128::from(order.limit_ticks),
    };
    let minimum = match rule {
        FillRule::Cross(k) => i128::from(k),
        FillRule::Touch => 0,
    };
    (distance >= minimum).then(|| match order.side {
        Side::Buy => bar.open.min(order.limit_ticks),
        Side::Sell => bar.open.max(order.limit_ticks),
    })
}

impl Replay {
    pub fn new(request: Request) -> Result<Self, String> {
        request.validate()?;
        Ok(Self {
            checkpoint: Checkpoint {
                request,
                next_bar: 0,
                fills: Vec::new(),
            },
        })
    }

    pub fn restore(checkpoint: Checkpoint, expected: &Request) -> Result<Self, String> {
        if checkpoint.request != *expected {
            return Err(
                "VersionMismatch：恢复必须绑定原模型、记录版本、规则、订单及行情快照".into(),
            );
        }
        if checkpoint.next_bar > expected.bars.len() {
            return Err("InvalidCheckpoint：游标越过行情前沿".into());
        }
        let mut restored = Self::new(expected.clone())?;
        restored.advance_to(checkpoint.next_bar)?;
        if restored.checkpoint.fills != checkpoint.fills {
            return Err("InvalidCheckpoint：成交序列不是冻结输入的完整首次命中序列".into());
        }
        Ok(restored)
    }

    pub fn advance_to(&mut self, end: usize) -> Result<(), String> {
        let state = &mut self.checkpoint;
        if end < state.next_bar || end > state.request.bars.len() {
            return Err("InvalidDomain：只允许增量推进到现有行情前沿".into());
        }
        let mut filled: BTreeSet<String> = state.fills.iter().map(|f| f.order_id.clone()).collect();
        for index in state.next_bar..end {
            let bar = &state.request.bars[index];
            for order in &state.request.orders {
                if filled.contains(&order.id) {
                    continue;
                }
                if let Some(price_ticks) = matched(order, bar, state.request.fill_rule) {
                    state.fills.push(Fill {
                        order_id: order.id.clone(),
                        side: order.side,
                        quantity: order.quantity,
                        bar_index: index,
                        event_time: bar.event_time,
                        known_at: bar.known_at,
                        price_ticks,
                    });
                    filled.insert(order.id.clone());
                }
            }
        }
        state.next_bar = end;
        Ok(())
    }

    pub fn checkpoint(&self) -> Checkpoint {
        self.checkpoint.clone()
    }

    pub fn report(&self) -> Report {
        let state = &self.checkpoint;
        let request = &state.request;
        let filled: BTreeSet<&str> = state.fills.iter().map(|f| f.order_id.as_str()).collect();
        Report {
            schema: "diagnostic-fill-report/1".into(),
            mode: "diagnostic".into(),
            model_id: request.model_id.clone(),
            record_version: request.record_version.clone(),
            fill_rule: request.fill_rule,
            market_snapshot: request.market_snapshot.clone(),
            as_of: request.as_of,
            processed_bars: state.next_bar,
            fills: state.fills.clone(),
            pending_order_ids: request
                .orders
                .iter()
                .filter(|o| !filled.contains(o.id.as_str()))
                .map(|o| o.id.clone())
                .collect(),
            warning: WARNING.into(),
        }
    }
}

pub fn replay(request: Request) -> Result<Report, String> {
    let end = request.bars.len();
    let mut replay = Replay::new(request)?;
    replay.advance_to(end)?;
    Ok(replay.report())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub schema: String,
    pub mode: String,
    pub cross: Report,
    pub touch: Report,
    pub changed_order_ids: Vec<String>,
    pub warning: String,
}

pub fn compare(request: &Request) -> Result<Comparison, String> {
    request.validate()?;
    if !matches!(request.fill_rule, FillRule::Cross(_)) {
        return Err("InvalidDomain：对照须显式指定cross(k)，不自动换规则".into());
    }
    let cross = replay(request.clone())?;
    let mut touch_request = request.clone();
    touch_request.fill_rule = FillRule::Touch;
    let touch = replay(touch_request)?;
    let changed_order_ids = request
        .orders
        .iter()
        .filter(|order| {
            cross.fills.iter().find(|f| f.order_id == order.id)
                != touch.fills.iter().find(|f| f.order_id == order.id)
        })
        .map(|o| o.id.clone())
        .collect();
    Ok(Comparison {
        schema: "diagnostic-fill-comparison/1".into(),
        mode: "diagnostic".into(),
        cross,
        touch,
        changed_order_ids,
        warning: WARNING.into(),
    })
}
