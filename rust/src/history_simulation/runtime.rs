use super::{
    model::{Batch, Change, Model},
    store,
};
use crate::session_protocol::{canonical_json, sha256_hex};
use crate::session_transport::{
    read_frame, serve_unix, write_frame_observed, NoObserver, TransportBounds,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const WIRE: &str = "x-exact-simulation/1";
pub const AUTHORITY_WIRE: &str = "x-exact-authority/1";
fn hash<T: Serialize>(v: &T) -> String {
    sha256_hex(canonical_json(&serde_json::to_value(v).expect("integer domain")).as_bytes())
}
fn error(e: impl std::fmt::Display) -> String {
    format!("InvalidDomain：{e}")
}
fn io(e: impl std::fmt::Display) -> String {
    format!("AuthorityUnavailable：{e}")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub socket: String,
    pub authority_id: String,
    pub authority_epoch: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub model: Model,
    pub session_id: String,
    pub session_generation: String,
    pub x_id: String,
    pub source_namespace: String,
    pub source_epoch: String,
    pub socket: String,
    pub e: Authority,
    pub s: Authority,
    pub timeout_ms: u64,
    pub max_frame_bytes: usize,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        self.model.validate()?;
        for s in [
            &self.session_id,
            &self.session_generation,
            &self.x_id,
            &self.source_namespace,
            &self.source_epoch,
            &self.e.authority_id,
            &self.e.authority_epoch,
            &self.s.authority_id,
            &self.s.authority_epoch,
        ] {
            if s.is_empty() {
                return Err("InvalidDomain：模拟会话/权威身份缺失".into());
            }
        }
        for path in [&self.socket, &self.e.socket, &self.s.socket] {
            if !Path::new(path).is_absolute() {
                return Err("InvalidDomain：模拟权威 socket 须绝对路径".into());
            }
        }
        if self.socket == self.e.socket
            || self.socket == self.s.socket
            || self.e.socket == self.s.socket
            || self.timeout_ms == 0
            || self.max_frame_bytes == 0
        {
            return Err("InvalidDomain：模拟端点须独立且资源界为正".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub condition_id: String,
    pub authority_revision: String,
    pub dependency_set: Vec<String>,
    pub invalidation_sources: Vec<String>,
    pub required_check_stages: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub coverage_proof: String,
    pub validator_capability: String,
    pub failure_result: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    pub rd11_policy: String,
    pub responsibility_refs: Vec<String>,
    pub structure_dependencies: Vec<String>,
    pub market_dependencies: Vec<String>,
    pub rule_revision: String,
    pub conditions: Vec<Condition>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExactCommand {
    pub command_id: String,
    pub logical_order_family: String,
    pub session_id: String,
    pub session_generation: String,
    pub x_id: String,
    pub x_epoch: i64,
    pub symbol: String,
    pub projection_kind: String,
    pub change: Change,
    pub plan_hash: String,
    pub certificate_hash: String,
    pub certificate: Certificate,
}
impl ExactCommand {
    fn validate(&self, p: &Profile, epoch: i64) -> Result<(), String> {
        if self.session_id != p.session_id
            || self.session_generation != p.session_generation
            || self.x_id != p.x_id
            || self.x_epoch != epoch
            || self.symbol != p.model.symbol
            || self.projection_kind != p.model.projection_kind
            || self.certificate.rd11_policy != p.model.rd11_policy
        {
            return Err("IdentityConflict：精确命令会话/执行域/模型不符".into());
        }
        let (command, family) = match &self.change {
            Change::Submit { order } | Change::Replace { order } => {
                (&order.command_id, &order.family_id)
            }
            Change::Cancel {
                command_id,
                family_id,
                ..
            } => (command_id, family_id),
        };
        if command != &self.command_id
            || family != &self.logical_order_family
            || command.is_empty()
            || family.is_empty()
            || self.plan_hash != hash(&self.change)
            || self.certificate_hash != hash(&self.certificate)
        {
            return Err("IdentityConflict：精确命令身份/计划/条件摘要不符".into());
        }
        if matches!(&self.change,Change::Submit{order}|Change::Replace{order} if order.projection_manifest.is_empty())
        {
            return Err("MissingDependency：精确命令缺 E 投影清单引用".into());
        }
        let c = &self.certificate;
        if c.rule_revision.is_empty()
            || [
                &c.responsibility_refs,
                &c.structure_dependencies,
                &c.market_dependencies,
            ]
            .iter()
            .any(|a| a.is_empty() || a.iter().any(String::is_empty))
        {
            return Err("MissingDependency：精确命令缺责任/结构/市场/规则依赖".into());
        }
        let expected = [
            ("released-exact/1", "E.ReleaseExact"),
            ("execute-committed/1", "S.RequestConditionalExecution"),
            ("simulation-local-transaction/1", "X.OwnAttemptAndCall"),
        ];
        if c.conditions.len() != expected.len() {
            return Err("Unsupported：不完整或未知条件清单".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for (capability, stage) in expected {
            let matching: Vec<_> = c
                .conditions
                .iter()
                .filter(|x| x.validator_capability == capability)
                .collect();
            if matching.len() != 1 {
                return Err("Unsupported：未知或重复 ConditionEvidence 核验能力".into());
            }
            let x = matching[0];
            if x.condition_id.is_empty()
                || !ids.insert(&x.condition_id)
                || x.authority_revision != capability
                || x.required_check_stages != [stage]
                || x.dependency_set.is_empty()
                || x.dependency_set.iter().any(String::is_empty)
                || x.evidence_refs.is_empty()
                || x.evidence_refs.iter().any(String::is_empty)
                || x.invalidation_sources.is_empty()
                || x.invalidation_sources.iter().any(String::is_empty)
                || x.coverage_proof.is_empty()
                || x.failure_result != "Denied"
            {
                return Err("Unsupported：ConditionEvidence 阶段/覆盖/失败语义未支持".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    op: String,
    payload: Value,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityReply {
    protocol: String,
    session_id: String,
    session_generation: String,
    authority_id: String,
    authority_epoch: String,
    command_id: String,
    command_hash: String,
    command: ExactCommand,
    state: String,
    receipt_id: String,
    release_receipt_id: Option<String>,
    commit_ns: i64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptClock {
    received_ns: i64,
    first_known_ns: i64,
    commit_ns: i64,
}
fn now_ns() -> Result<i64, String> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_nanos(),
    )
    .map_err(error)
}
fn validate_accept_clock(clock: &AcceptClock, release_commit_ns: i64) -> Result<(), String> {
    if release_commit_ns < 0
        || clock.received_ns < 0
        || clock.received_ns > clock.first_known_ns
        || clock.first_known_ns > clock.commit_ns
        || clock.first_known_ns < release_commit_ns
    {
        return Err("InvalidDomain：X 接纳时钟早于 E 持久交付或本地相位倒退".into());
    }
    Ok(())
}
fn audit_accepts(c: &Connection, p: &Profile) -> Result<(), String> {
    let mut q = c
        .prepare("SELECT accept_clock,release_evidence,command FROM x_exact_commands")
        .map_err(error)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (clock, release, command) = row.map_err(error)?;
        let clock: AcceptClock = serde_json::from_str(&clock).map_err(error)?;
        let release: AuthorityReply = serde_json::from_str(&release).map_err(error)?;
        if canonical_json(&serde_json::to_value(&release.command).map_err(error)?) != command {
            return Err("IdentityConflict：恢复交付与命令不符".into());
        }
        let command: ExactCommand = serde_json::from_str(&command).map_err(error)?;
        validate_authority_reply(p, &p.e, "QueryReleasedExact", &command, None, &release)?;
        validate_accept_clock(&clock, release.commit_ns)?;
    }
    Ok(())
}
fn audit_execution(c: &Connection, p: &Profile) -> Result<(), String> {
    let pending = store::pending_batch(c)?;
    let mut required = std::collections::BTreeMap::new();
    let mut q = c
        .prepare("SELECT request FROM x_sim_batches")
        .map_err(error)?;
    let rows = q.query_map([], |r| r.get::<_, String>(0)).map_err(error)?;
    for row in rows {
        let raw = row.map_err(error)?;
        let b: Batch = serde_json::from_str(&raw).map_err(error)?;
        for change in b.close_changes {
            let id = command_id(&change).to_owned();
            if required.insert(id, (raw.clone(), change, false)).is_some() {
                return Err("IdentityConflict：命令跨批重复".into());
            }
        }
    }
    if let Some(raw) = &pending {
        let b: Batch = serde_json::from_str(raw).map_err(error)?;
        let mut normal = b.clone();
        normal.normalize();
        if canonical_json(&serde_json::to_value(normal).map_err(error)?) != *raw
            || store::replay_batch(c, &b)?.is_some()
        {
            return Err("IdentityConflict：pending内容非法或已提交".into());
        }
        store::validate_batch(c, &b)?;
        for change in b.close_changes {
            let id = command_id(&change).to_owned();
            if required.insert(id, (raw.clone(), change, true)).is_some() {
                return Err("IdentityConflict：pending重复已执行命令".into());
            }
        }
    }
    let mut q=c.prepare("SELECT id,command,release_receipt,state,attempt_batch,execute_evidence,release_evidence FROM x_exact_commands").map_err(error)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (id, raw, release, state, attempt, evidence, released) = row.map_err(error)?;
        let command: ExactCommand = serde_json::from_str(&raw).map_err(error)?;
        command.validate(p, command.x_epoch)?;
        let released: AuthorityReply = serde_json::from_str(&released).map_err(error)?;
        if id != command.command_id || released.receipt_id != release {
            return Err("IdentityConflict：接纳身份或交付引用不符".into());
        }
        if let Some((batch, change, is_pending)) = required.remove(&id) {
            if command.change != change {
                return Err("IdentityConflict：批命令与已接纳内容不符".into());
            }
            if is_pending && state == "Accepted" {
                if attempt.is_some() || evidence.is_some() {
                    return Err("IdentityConflict：Accepted混入调用证据".into());
                }
                continue;
            }
            let allowed = if is_pending {
                matches!(
                    state.as_str(),
                    "ExecutionRequested" | "ExecutionReady" | "MayHaveCalled"
                )
            } else {
                state == "Observed"
            };
            if !allowed || attempt.as_deref() != Some(batch.as_str()) {
                return Err("IdentityConflict：批执行状态或占有内容不符".into());
            }
            if state == "ExecutionRequested" {
                if evidence.is_some() {
                    return Err("IdentityConflict：未取得终态却有回执".into());
                }
            } else {
                let evidence: Value = serde_json::from_str(
                    evidence.as_deref().ok_or("IdentityConflict：执行缺S原件")?,
                )
                .map_err(error)?;
                validate_execution_record(p, &command, &release, &evidence)?;
            }
        } else if state != "Accepted" || attempt.is_some() || evidence.is_some() {
            return Err("IdentityConflict：游离执行命令".into());
        }
    }
    if !required.is_empty() {
        return Err("IdentityConflict：工作状态或批次缺已接纳命令权威原件".into());
    }
    Ok(())
}

fn validate_execution_record(
    p: &Profile,
    command: &ExactCommand,
    release: &str,
    evidence: &Value,
) -> Result<AuthorityReply, String> {
    let reply: AuthorityReply = serde_json::from_value(evidence["reply"].clone()).map_err(error)?;
    validate_authority_reply(
        p,
        &p.s,
        "RequestConditionalExecution",
        command,
        Some(release),
        &reply,
    )?;
    let eligibility = match &command.change {
        Change::Submit { order } | Change::Replace { order } => &order.eligibility_ref,
        Change::Cancel {
            eligibility_ref, ..
        } => eligibility_ref,
    };
    if eligibility != &reply.receipt_id
        || evidence["claimed_ns"]
            .as_i64()
            .is_none_or(|t| t < reply.commit_ns)
    {
        return Err("IdentityConflict：持久S执行证据与资格/调用占有时刻不符".into());
    }
    Ok(reply)
}

fn validate_frame(v: Value) -> Result<Value, String> {
    let r: Request = serde_json::from_value(v.clone()).map_err(error)?;
    if r.protocol != WIRE {
        return Err("VersionUnsupported：X wire".into());
    }
    Ok(v)
}
fn json_frame(v: Value) -> Result<Value, String> {
    Ok(v)
}
fn authority(
    p: &Profile,
    a: &Authority,
    op: &str,
    command: &ExactCommand,
    release: Option<&str>,
) -> Result<AuthorityReply, String> {
    let deadline = Instant::now() + Duration::from_millis(p.timeout_ms);
    let mut stream = UnixStream::connect(&a.socket).map_err(io)?;
    let mut bytes=canonical_json(&json!({"protocol":AUTHORITY_WIRE,"op":op,"session_id":p.session_id,"session_generation":p.session_generation,"authority_id":a.authority_id,"authority_epoch":a.authority_epoch,"command_id":command.command_id,"command_hash":hash(command),"command":command,"release_receipt_id":release})).into_bytes();
    bytes.push(b'\n');
    if bytes.len() > p.max_frame_bytes {
        return Err("Unsupported：权威请求超过帧界".into());
    }
    write_frame_observed(&mut stream, &bytes, deadline, &mut NoObserver)?;
    stream.shutdown(std::net::Shutdown::Write).map_err(io)?;
    let reply = read_frame(&mut stream, p.max_frame_bytes, deadline, json_frame)?;
    let r: AuthorityReply = serde_json::from_value(reply).map_err(error)?;
    validate_authority_reply(p, a, op, command, release, &r)?;
    Ok(r)
}

fn validate_authority_reply(
    p: &Profile,
    a: &Authority,
    op: &str,
    command: &ExactCommand,
    release: Option<&str>,
    r: &AuthorityReply,
) -> Result<(), String> {
    if r.protocol != AUTHORITY_WIRE
        || r.session_id != p.session_id
        || r.session_generation != p.session_generation
        || r.authority_id != a.authority_id
        || r.authority_epoch != a.authority_epoch
        || r.command_id != command.command_id
        || r.command_hash != hash(command)
        || r.command != *command
        || r.receipt_id.is_empty()
    {
        return Err("IdentityConflict：权威回执未绑定原命令/内容/会话/身份".into());
    }
    let expected = if op == "QueryReleasedExact" {
        "ReleasedExact"
    } else {
        "ExecuteCommitted"
    };
    if r.commit_ns < 0 {
        return Err("InvalidDomain：权威提交时刻不得为负".into());
    }
    if r.state != expected {
        return Err(format!(
            "MissingDependency：权威结果为{}，需要{expected}",
            r.state
        ));
    }
    if release.is_some() && r.release_receipt_id.as_deref() != release {
        return Err("IdentityConflict：S 收据未绑定本次精确交付".into());
    }
    Ok(())
}

fn get(
    c: &Connection,
    id: &str,
) -> Result<Option<(String, String, String, Option<String>)>, String> {
    c.query_row(
        "SELECT command,release_receipt,state,attempt_batch FROM x_exact_commands WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()
    .map_err(error)
}
fn query_command(c: &Connection, p: &Profile, id: &str) -> Result<Value, String> {
    match get(c, id)? {
        None => Ok(json!({"kind":"ExactCommandState","command_id":id,"state":"NoRecord"})),
        Some((_, release, state, _)) => {
            let (clock, evidence): (String, String) = c
                .query_row(
                    "SELECT accept_clock,release_evidence FROM x_exact_commands WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(error)?;
            let clock: Value = serde_json::from_str(&clock).map_err(error)?;
            let evidence: AuthorityReply = serde_json::from_str(&evidence).map_err(error)?;
            Ok(
                json!({"kind":"ExactCommandState","command_id":id,"state":state,"x_accept":clock,"causal_refs":[{"authority_id":p.e.authority_id,"authority_epoch":p.e.authority_epoch,"receipt_id":release,"command_hash":evidence.command_hash,"release_commit_ns":evidence.commit_ns}]}),
            )
        }
    }
}
fn accept(
    c: &mut Connection,
    p: &Profile,
    epoch: i64,
    command: ExactCommand,
) -> Result<Value, String> {
    if store::pending_batch(c)?.is_some() {
        return Err("PendingBatch：未完成批次只允许同内容恢复或只读".into());
    }
    let received_ns = now_ns()?;
    let raw = canonical_json(&serde_json::to_value(&command).map_err(error)?);
    if let Some((old, _, _, _)) = get(c, &command.command_id)? {
        if raw != old {
            return Err("IdentityConflict：同 command_id 改变内容/epoch".into());
        }
        return query_command(c, p, &command.command_id);
    }
    command.validate(p, epoch)?;
    let released = authority(p, &p.e, "QueryReleasedExact", &command, None)?;
    let clock = AcceptClock {
        received_ns,
        first_known_ns: now_ns()?,
        commit_ns: now_ns()?,
    };
    validate_accept_clock(&clock, released.commit_ns)?;
    c.execute("INSERT INTO x_exact_commands(id,command,release_receipt,state,attempt_batch,accept_clock,release_evidence) VALUES(?1,?2,?3,'Accepted',NULL,?4,?5)",params![command.command_id,raw,released.receipt_id,canonical_json(&serde_json::to_value(&clock).map_err(error)?),canonical_json(&serde_json::to_value(&released).map_err(error)?)]).map_err(error)?;
    query_command(c, p, &command.command_id)
}
fn command_id(change: &Change) -> &str {
    match change {
        Change::Submit { order } | Change::Replace { order } => &order.command_id,
        Change::Cancel { command_id, .. } => command_id,
    }
}
fn execute(c: &mut Connection, p: &Profile, epoch: i64, mut batch: Batch) -> Result<Value, String> {
    batch.normalize();
    let encoded = canonical_json(&serde_json::to_value(&batch).map_err(error)?);
    if store::pending_batch(c)?.is_some_and(|old| old != encoded) {
        return Err("PendingBatch：未完成批次不得由另一批推进".into());
    }
    if let Some(result) = store::replay_batch(c, &batch)? {
        return Ok(result);
    }
    // 完整输入/模型校验必须先于 S 资格消费和 MayHaveCalled。
    store::validate_batch(c, &batch)?;
    let mut ids = std::collections::BTreeSet::new();
    let mut commands = Vec::new();
    for change in &batch.close_changes {
        let id = command_id(change);
        if !ids.insert(id.to_owned()) {
            return Err("IdentityConflict：批内重复 command".into());
        }
        let (raw, release, state, attempt) =
            get(c, id)?.ok_or("MissingDependency：X 尚未接纳精确命令")?;
        if state != "Accepted"
            && !((state == "MayHaveCalled"
                || state == "ExecutionReady"
                || state == "ExecutionRequested")
                && attempt.as_deref()
                    == Some(canonical_json(&serde_json::to_value(&batch).map_err(error)?).as_str()))
        {
            return Err("IdentityConflict：该 command 已作用于另一批次".into());
        }
        let command: ExactCommand = serde_json::from_str(&raw).map_err(error)?;
        command.validate(p, epoch)?;
        if command.change != *change {
            return Err("IdentityConflict：批次改变精确命令".into());
        }
        commands.push((command, release, state));
    }
    let pending_tx = c.transaction().map_err(error)?;
    pending_tx.execute("INSERT INTO x_sim_meta(k,v) VALUES('pending_batch',?1) ON CONFLICT(k) DO UPDATE SET v=excluded.v",[&encoded]).map_err(error)?;
    pending_tx.execute("INSERT INTO x_sim_meta(k,v) VALUES('pending_epoch',?1) ON CONFLICT(k) DO UPDATE SET v=excluded.v",[epoch.to_string()]).map_err(error)?;
    pending_tx.commit().map_err(error)?;
    let mut executed_records = Vec::new();
    for (command, release, state) in commands {
        // AcceptExact 不产生调用权；在真正模型动作前才取得本命令 S 终态。
        let executed = if state == "MayHaveCalled" || state == "ExecutionReady" {
            let raw: String = c
                .query_row(
                    "SELECT execute_evidence FROM x_exact_commands WHERE id=?1",
                    [&command.command_id],
                    |r| r.get(0),
                )
                .map_err(error)?;
            let evidence: Value = serde_json::from_str(&raw).map_err(error)?;
            validate_execution_record(p, &command, &release, &evidence)?
        } else if state == "ExecutionRequested" {
            authority(p, &p.s, "QueryExecuteCommitted", &command, Some(&release))?
        } else {
            c.execute("UPDATE x_exact_commands SET state='ExecutionRequested',attempt_batch=?2 WHERE id=?1 AND state='Accepted'",params![command.command_id,encoded]).map_err(error)?;
            authority(
                p,
                &p.s,
                "RequestConditionalExecution",
                &command,
                Some(&release),
            )?
        };
        let eligibility = match &command.change {
            Change::Submit { order } | Change::Replace { order } => &order.eligibility_ref,
            Change::Cancel {
                eligibility_ref, ..
            } => eligibility_ref,
        };
        if eligibility != &executed.receipt_id {
            return Err("IdentityConflict：命令资格引用未绑定 S 实际提交收据".into());
        }
        let claimed_ns = now_ns()?;
        if claimed_ns < executed.commit_ns {
            return Err("InvalidDomain：X 调用占有早于 S 持久提交".into());
        }
        let evidence = json!({"reply":executed,"claimed_ns":claimed_ns});
        c.execute("UPDATE x_exact_commands SET state='ExecutionReady',attempt_batch=?2,execute_evidence=COALESCE(execute_evidence,?3) WHERE id=?1 AND state IN ('ExecutionRequested','ExecutionReady')",params![command.command_id,encoded,canonical_json(&evidence)]).map_err(error)?;
        executed_records.push((command.command_id, evidence));
    }
    let tx = c.transaction().map_err(error)?;
    tx.execute("INSERT INTO x_sim_meta(k,v) VALUES('pending_batch',?1) ON CONFLICT(k) DO UPDATE SET v=excluded.v",[&encoded]).map_err(error)?;
    for (id, evidence) in &executed_records {
        let changed=tx.execute("UPDATE x_exact_commands SET state='MayHaveCalled',attempt_batch=?2,execute_evidence=COALESCE(execute_evidence,?3) WHERE id=?1 AND (state='ExecutionReady' OR (state='MayHaveCalled' AND attempt_batch=?2))",params![id,encoded,canonical_json(evidence)]).map_err(error)?;
        if changed != 1 {
            return Err("IdentityConflict：X 同命令调用占有冲突".into());
        }
    }
    tx.commit().map_err(error)?;
    #[cfg(test)]
    if std::env::var("ISSUE1381_TEST_CRASH_AFTER_CLAIM")
        .ok()
        .as_deref()
        == Some(batch.request_id.as_str())
    {
        std::process::exit(43);
    }
    // 仅本地模拟 adapter 有已验证的同批 SQLite 幂等合同；此恢复许可不适用于网络场所。
    store::commit_batch_with_commands(c, &batch, &ids.into_iter().collect::<Vec<_>>())
}

fn validate_pending_epoch(c: &Connection, epoch: i64) -> Result<(), String> {
    let saved: Option<String> = c
        .query_row(
            "SELECT v FROM x_sim_meta WHERE k='pending_epoch'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    if store::pending_batch(c)?.is_some() {
        let saved: i64 = saved
            .ok_or("IdentityConflict：pending缺原writer epoch")?
            .parse()
            .map_err(error)?;
        if saved != epoch {
            return Err("PendingBatch：不兼容pending换代须在writer epoch写入前拒绝".into());
        }
    } else if saved.is_some() {
        return Err("IdentityConflict：游离pending epoch".into());
    }
    Ok(())
}

/// 在 bin 写入 writer_epoch 之前检查已有冻结合同与接纳记录。
pub fn validate_existing(c: &Connection, p: &Profile, epoch: i64) -> Result<(), String> {
    p.validate()?;
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='x_sim_meta')",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    if exists {
        validate_pending_epoch(c, epoch)?;
        store::audit_history(c)?;
        let frozen: Option<String> = c
            .query_row("SELECT v FROM x_sim_meta WHERE k='contract'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(error)?;
        let expected = canonical_json(
            &json!({"model":p.model,"source_namespace":p.source_namespace,"source_epoch":p.source_epoch}),
        );
        if frozen.as_ref().is_some_and(|s| s != &expected) {
            return Err("IdentityConflict：X 模型或事实来源已冻结".into());
        }
        let prior: Option<String> = c
            .query_row(
                "SELECT v FROM x_sim_meta WHERE k='authority_profile'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        let binding = canonical_json(&serde_json::to_value(p).map_err(error)?);
        if prior.as_ref() != Some(&binding) {
            return Err("IdentityConflict：X 权威端点合同已冻结".into());
        }
    }
    let commands:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='x_exact_commands')",[],|r|r.get(0)).map_err(error)?;
    if commands {
        audit_accepts(c, p)?;
        audit_execution(c, p)?;
    } else if exists {
        return Err("IdentityConflict：模拟历史缺命令权威表".into());
    }
    Ok(())
}

/// 必须由调用方持有SQLite事务；X launcher把自身meta/epoch纳入同一次提交。
pub fn initialize_in_transaction(
    c: &rusqlite::Transaction<'_>,
    p: &Profile,
    epoch: i64,
) -> Result<(), String> {
    p.validate()?;
    validate_existing(c, p, epoch)?;
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='x_sim_meta')",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    if exists {
        return Ok(());
    }
    store::initialize_in_transaction(c, &p.model, &p.source_namespace, &p.source_epoch)?;
    c.execute_batch("CREATE TABLE x_exact_commands(id TEXT PRIMARY KEY,command TEXT NOT NULL,release_receipt TEXT NOT NULL,state TEXT NOT NULL,attempt_batch TEXT,accept_clock TEXT NOT NULL,release_evidence TEXT NOT NULL,execute_evidence TEXT) STRICT;").map_err(error)?;
    c.execute(
        "INSERT INTO x_sim_meta VALUES('authority_profile',?1)",
        [canonical_json(&serde_json::to_value(p).map_err(error)?)],
    )
    .map_err(error)?;
    Ok(())
}

pub fn serve(c: &mut Connection, p: &Profile, epoch: i64) -> Result<(), String> {
    let tx = c.transaction().map_err(error)?;
    initialize_in_transaction(&tx, p, epoch)?;
    #[cfg(test)]
    if std::env::var("ISSUE1381_TEST_INIT_INTERRUPT")
        .ok()
        .as_deref()
        == Some("before")
    {
        std::process::exit(44);
    }
    tx.commit().map_err(error)?;
    #[cfg(test)]
    if std::env::var("ISSUE1381_TEST_INIT_INTERRUPT")
        .ok()
        .as_deref()
        == Some("after")
    {
        std::process::exit(45);
    }
    let timeout = Duration::from_millis(p.timeout_ms);
    serve_unix(
        Path::new(&p.socket),
        TransportBounds {
            frame: p.max_frame_bytes,
            read: timeout,
            write: timeout,
            response: timeout.saturating_mul(4),
            queue: 16,
            connections: 16,
        },
        validate_frame,
        |v| {
            let result = (|| {
                let r: Request = serde_json::from_value(v.clone()).map_err(error)?;
                match r.op.as_str() {
                    "AcceptExact" => accept(
                        c,
                        p,
                        epoch,
                        serde_json::from_value(r.payload).map_err(error)?,
                    ),
                    "AdvanceBar" => execute(
                        c,
                        p,
                        epoch,
                        serde_json::from_value(r.payload).map_err(error)?,
                    ),
                    "QueryCommand" => {
                        let id = r
                            .payload
                            .as_str()
                            .ok_or("InvalidDomain：QueryCommand 身份须字符串")?;
                        query_command(c, p, id)
                    }
                    "QueryBatch" => store::read_batch(
                        c,
                        r.payload
                            .as_str()
                            .ok_or("InvalidDomain：QueryBatch 身份须字符串")?,
                    ),
                    "QueryOriginal" => store::read_original(
                        c,
                        r.payload
                            .as_str()
                            .ok_or("InvalidDomain：QueryOriginal 身份须字符串")?,
                    ),
                    _ => Err("UnsupportedCapability：未知 X 操作".into()),
                }
            })();
            match result {
                Ok(v) => v,
                Err(e) => json!({"kind":"Error","error":e}),
            }
        },
    )
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn acceptance_clock_rejects_exactly_one_ns_early() {
        let mut c = AcceptClock {
            received_ns: 98,
            first_known_ns: 99,
            commit_ns: 101,
        };
        assert!(validate_accept_clock(&c, 100)
            .unwrap_err()
            .starts_with("InvalidDomain"));
        c.first_known_ns = 100;
        assert!(validate_accept_clock(&c, 100).is_ok());
    }
}
