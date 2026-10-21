//! #1406 C5 / OQ-4：已完成 TestOnly 分拆原件的金额登记与一次目标应用。
//! E.facts 中不可变 Claim 是身份占用，不是 Activated；本模块不执行 G 推导、七态托管或退款。
use super::book::{add, array, hash, integer, target};
use super::store::{self, Config};
use crate::s_content::job::Job;
use crate::session_protocol::{nonnegative, positive, required, strict_json};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) fn is_in(event: &Value) -> bool {
    event["transfer"]["role"] == "in"
}
pub(super) fn is_transfer(event: &Value) -> bool {
    event.get("transfer").is_some()
}
fn identity(event: &Value) -> Value {
    json!({"source_namespace":event["source"]["source_namespace"],"source_epoch":event["source"]["source_epoch"],"external_fact_id":event["source"]["external_fact_id"],"allocation_id":event["allocation_id"]})
}
fn identity_key(id: &Value) -> Result<String, String> {
    super::voice::exact(
        id,
        &[
            "source_namespace",
            "source_epoch",
            "external_fact_id",
            "allocation_id",
        ],
    )?;
    for k in [
        "source_namespace",
        "source_epoch",
        "external_fact_id",
        "allocation_id",
    ] {
        required(id, k)?;
    }
    Ok(hash(&json!([
        id["source_namespace"],
        id["source_epoch"],
        id["external_fact_id"]
    ])))
}
fn amounts(v: &Value) -> Result<[i64; 3], String> {
    Ok([
        nonnegative(required(v, "cash")?, "transfer.cash")?,
        nonnegative(required(v, "units")?, "transfer.units")?,
        nonnegative(required(v, "carrying_cost")?, "transfer.carrying_cost")?,
    ])
}
/// 当前事件来源索引核验沿用 book::decode，不接受跨事件借来的来源指针。
pub(super) fn decode(
    event: &Value,
    pointer: &str,
    check_ref: &dyn Fn(&str, &str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(lot) = event.get("completed_growth") {
        super::voice::exact(
            lot,
            &[
                "profile",
                "lot_id",
                "kind",
                "cash",
                "units",
                "carrying_cost",
                "authorization_ref",
                "source_ref",
            ],
        )?;
        if lot["profile"] != "TestOnly" {
            return Err("SchemaUnsupported：增长分拆仅已完成 TestOnly 原件".into());
        }
        required(lot, "lot_id")?;
        required(lot, "authorization_ref")?;
        check_ref(
            required(lot, "source_ref")?,
            &format!("{pointer}/completed_growth"),
            lot,
        )?;
        let [cash, units, cost] = amounts(lot)?;
        if event.get("observed_existing_book").is_some()
            || is_transfer(event)
            || event.get("q_rebase").is_some()
        {
            return Err("SchemaUnsupported：增长来源批不能来自开局、转移或重基".into());
        }
        match required(lot, "kind")? {
            "cash_capitalization" => {
                let comps = array(event, "known_cost_components")?;
                if units != 0
                    || cost != 0
                    || cash == 0
                    || comps.len() != 1
                    || comps[0]["kind"] != "growth_capitalization"
                    || integer(&comps[0], "canonical_amount")? != cash
                {
                    return Err("IdentityConflict：现金增长批只能对应本件已完成资本化金额".into());
                }
            }
            "units_split" => {
                let du = integer(&event["deltas"], "units")?;
                if cash != 0 || units == 0 || du <= 0 || units > du {
                    return Err("Unsupported：本片筹码增长批仅具名已完成正增量中的分拆".into());
                }
                if ["Pi", "A", "W"].iter().any(|k| event["deltas"][k] != "0") {
                    return Err("Unsupported：增长筹码分拆不能夹带收益或资本化".into());
                }
            }
            _ => return Err("Unsupported：未实现的增长来源类型".into()),
        }
    }
    let Some(t) = event.get("transfer") else {
        if event.get("transfer_opening").is_some() {
            return Err("SchemaUnsupported：转移开局缺 transfer".into());
        }
        return Ok(());
    };
    super::voice::exact(t, &["role", "claim", "source_ref"])?;
    let role = required(t, "role")?;
    if !["out", "in"].contains(&role) {
        return Err("SchemaUnsupported：转移角色仅 out/in".into());
    }
    check_ref(
        required(t, "source_ref")?,
        &format!("{pointer}/transfer"),
        t,
    )?;
    let c = &t["claim"];
    super::voice::exact(
        c,
        &[
            "revision",
            "profile",
            "transfer_id",
            "kind",
            "source_chong",
            "target_chong",
            "source_fact",
            "target_fact",
            "payload",
            "units",
            "growth_source",
            "trigger_evidence_ref",
            "authorization_ref",
            "policy_versions",
        ],
    )?;
    if c["revision"] != "tb03c-transfer/1"
        || c["profile"] != "TestOnly"
        || c["kind"] != "capital_overflow"
    {
        return Err(
            "Unsupported：仅 tb03c-transfer/1 已完成 TestOnly capital_overflow 金额片".into(),
        );
    }
    for k in [
        "transfer_id",
        "source_chong",
        "target_chong",
        "trigger_evidence_ref",
        "authorization_ref",
    ] {
        required(c, k)?;
    }
    if c["source_chong"] == c["target_chong"] {
        return Err("IdentityConflict：转移源目标不可相同".into());
    }
    identity_key(&c["source_fact"])?;
    identity_key(&c["target_fact"])?;
    if identity_key(&c["source_fact"])? == identity_key(&c["target_fact"])? {
        return Err("IdentityConflict：转移两腿原事实必须不同".into());
    }
    super::voice::exact(&c["growth_source"], &["fact", "lot_id"])?;
    identity_key(&c["growth_source"]["fact"])?;
    required(&c["growth_source"], "lot_id")?;
    super::voice::exact(&c["policy_versions"], &["quantity_policy_ref"])?;
    required(&c["policy_versions"], "quantity_policy_ref")?;
    super::voice::exact(&c["payload"], &["cash", "units", "carrying_cost"])?;
    let [cash, units, cost] = amounts(&c["payload"])?;
    if cash == 0 && units == 0 || units == 0 && cost != 0 {
        return Err("InvalidDomain：空转移或无筹码却有携带成本".into());
    }
    add(cash, cost)?;
    let (owner, leg) = if role == "out" {
        ("source_chong", "source_fact")
    } else {
        ("target_chong", "target_fact")
    };
    if c[owner] != target(event)? || c[leg] != identity(event) {
        return Err("IdentityConflict：转移角色与原 owner/事实/allocation 不符".into());
    }
    if event.get("observed_existing_book").is_some()
        || event.get("corporate_action").is_some()
        || event.get("valuation_observation").is_some()
        || event.get("q_rebase").is_some()
        || event.get("depends_on").is_some()
        || !array(event, "known_cost_components")?.is_empty()
        || ["Pi", "A", "W"].iter().any(|k| event["deltas"][k] != "0")
        || integer(&event["deltas"], "units")? != if role == "out" { -units } else { units }
    {
        return Err("SchemaUnsupported：转移原件仅承载具名转移量，不混合成交/本金/通用依赖".into());
    }
    if role == "in" {
        let opening = event
            .get("transfer_opening")
            .ok_or("MissingDependency：目标转移缺冻结 TestOnly 开局语义")?;
        super::voice::exact(
            opening,
            &["symbol", "op_level", "fixed_q", "side", "source_ref"],
        )?;
        required(opening, "symbol")?;
        nonnegative(required(opening, "op_level")?, "transfer level")?;
        positive(required(opening, "fixed_q")?, "transfer fixed_q")?;
        if opening["side"] != "long" {
            return Err("Unsupported：本片转移目标仅多头金额账".into());
        }
        check_ref(
            required(opening, "source_ref")?,
            &format!("{pointer}/transfer_opening"),
            opening,
        )?;
    } else if event.get("transfer_opening").is_some() {
        return Err("SchemaUnsupported：源腿不能携带目标开局".into());
    }
    Ok(())
}

pub(super) fn check_units(doc: &Value, event: &Value) -> Result<(), String> {
    if is_transfer(event) && event["transfer"]["claim"]["units"] != doc["units"] {
        return Err("IdentityConflict：转移 claim 与当前原件量纲不同".into());
    }
    Ok(())
}
pub(super) fn check_capture(config: &Config, event: &Value) -> Result<(), String> {
    if !is_transfer(event) {
        return Ok(());
    }
    let c = &event["transfer"]["claim"];
    let source_route = config.route(required(c, "source_chong")?)?;
    let route = config.route(required(c, "target_chong")?)?;
    if integer(&c["payload"], "units")? > 0 && source_route["symbol"] != route["symbol"] {
        return Err("IdentityConflict：筹码转移不能改变标的".into());
    }
    if is_in(event) {
        let opening = &event["transfer_opening"];
        if opening["symbol"] != route["symbol"]
            || opening["op_level"] != route["operation_level"]
            || opening["fixed_q"] != route["fixed_q"]
        {
            return Err("IdentityConflict：目标转移开局与冻结 manifest 不符".into());
        }
    }
    Ok(())
}
/// E 的持久事实就是占用正本。写入与 audit 重放在同一函数核同一不变式。
pub(super) fn check_claim(records: &[Value], record: &Value) -> Result<(), String> {
    let event = &record["event"];
    if !is_transfer(event) {
        return Ok(());
    }
    let transfer = &event["transfer"];
    for old in records.iter().filter(|r| {
        r["event"]["transfer"]["claim"]["transfer_id"] == transfer["claim"]["transfer_id"]
    }) {
        let previous = &old["event"]["transfer"];
        if previous["claim"] != transfer["claim"] || previous["role"] == transfer["role"] {
            return Err(
                "IdentityConflict：transfer_id 已持久绑定原 tuple/角色事实，不能重新激活或改目标"
                    .into(),
            );
        }
    }
    Ok(())
}
pub(super) fn waiting_on(records: &[Value], event: &Value) -> Result<Option<String>, String> {
    if !is_in(event) {
        return Ok(None);
    }
    let claim = &event["transfer"]["claim"];
    let key = identity_key(&claim["source_fact"])?;
    match records.iter().find(|r| r["fact_key"] == key) {
        None => Ok(Some(key)),
        Some(r)
            if r["event"]["transfer"]["role"] == "out"
                && r["event"]["transfer"]["claim"] == *claim =>
        {
            Ok(None)
        }
        Some(_) => Err("IdentityConflict：转移具名源事实不是对应源腿".into()),
    }
}

/// 源 B 的每个增长批必须来自本 owner 已应用原件；所有源腿按同批逐分量核消费上限。
pub(super) fn source_effect(image: &Value, event: &Value, book: &mut Value) -> Result<(), String> {
    if event["transfer"]["role"] != "out" {
        return Ok(());
    }
    if book["side"] != "long" {
        return Err("Unsupported：空头转移不在 C5 成功域".into());
    }
    let claim = &event["transfer"]["claim"];
    let growth = &claim["growth_source"];
    let key = identity_key(&growth["fact"])?;
    let source = array(image, "recorded")?
        .iter()
        .find(|r| r["fact_key"] == key)
        .ok_or("AwaitingDependency：源 B 尚未持久应用具名增长来源")?;
    let source_event = &source["event"];
    if identity(source_event) != growth["fact"]
        || target(source_event)? != target(event)?
        || source_event["completed_growth"]["lot_id"] != growth["lot_id"]
    {
        return Err("IdentityConflict：增长批不属于原重/原事件/原分量".into());
    }
    if !array(image, "applied")?.iter().any(|a| {
        a["fact_key"] == key
            && a["allocation_id"] == growth["fact"]["allocation_id"]
            && a["status"] == "Applied"
    }) {
        return Err("AwaitingDependency：增长原件未 Applied".into());
    }
    let available = amounts(&source_event["completed_growth"])?;
    let mut spent = amounts(&claim["payload"])?;
    for previous in array(image, "recorded")? {
        let old = &previous["event"]["transfer"];
        if old["role"] == "out"
            && old["claim"]["growth_source"] == *growth
            && array(image, "applied")?
                .iter()
                .any(|a| a["fact_key"] == previous["fact_key"] && a["status"] == "Applied")
        {
            let prior = amounts(&old["claim"]["payload"])?;
            for i in 0..3 {
                spent[i] = add(spent[i], prior[i])?;
            }
        }
    }
    if spent
        .iter()
        .zip(available)
        .any(|(used, total)| *used > total)
    {
        return Err("InvalidDomain：具名增长批的转移累计消耗超额".into());
    }
    let prior = book
        .get("transfer_cost_out")
        .map(|v| {
            crate::session_protocol::parse_canonical_i64(
                v.as_str().ok_or("StorageUnavailable：transfer cost 形状")?,
                "transfer_cost_out",
            )
        })
        .transpose()?
        .unwrap_or(0);
    book["transfer_cost_out"] =
        json!(add(prior, integer(&claim["payload"], "carrying_cost")?)?.to_string());
    Ok(())
}
pub(super) fn target_effect(event: &Value, record: &Value, book: &mut Value) -> Result<(), String> {
    if !is_in(event) {
        return Ok(());
    }
    let c = &event["transfer"]["claim"];
    let p = &c["payload"];
    let cost = integer(p, "carrying_cost")?;
    book["transfer_cost_in"] = json!(cost.to_string());
    book["principal"]["batches"] = json!([{"batch_id":format!("P0:{}",required(c,"transfer_id")?),"amount":add(integer(p,"cash")?,cost)?.to_string(),"authorization_ref":c["authorization_ref"],"fact_key":record["fact_key"],"source_ref":event["transfer"]["source_ref"],"source_transfer_id":c["transfer_id"],"known_at":record["b_observation"]["first_known_ns"]}]);
    Ok(())
}
fn source_config(config: &Config, claim: &Value) -> Result<Config, String> {
    let id = required(claim, "source_chong")?;
    let route = config.route(id)?;
    let mut source = config.clone();
    source.domain = format!("B:{id}");
    source.producer = format!("{}:{}", source.domain, source.session);
    source.db = PathBuf::from(required(route, "db")?);
    source.socket = PathBuf::from(required(route, "socket")?);
    Ok(source)
}
/// 目标写事务之外只读实取固定源 owner；从不接收 caller 提供的派生扣账证明。
pub(super) fn attach_source_proof(
    config: &Config,
    record: &mut Value,
    mut import: Option<(&mut super::content_store::Stage<'_>, &mut Job)>,
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    if !is_in(&record["event"]) {
        return Ok(());
    }
    #[cfg(test)]
    let mut injected = if import.is_none() {
        super::tb03c_c5_read_tests::take_job()
    } else {
        None
    };
    let shared_job = import.as_mut().map(|(_, job)| &mut **job);
    #[cfg(test)]
    let shared_job = shared_job.or_else(|| injected.as_mut().and_then(|g| g.job.as_mut()));
    let mut budget = store::ReadBudget::new(shared_job, deadline);
    budget.before()?;
    if let Some(job) = budget.job.as_deref_mut() {
        job.charge_requests(1).map_err(|e| e.to_string())?;
    }
    let claim = &record["event"]["transfer"]["claim"];
    let source = source_config(config, claim)?;
    let (mut c, _) = store::open_metered(&source.db, true, &mut budget)?;
    let tx = c.transaction().map_err(store::err)?;
    store::audit_with_budget(&tx, &source, &mut budget)?;
    let r = store::metered_row(
        &tx,
        "receipts",
        required(&claim["source_fact"], "allocation_id")?,
        &mut budget,
    )?
    .ok_or("AwaitingDependency：源 B 的具名扣账尚未 Applied")?;
    if r["kind"] != "Applied" || r["fact_key"] != identity_key(&claim["source_fact"])? {
        return Err("AwaitingDependency：源 B 具名扣账不是 Applied".into());
    }
    let seq = positive(required(&r["cut"], "commit_seq")?, "source seq")?;
    let mut read = |seq: i64| -> Result<(String, String, Value), String> {
        let (prev, ns, raw): (String, String, String) = budget.one(
            &tx,
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok((prev, ns, strict_json(raw.as_bytes())?))
    };
    let (prev, ns, image) = read(seq)?;
    let (pprev, pns, pimage) = read(seq - 1)?;
    let proof = json!({"revision":"tb03c-source-debit-proof/1","domain_id":source.domain,"session_id":source.session,"session_generation":source.generation,"receipt":r,"previous_root":prev,"commit_ns":ns,"image":image,"previous_previous_root":pprev,"previous_commit_ns":pns,"previous_image":pimage});
    record["b_source_debit_proof"] = proof;
    #[cfg(test)]
    super::tb03c_c5_read_tests::proof_read(record);
    verify_source_proof(config, record)?;
    budget.before()?;
    drop(budget);
    if let Some((stage, job)) = import {
        store::check_operation_deadline(Some(&mut *job), deadline)?;
        let mut refs = super::content_store::image_refs(&record["b_source_debit_proof"]["image"])
            .map_err(|e| e.to_string())?;
        refs.extend(
            super::content_store::image_refs(&record["b_source_debit_proof"]["previous_image"])
                .map_err(|e| e.to_string())?,
        );
        super::content_store::import_peer_refs_with_deadline(&tx, &refs, stage, job, deadline)
            .map_err(|e| format!("MissingDependency：源扣账本地闭包导入失败：{e}"))?;
    }
    store::check_operation_deadline(None, deadline)?;
    tx.commit().map_err(store::err)?;
    Ok(())
}
/// 恢复只核本地保存的证明，不请求当前 E/源 B；源 Applied 必须等于同一 reducer 的真实扣账。
pub(super) fn verify_source_proof(config: &Config, record: &Value) -> Result<(), String> {
    if !is_in(&record["event"]) {
        if record.get("b_source_debit_proof").is_some() {
            return Err("IdentityConflict：非转入原件不能携带源扣账证明".into());
        }
        return Ok(());
    }
    let proof = record
        .get("b_source_debit_proof")
        .ok_or("AwaitingDependency：目标尚无本地源 Applied 证明")?;
    super::voice::exact(
        proof,
        &[
            "revision",
            "domain_id",
            "session_id",
            "session_generation",
            "receipt",
            "previous_root",
            "commit_ns",
            "image",
            "previous_previous_root",
            "previous_commit_ns",
            "previous_image",
        ],
    )?;
    let claim = &record["event"]["transfer"]["claim"];
    let source = source_config(config, claim)?;
    if proof["revision"] != "tb03c-source-debit-proof/1"
        || proof["domain_id"] != source.domain
        || proof["session_id"] != source.session
        || proof["session_generation"] != source.generation
    {
        return Err("IdentityConflict：源扣账证明 owner/会话不符".into());
    }
    let receipt = &proof["receipt"];
    let seq = positive(required(&receipt["cut"], "commit_seq")?, "source seq")?;
    let ns = required(proof, "commit_ns")?;
    let previous = required(proof, "previous_root")?;
    let image = &proof["image"];
    let prior = &proof["previous_image"];
    let original = array(image, "recorded")?
        .last()
        .ok_or("MissingDependency：源扣账提交缺原件")?;
    if original["event"]["transfer"]["role"] != "out"
        || original["event"]["transfer"]["claim"] != *claim
        || identity(&original["event"]) != claim["source_fact"]
    {
        return Err("IdentityConflict：证明不是同 transfer 的具名源扣账".into());
    }
    store::verify_record(original, source.book_rules()?)?;
    store::verify_e_copy(original, &source)?;
    let cut = store::cut(
        &source,
        seq,
        &store::root(&source, seq, previous, ns, image),
    );
    if previous
        != store::root(
            &source,
            seq - 1,
            required(proof, "previous_previous_root")?,
            required(proof, "previous_commit_ns")?,
            prior,
        )
        || original["b_observation"]["commit_ns"] != ns
        || super::book::apply_in(&source.manifest, prior, original, seq, ns)?.0 != *image
    {
        return Err("IdentityConflict：源扣账前后提交不等于实际金额效果".into());
    }
    let observation = &original["b_observation"];
    let expected = json!({"kind":"Applied","domain_id":source.domain,"fact_key":original["fact_key"],"fact_hash":original["fact_hash"],"allocation_id":original["event"]["allocation_id"],"target_domain":source.domain,"received_ns":observation["received_ns"],"first_known_ns":observation["first_known_ns"],"commit_ns":ns,"e_first_known_ns":original["first_known_ns"],"cut":cut});
    if *receipt != expected {
        return Err("IdentityConflict：源 Applied 回执不等于实际扣账提交".into());
    }
    Ok(())
}
