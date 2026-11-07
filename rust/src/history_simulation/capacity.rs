//! #1381 R3：容量 epoch 与 E 投影身份独立；这里只消费已获准命令。
use super::model::{Batch, Capacity, Model, Side, State};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Candidate = (String, i64, i64, &'static str, &'static str);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub family_id: String,
    pub chong_id: String,
    pub requested_q: i64,
    pub base_filled: i64,
    pub authorized_limit: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub capacity_epoch: String,
    pub symbol: String,
    pub side: Side,
    pub slot: String,
    pub capacity_kind: String,
    pub policy: String,
    pub participants: Vec<Member>,
    pub tie_break_order: Vec<String>,
    pub formed_bar: super::model::Bar,
    pub known_frontier_cut: String,
    pub content_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Epoch {
    pub manifest: Manifest,
    pub cumulative: i64,
    pub allocated: Vec<i64>,
    pub closed_at: Option<i64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapacityState {
    pub epochs: BTreeMap<String, Epoch>,
    pub active: BTreeMap<String, String>,
}
fn digest(m: &Manifest) -> String {
    let mut v = serde_json::to_value(m).expect("integer manifest");
    v.as_object_mut().unwrap().remove("content_hash");
    sha256_hex(canonical_json(&v).as_bytes())
}
fn side_name(s: Side) -> &'static str {
    match s {
        Side::Buy => "buy",
        Side::Sell => "sell",
    }
}
fn ranks(m: &Manifest) -> Result<Vec<usize>, String> {
    m.participants
        .iter()
        .map(|p| {
            m.tie_break_order
                .iter()
                .position(|x| x == &p.chong_id)
                .ok_or_else(|| "IdentityConflict：容量并列键缺参与重".into())
        })
        .collect()
}
fn audit_epoch(e: &Epoch, state: &State) -> Result<(), String> {
    let m = &e.manifest;
    if m.content_hash != digest(m)
        || m.participants.is_empty()
        || m.participants.len() != e.allocated.len()
        || m.policy != "frozen-manifest-proportional-cumulative/2"
    {
        return Err("IdentityConflict：容量清单内容或累计形状不符".into());
    }
    let weights: Vec<_> = m.participants.iter().map(|x| x.requested_q).collect();
    let expected =
        crate::economic_session::simulation_increment(0, e.cumulative, &weights, &ranks(m)?)?;
    if expected != e.allocated {
        return Err("IdentityConflict：容量累计分量不等于冻结配额".into());
    }
    for (p, &f) in m.participants.iter().zip(&e.allocated) {
        let w = state
            .families
            .get(&p.family_id)
            .ok_or("IdentityConflict：容量成员缺原family")?;
        if p.base_filled.checked_add(f) != Some(w.filled)
            || p.base_filled.checked_add(p.requested_q) != Some(p.authorized_limit)
        {
            return Err("IdentityConflict：容量累计证据与family累计不符".into());
        }
    }
    Ok(())
}

/// 返回分配、完整新容量状态、切换与累计证据；最终由 X 同一 SQLite 事务持久化。
pub(super) fn allocate(
    model: &Model,
    state: &State,
    batch: &Batch,
    candidates: &[Candidate],
) -> Result<(Vec<i64>, CapacityState, Vec<Value>, Vec<Value>), String> {
    for (id, e) in &state.capacity_state.epochs {
        if e.closed_at.is_none() {
            let key = format!("{}/{}", side_name(e.manifest.side), e.manifest.slot);
            if state.capacity_state.active.get(&key) != Some(id)
                || e.manifest.capacity_epoch != *id
                || e.manifest.symbol != model.symbol
            {
                return Err("IdentityConflict：容量旧epoch或活动累计引用缺失".into());
            }
        }
    }
    let mut next = state.capacity_state.clone();
    let mut allocated = vec![0; candidates.len()];
    let mut transitions = Vec::new();
    let mut evidence = Vec::new();
    let kind = match &batch.capacity {
        Capacity::None => "none",
        Capacity::Input { .. } => "input",
        Capacity::Scenario { .. } => "scenario",
    };
    // 已完成、已取消和未穿越成员都不能靠调用方子集隐去：集合只从完整已知状态与完整bar算出。
    for side in [Side::Buy, Side::Sell] {
        let mut budget = match &batch.capacity {
            Capacity::None => i64::MAX,
            Capacity::Input { buy, sell, .. } | Capacity::Scenario { buy, sell, .. } => {
                if side == Side::Buy {
                    *buy
                } else {
                    *sell
                }
            }
        };
        // 限价全区间逻辑先于具名收盘槽；同方向总容量只消费一次。
        for slot in ["limit_interval", "close"] {
            let key = format!("{}/{}", side_name(side), slot);
            let ids: Vec<_> = candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| state.families[&c.0].order.side == side && c.3 == slot)
                .map(|(i, _)| i)
                .collect();
            let current: BTreeSet<_> = ids.iter().map(|&i| candidates[i].0.clone()).collect();
            let old_id = next.active.get(&key).cloned();
            let mut keep = false;
            if let Some(id) = &old_id {
                let old = next
                    .epochs
                    .get(id)
                    .ok_or("IdentityConflict：活动容量epoch原件缺失")?;
                if old.closed_at.is_some() {
                    return Err("IdentityConflict：已关闭容量epoch仍活动".into());
                }
                audit_epoch(old, state)?;
                let positive: BTreeSet<_> = old
                    .manifest
                    .participants
                    .iter()
                    .filter(|p| state.families[&p.family_id].filled < p.authorized_limit)
                    .map(|p| p.family_id.clone())
                    .collect();
                let same_limits = old
                    .manifest
                    .participants
                    .iter()
                    .filter(|p| state.families[&p.family_id].filled < p.authorized_limit)
                    .all(|p| {
                        let w = &state.families[&p.family_id];
                        w.order.quantity == p.authorized_limit && w.order.chong_id == p.chong_id
                    });
                keep = kind != "none"
                    && !current.is_empty()
                    && positive == current
                    && same_limits
                    && old.manifest.capacity_kind == kind;
            }
            if !keep {
                let previous = if let Some(id) = old_id.as_ref() {
                    let old = next.epochs.get_mut(id).unwrap();
                    old.closed_at = Some(batch.bar.index);
                    next.active.remove(&key);
                    Some(old.clone())
                } else {
                    None
                };
                let successor = if kind != "none" && !ids.is_empty() {
                    let members: Vec<_> = ids
                        .iter()
                        .map(|&i| {
                            let w = &state.families[&candidates[i].0];
                            Member {
                                family_id: w.order.family_id.clone(),
                                chong_id: w.order.chong_id.clone(),
                                requested_q: w.order.quantity - w.filled,
                                base_filled: w.filled,
                                authorized_limit: w.order.quantity,
                            }
                        })
                        .collect();
                    let mut ties: Vec<_> = members.iter().map(|p| p.chong_id.clone()).collect();
                    ties.sort();
                    let id = format!("capacity:{}:{}:{}", batch.bar.index, side_name(side), slot);
                    let mut m = Manifest {
                        capacity_epoch: id.clone(),
                        symbol: model.symbol.clone(),
                        side,
                        slot: slot.into(),
                        capacity_kind: kind.into(),
                        policy: "frozen-manifest-proportional-cumulative/2".into(),
                        participants: members,
                        tie_break_order: ties,
                        formed_bar: batch.bar.clone(),
                        known_frontier_cut: batch.known_frontier_cut.clone(),
                        content_hash: String::new(),
                    };
                    m.content_hash = digest(&m);
                    if next.epochs.contains_key(&id) {
                        return Err("IdentityConflict：容量epoch身份已存在，禁止换内容".into());
                    }
                    next.epochs.insert(
                        id.clone(),
                        Epoch {
                            allocated: vec![0; m.participants.len()],
                            manifest: m.clone(),
                            cumulative: 0,
                            closed_at: None,
                        },
                    );
                    next.active.insert(key.clone(), id);
                    Some(m)
                } else {
                    None
                };
                if previous.is_some() || successor.is_some() {
                    transitions.push(json!({"previous":previous,"successor":successor,"effective_bar":batch.bar,"slot":slot,"known_frontier_cut":batch.known_frontier_cut,"complete_known_commands":state.families,"preceding_close_changes":state.last_close_changes,"originals_frontier":state.last_request_id}));
                }
            }
            if ids.is_empty() {
                continue;
            }
            let total = ids
                .iter()
                .try_fold(0i64, |v, &i| v.checked_add(candidates[i].1))
                .ok_or("InvalidDomain：容量可成合计溢出")?;
            if kind == "none" {
                for &i in &ids {
                    allocated[i] = candidates[i].1;
                }
                continue;
            }
            let amount = total.min(budget);
            budget -= amount;
            let id = next.active[&key].clone();
            let epoch = next.epochs.get_mut(&id).unwrap();
            let from = epoch.cumulative;
            let to = from
                .checked_add(amount)
                .ok_or("InvalidDomain：容量累计溢出")?;
            let weights: Vec<_> = epoch
                .manifest
                .participants
                .iter()
                .map(|x| x.requested_q)
                .collect();
            let delta = crate::economic_session::simulation_increment(
                from,
                to,
                &weights,
                &ranks(&epoch.manifest)?,
            )?;
            let mut legs = Vec::new();
            for (n, (p, &q)) in epoch.manifest.participants.iter().zip(&delta).enumerate() {
                epoch.allocated[n] = epoch.allocated[n]
                    .checked_add(q)
                    .ok_or("InvalidDomain：容量腿累计溢出")?;
                let w = &state.families[&p.family_id];
                if q > 0 {
                    let i = *ids
                        .iter()
                        .find(|&&i| candidates[i].0 == p.family_id)
                        .ok_or("IdentityConflict：容量份额指向非可成腿")?;
                    if q > candidates[i].1 {
                        return Err("IdentityConflict：容量份额超合法剩余量".into());
                    }
                    allocated[i] = q;
                }
                legs.push(json!({"family_id":p.family_id,"delta":q,"cumulative":epoch.allocated[n],"command_id":w.order.command_id,"revision":w.order.revision,"projection_manifest":w.order.projection_manifest}));
            }
            epoch.cumulative = to;
            evidence.push(json!({"capacity_epoch":id,"manifest":epoch.manifest,"from":from,"to":to,"legs":legs,"capacity":batch.capacity}));
        }
    }
    Ok((allocated, next, transitions, evidence))
}

pub(super) fn close_completed(state: &mut State, batch: &Batch, transitions: &mut Vec<Value>) {
    let completed: Vec<_> = state
        .capacity_state
        .active
        .iter()
        .filter(|(_, id)| {
            state.capacity_state.epochs[*id]
                .manifest
                .participants
                .iter()
                .all(|p| state.families[&p.family_id].filled == p.authorized_limit)
        })
        .map(|(k, id)| (k.clone(), id.clone()))
        .collect();
    for (key, id) in completed {
        let e = state.capacity_state.epochs.get_mut(&id).unwrap();
        e.closed_at = Some(batch.bar.index);
        transitions.push(json!({"previous":e,"successor":null,"completion":true,"effective_bar":batch.bar,"known_frontier_cut":batch.known_frontier_cut,"complete_known_commands":state.families,"originals_frontier":batch.request_id}));
        state.capacity_state.active.remove(&key);
    }
}
