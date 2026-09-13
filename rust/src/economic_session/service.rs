use super::book::{apply, decode, fact_key, hash, target};
use super::store::*;
use crate::session_protocol::{
    canonical_json, nonnegative, positive, reply_envelope, required, sha256_hex, validate_envelope,
};
use crate::session_transport::{process_lock, serve_unix, TransportBounds};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn validate(v: Value) -> Result<Value, String> {
    validate_envelope(v, SCHEMA)
}
fn exact(v: &Value, keys: &[&str]) -> Result<(), String> {
    let m = v.as_object().ok_or("SchemaUnsupported：请求须对象")?;
    if m.len() != keys.len() || m.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err("SchemaUnsupported：业务字段不完整或多余".into());
    }
    Ok(())
}
fn message_key(e: &Value) -> String {
    hash(&json!([
        e["source_namespace"],
        e["source_epoch"],
        e["message_id"]
    ]))
}
fn previous_message(c: &Connection, e: &Value) -> Result<Option<Value>, String> {
    let r: Option<(String, String, String)> = c
        .query_row(
            "SELECT payload_hash,receipt_key,request_json FROM messages WHERE k=?",
            [message_key(e)],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(err)?;
    if let Some((h, k, original)) = r {
        if e["payload_hash"] != h || canonical_json(e) != original {
            return Err("IdentityConflict：原消息内容改变".into());
        }
        return row(c, "receipts", &k)?
            .map(Some)
            .ok_or_else(|| "StorageUnavailable：域损坏：消息缺回执".into());
    }
    Ok(None)
}
fn bind_message(c: &Connection, e: &Value, k: &str) -> Result<(), String> {
    c.execute(
        "INSERT INTO messages(k,payload_hash,receipt_key,request_json)VALUES(?,?,?,?)",
        params![
            message_key(e),
            required(e, "payload_hash")?,
            k,
            canonical_json(e)
        ],
    )
    .map_err(err)?;
    Ok(())
}
fn clocks(
    config: &Config,
    e: &Value,
    last_ns: &str,
    epoch: i64,
) -> Result<(String, String, String), String> {
    let msg = required(e, "message_id")?;
    let r = config.clock(Some(msg), "received_ns", (epoch - 1) as usize)?;
    let f = config.clock(Some(msg), "first_known_ns", (epoch - 1) as usize)?;
    let c = config.clock(Some(msg), "commit_ns", (epoch - 1) as usize)?;
    if nonnegative(&r, "received_ns")? > nonnegative(&f, "first_known_ns")?
        || nonnegative(&f, "first_known_ns")? > nonnegative(&c, "commit_ns")?
        || nonnegative(&f, "first_known_ns")? < nonnegative(last_ns, "previous commit/recover_ns")?
    {
        return Err("InvalidDomain：实际时钟阶段倒退".into());
    }
    Ok((r, f, c))
}
fn record_from_payload(config: &Config, e: &Value) -> Result<Value, String> {
    let p = &e["payload"];
    exact(
        p,
        &[
            "op",
            "raw_utf8",
            "json_pointer",
            "token",
            "source_event_time",
        ],
    )?;
    let raw = required(p, "raw_utf8")?;
    let pointer = required(p, "json_pointer")?;
    let (doc, event) = decode(raw, pointer)?;
    if event["source"]["source_namespace"] != e["source_namespace"]
        || event["source"]["source_epoch"] != e["source_epoch"]
    {
        return Err("InvalidDomain：来源未获准：原件来源不等于获准连接来源".into());
    }
    let id = target(&event)?;
    let route = config.route(id)?;
    if let Some(b) = event.get("book_identity") {
        if b["symbol"] != route["symbol"]
            || b["op_level"] != route["operation_level"]
            || b["fixed_q"] != route["fixed_q"]
        {
            return Err("IdentityConflict：原非空重与manifest语义/固定Q冲突".into());
        }
    }
    if p["source_event_time"] != event["source"]["source_event_time"] {
        return Err("IdentityConflict：不能以caller值回填来源事件时刻".into());
    }
    let raw_hash = sha256_hex(raw.as_bytes());
    let fact_hash = hash(
        &json!({"raw_sha256":raw_hash,"json_pointer":pointer,"event":event,"units":doc["units"]}),
    );
    Ok(
        json!({"fact_key":fact_key(&event)?,"fact_hash":fact_hash,"raw_sha256":raw_hash,"raw_utf8":raw,"json_pointer":pointer,"event":event,"token":required(p,"token")?,"target_domain":format!("B:{id}"),"source_event_time":p["source_event_time"],"source_received_time":event["source"]["received_time"],"source_producer":{"producer_id":e["producer_id"],"producer_epoch":e["producer_epoch"]},"origin_message":{"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]},"origin_request_hash":hash(e)}),
    )
}
fn capture(c: &mut Connection, config: &Config, epoch: i64, e: &Value) -> Result<Value, String> {
    let mut record = record_from_payload(config, e)?;
    let key = required(&record, "fact_key")?.to_owned();
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    if let Some(r) = previous_message(&tx, e)? {
        return Ok(r);
    }
    if let Some(old) = row(&tx, "facts", &key)? {
        for k in [
            "fact_hash",
            "token",
            "target_domain",
            "source_event_time",
            "source_producer",
        ] {
            if old[k] != record[k] {
                return Err(format!("IdentityConflict：原事实{k}改变"));
            }
        }
        let receipt =
            row(&tx, "receipts", &key)?.ok_or("StorageUnavailable：域损坏：原事实缺回执")?;
        bind_message(&tx, e, &key)?;
        tx.commit().map_err(err)?;
        return Ok(receipt);
    }
    let token = required(&record, "token")?;
    if rows(&tx, "pending")?.iter().any(|r| r["token"] == token) {
        return Err("IdentityConflict：token已绑定另一原事实".into());
    }
    let (seq, prev, last_ns, mut image) = latest(&tx)?;
    let seq = seq.checked_add(1).ok_or("InvalidDomain：commit seq溢出")?;
    let recovered_ns: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last_ns, "last commit")?
        .max(nonnegative(&recovered_ns, "epoch at_ns")?)
        .to_string();
    let (received, first, ns) = clocks(config, e, &floor, epoch)?;
    record["received_ns"] = json!(received);
    record["first_known_ns"] = json!(first);
    record["commit_ns"] = json!(ns);
    record["recorded_seq"] = json!(seq.to_string());
    let pending = json!({"fact_key":key,"token":record["token"],"target_domain":record["target_domain"],"status":"Pending","reason":"No Resolve/Reduce evidence in TB-03-A","first_known_ns":first,"recorded_seq":seq.to_string()});
    image["recorded"]
        .as_array_mut()
        .unwrap()
        .push(record.clone());
    image["pending"]
        .as_array_mut()
        .unwrap()
        .push(pending.clone());
    image["history"].as_array_mut().unwrap().push(json!({"kind":"CaptureExternal","fact_key":key,"commit_seq":seq.to_string(),"commit_ns":ns}));
    put(&tx, "facts", &key, &record)?;
    put(&tx, "pending", &key, &pending)?;
    let cut = append(&tx, config, seq, &prev, &ns, &image)?;
    let receipt = json!({"kind":"Recorded","domain_id":config.domain,"fact_key":key,"fact_hash":record["fact_hash"],"allocation_id":record["event"]["allocation_id"],"target_domain":record["target_domain"],"token":record["token"],"first_known_ns":first,"commit_ns":ns,"cut":cut});
    put(&tx, "receipts", &key, &receipt)?;
    bind_message(&tx, e, &key)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
fn authoritative_fact(config: &Config, key: &str, expected_epoch: i64) -> Result<Value, String> {
    let mut ec = config.clone();
    ec.domain = "E".into();
    ec.producer = format!("E:{}", ec.session);
    ec.db = PathBuf::from(required(&ec.manifest["e"], "db")?);
    ec.socket = PathBuf::from(required(&ec.manifest["e"], "socket")?);
    let (mut c, _) = open(&ec.db, true)?;
    let tx = c.transaction().map_err(err)?;
    check_epoch(&tx, expected_epoch)?;
    audit(&tx, &ec)?;
    let mut f = row(&tx, "facts", key)?.ok_or("MissingDependency：固定E权威不存在该原事实")?;
    let r = row(&tx, "receipts", key)?.ok_or("StorageUnavailable：域损坏：固定E缺原回执")?;
    verify_record(&f)?;
    f["cut"] = r["cut"].clone();
    f["e_receipt"] = r;
    f["e_domain_id"] = json!("E");
    f["e_session_id"] = json!(ec.session);
    f["e_session_generation"] = json!(ec.generation);
    let seq = positive(required(&f, "recorded_seq")?, "recorded_seq")?;
    let (previous_root, commit_ns, image): (String, String, String) = tx
        .query_row(
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(err)?;
    f["e_commit_proof"] = json!({"previous_root":previous_root,"commit_ns":commit_ns,"image":crate::session_protocol::strict_json(image.as_bytes())?});
    verify_e_copy(&f, config)?;
    tx.commit().map_err(err)?;
    Ok(f)
}
fn apply_allocation(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e_epoch: i64,
    e: &Value,
) -> Result<Value, String> {
    let p = &e["payload"];
    exact(
        p,
        &["op", "fact_key", "allocation_id", "expected_fact_hash"],
    )?;
    let key = required(p, "fact_key")?;
    let allocation = required(p, "allocation_id")?;
    required(p, "expected_fact_hash")?;
    // #1374：已提交身份先从本域查回，不让当前E故障抹掉原成功。
    {
        let tx = immediate(c)?;
        check_epoch(&tx, epoch)?;
        audit(&tx, config)?;
        if let Some(r) = previous_message(&tx, e)? {
            return Ok(r);
        }
        if let Some(r) = row(&tx, "receipts", allocation)? {
            if r["fact_key"] != key || r["fact_hash"] != p["expected_fact_hash"] {
                return Err("IdentityConflict：allocation已绑定其他内容".into());
            }
            bind_message(&tx, e, allocation)?;
            tx.commit().map_err(err)?;
            return Ok(r);
        }
        if row(&tx, "facts", key)?.is_some() {
            return Err("IdentityConflict：原事实已被别的allocation消费".into());
        }
    }
    // 固定 E 只读审计发生在 B 写事务之外；无跨域写锁、无 ATTACH。
    let mut record = authoritative_fact(config, key, e_epoch)?;
    if record["fact_hash"] != p["expected_fact_hash"]
        || record["event"]["allocation_id"] != allocation
        || record["target_domain"] != config.domain
    {
        return Err("IdentityConflict：实读E事实/金额/归属与请求不匹配".into());
    }
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    if row(&tx, "facts", key)?.is_some() || row(&tx, "receipts", allocation)?.is_some() {
        return Err("IdentityConflict：事实已消费".into());
    }
    let (seq, prev, last_ns, image) = latest(&tx)?;
    let seq = seq.checked_add(1).ok_or("InvalidDomain：commit seq溢出")?;
    let recovered_ns: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last_ns, "last commit")?
        .max(nonnegative(&recovered_ns, "epoch at_ns")?)
        .to_string();
    let (received, first, ns) = clocks(config, e, &floor, epoch)?;
    record["b_observation"] = json!({"received_ns":received,"first_known_ns":first,"commit_ns":ns});
    record["b_origin_message"] = json!({"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]});
    record["b_origin_request_hash"] = json!(hash(e));
    let (next, unmatched) = apply(&image, &record, seq, &ns)?;
    let applied = next["applied"].as_array().unwrap().last().unwrap();
    put(&tx, "facts", key, &record)?;
    tx.execute(
        "INSERT INTO applications(k,fact_key,json)VALUES(?,?,?)",
        params![allocation, key, canonical_json(applied)],
    )
    .map_err(err)?;
    if unmatched {
        put(
            &tx,
            "unmatched",
            allocation,
            next["unmatched"].as_array().unwrap().last().unwrap(),
        )?;
    }
    if !next["book"].is_null() {
        tx.execute("INSERT INTO book(k,json)VALUES('book',?)ON CONFLICT(k)DO UPDATE SET json=excluded.json",[canonical_json(&next["book"])]).map_err(err)?;
    }
    let cut = append(&tx, config, seq, &prev, &ns, &next)?;
    let receipt = json!({"kind":if unmatched{"Unmatched"}else{"Applied"},"domain_id":config.domain,"fact_key":key,"fact_hash":record["fact_hash"],"allocation_id":allocation,"target_domain":config.domain,"received_ns":received,"first_known_ns":first,"commit_ns":ns,"e_first_known_ns":record["first_known_ns"],"cut":cut});
    put(&tx, "receipts", allocation, &receipt)?;
    bind_message(&tx, e, allocation)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
fn bind_voice(c: &mut Connection, config: &Config, epoch: i64, e: &Value) -> Result<Value, String> {
    let p = &e["payload"];
    super::voice::validate_request(p)?;
    let binding = required(p, "binding_id")?;
    let key = super::voice::receipt_key(binding);
    // 原提交优先；恢复与重送不依赖当前外部证据文件仍可读。
    {
        let tx = immediate(c)?;
        check_epoch(&tx, epoch)?;
        audit(&tx, config)?;
        if let Some(r) = previous_message(&tx, e)? {
            return Ok(r);
        }
        let (_, _, _, image) = latest(&tx)?;
        if super::voice::find_binding(&image, binding)?.is_some() {
            let event = super::voice::event_for(&image, binding)?;
            let original = super::voice::original_request(&tx, event, config)?;
            if original["payload"] != *p {
                return Err("IdentityConflict：Voice 原绑定内容改变".into());
            }
            let r = row(&tx, "receipts", &key)?.ok_or("StorageUnavailable：Voice 缺原回执")?;
            bind_message(&tx, e, &key)?;
            tx.commit().map_err(err)?;
            return Ok(r);
        }
        if row(&tx, "receipts", &key)?.is_some() {
            return Err("IdentityConflict：Voice 回执键已属于另一业务".into());
        }
    }
    // 读取固定证据在 B 写事务之外，绝不持生命周期控制锁等待外部依据。
    let verified = super::evidence::load(
        config,
        required(p, "evidence_id")?,
        required(&p["carrier_ref"], "authority_id")?,
    )?;
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    let (seq, previous_root, last_ns, image) = latest(&tx)?;
    let seq = seq.checked_add(1).ok_or("InvalidDomain：commit seq 溢出")?;
    if super::voice::find_binding(&image, binding)?.is_some()
        || row(&tx, "receipts", &key)?.is_some()
    {
        return Err("IdentityConflict：Voice 已被另一次事务绑定".into());
    }
    let recovered_ns: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last_ns, "last commit")?
        .max(nonnegative(&recovered_ns, "epoch at_ns")?)
        .to_string();
    let (received, first, ns) = clocks(config, e, &floor, epoch)?;
    let (mut next, record) =
        super::voice::bind_image(&image, config, p, &verified, &received, &first, &ns, seq)?;
    next["history"]
        .as_array_mut()
        .unwrap()
        .push(super::voice::history_event(
            e,
            &record,
            &received,
            verified.witness(),
        ));
    tx.execute(
        "UPDATE book SET json=? WHERE k='book'",
        [canonical_json(&next["book"])],
    )
    .map_err(err)?;
    let cut = append(&tx, config, seq, &previous_root, &ns, &next)?;
    let receipt = super::voice::receipt(config, &record, cut);
    put(&tx, "receipts", &key, &receipt)?;
    bind_message(&tx, e, &key)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
fn select_cut(
    c: &Connection,
    config: &Config,
    p: &Value,
) -> Result<(i64, String, String, Value), String> {
    if !p["cut"].is_null() && !p["as_known_ns"].is_null() {
        return Err("InvalidDomain：cut与AsKnown不能同时指定".into());
    }
    let selected = if !p["cut"].is_null() {
        let q = &p["cut"];
        exact(q, &["domain_id", "commit_seq", "root_hash"])?;
        if q["domain_id"] != config.domain {
            return Err("IdentityConflict：跨域cut".into());
        }
        nonnegative(required(q, "commit_seq")?, "commit_seq")?
    } else if !p["as_known_ns"].is_null() {
        let n = nonnegative(
            p["as_known_ns"]
                .as_str()
                .ok_or("InvalidDomain：AsKnown须整数文本")?,
            "as_known_ns",
        )?;
        c.query_row(
            "SELECT max(seq) FROM commits WHERE CAST(commit_ns AS INTEGER)<=?",
            [n],
            |r| r.get::<_, Option<i64>>(0),
        )
        .map_err(err)?
        .ok_or("MissingDependency：AsKnown早于域创世")?
    } else {
        latest(c)?.0
    };
    let r: Option<(i64, String, String, String)> = c
        .query_row(
            "SELECT seq,root,commit_ns,image FROM commits WHERE seq=?",
            [selected],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(err)?;
    let (s, h, n, i) = r.ok_or("MissingDependency：指定历史cut不存在")?;
    if !p["cut"].is_null() && p["cut"] != cut(config, s, &h) {
        return Err("IdentityConflict：cut根不匹配".into());
    }
    Ok((s, h, n, crate::session_protocol::strict_json(i.as_bytes())?))
}
fn read(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    runtime: &Value,
    p: &Value,
) -> Result<Value, String> {
    let tx = c.transaction().map_err(err)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    let result = match required(p, "op")? {
        "ReadView" => {
            exact(p, &["op", "cut", "as_known_ns"])?;
            let (s, h, n, image) = select_cut(&tx, config, p)?;
            json!({"kind":"EconomicView","domain_id":config.domain,"cut":cut(config,s,&h),"image_hash":hash(&image),"image":image,"commit_ns":n,"writer_epoch":epoch.to_string(),"runtime":runtime})
        }
        "Watch" => {
            exact(p, &["op", "after_cut"])?;
            let after = if p["after_cut"].is_null() {
                -1
            } else {
                select_cut(
                    &tx,
                    config,
                    &json!({"cut":p["after_cut"],"as_known_ns":null}),
                )?
                .0
            };
            let mut q = tx
                .prepare("SELECT seq,root,commit_ns,image FROM commits WHERE seq>? ORDER BY seq")
                .map_err(err)?;
            let rows = q
                .query_map([after], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(err)?;
            let mut changes = Vec::new();
            for r in rows {
                let (s, h, n, i) = r.map_err(err)?;
                let image = crate::session_protocol::strict_json(i.as_bytes())?;
                changes.push(json!({"cut":cut(config,s,&h),"image_hash":hash(&image),"image":image,"commit_ns":n}));
            }
            let (s, h, _, _) = latest(&tx)?;
            json!({"kind":"EconomicChanges","domain_id":config.domain,"cut":cut(config,s,&h),"changes":changes,"writer_epoch":epoch.to_string()})
        }
        "QueryFact" => {
            exact(p, &["op", "fact_key"])?;
            let k = required(p, "fact_key")?;
            let f = row(&tx, "facts", k)?.ok_or("MissingDependency：原事实不存在")?;
            let receipt_key = if config.domain == "E" {
                k
            } else {
                required(&f["event"], "allocation_id")?
            };
            json!({"kind":"EconomicFact","domain_id":config.domain,"fact":f,"receipt":row(&tx,"receipts",receipt_key)?.ok_or("StorageUnavailable：域损坏：原事实缺回执")?})
        }
        "QueryVoice" => {
            exact(p, &["op", "binding_id"])?;
            if config.domain == "E" {
                return Err("SchemaUnsupported：E 不拥有 Voice".into());
            }
            let binding = required(p, "binding_id")?;
            let (_, _, _, image) = latest(&tx)?;
            let record = super::voice::find_binding(&image, binding)?
                .ok_or("MissingDependency：Voice 尚未绑定")?;
            json!({"kind":"EconomicVoice","domain_id":config.domain,"record":record,"receipt":row(&tx,"receipts",&super::voice::receipt_key(binding))?.ok_or("StorageUnavailable：Voice 缺原回执")?})
        }
        "QueryAllocation" => {
            exact(p, &["op", "allocation_id"])?;
            if config.domain == "E" {
                return Err("SchemaUnsupported：E不拥有allocation".into());
            }
            let k = required(p, "allocation_id")?;
            json!({"kind":"EconomicAllocation","domain_id":config.domain,"application":row(&tx,"applications",k)?.ok_or("MissingDependency：allocation不存在")?,"receipt":row(&tx,"receipts",k)?.ok_or("StorageUnavailable：域损坏：allocation缺回执")?})
        }
        _ => return Err("SchemaUnsupported：未知只读操作".into()),
    };
    tx.commit().map_err(err)?;
    Ok(result)
}
pub(super) fn handle(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e_epoch: i64,
    runtime: &Value,
    e: &Value,
) -> Value {
    let result = (|| {
        config.authorized(e)?;
        let op = required(&e["payload"], "op")?;
        match op {
            "CaptureExternal" if config.domain == "E" => capture(c, config, epoch, e),
            "ApplyAllocation" if config.domain.starts_with("B:") => {
                apply_allocation(c, config, epoch, e_epoch, e)
            }
            "BindVoice" if config.domain.starts_with("B:") => bind_voice(c, config, epoch, e),
            "ReadView" | "Watch" | "QueryFact" | "QueryAllocation" | "QueryVoice" => {
                read(c, config, epoch, runtime, &e["payload"])
            }
            _ => Err("SchemaUnsupported：操作不属于本域".into()),
        }
    })();
    let payload = match result {
        Ok(v) => v,
        Err(error) => json!({"kind":"Error","domain_id":config.domain,"error":error}),
    };
    reply_envelope(
        e,
        payload,
        SCHEMA,
        &config.session,
        &config.generation,
        "economic-session/replies",
        &config.producer,
        &epoch.to_string(),
    )
}
/// 正式 owner 入口。init/recover/serve 共用同一域独占锁；不给客户端创建替代权威路径。
pub fn cli(is_b: bool) -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or(
        "usage: init|serve|recover --manifest FILE --db FILE [--chong ID] [--epoch N --e-epoch N]",
    )?;
    let mut opts = BTreeMap::new();
    while let Some(k) = args.next() {
        if !k.starts_with("--") || opts.contains_key(&k) {
            return Err("InvalidDomain：未知/重复CLI参数".into());
        }
        opts.insert(k, args.next().ok_or("InvalidDomain：参数缺值")?);
    }
    for k in opts.keys() {
        if !["--manifest", "--db", "--chong", "--epoch", "--e-epoch"].contains(&k.as_str()) {
            return Err(format!("InvalidDomain：未知参数{k}"));
        }
    }
    let get = |k: &str| {
        opts.get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("InvalidDomain：缺{k}"))
    };
    let config = Config::load(
        Path::new(get("--manifest")?),
        Path::new(get("--db")?),
        if is_b { Some(get("--chong")?) } else { None },
    )?;
    if !is_b && opts.contains_key("--chong") {
        return Err("InvalidDomain：E不能指定重".into());
    }
    let _lock = process_lock(&config.db, &config.domain)?;
    match command.as_str() {
        "init" => println!("{}", canonical_json(&init(&config)?)),
        "recover" => println!("{}", canonical_json(&recover(&config)?)),
        "serve" => {
            let epoch = positive(get("--epoch")?, "epoch")?;
            let e_epoch = if is_b {
                positive(get("--e-epoch")?, "e_epoch")?
            } else {
                epoch
            };
            let (mut c, runtime) = open(&config.db, false)?;
            check_epoch(&c, epoch)?;
            audit(&c, &config)?;
            let r = &config.manifest["resources"];
            let n = |k: &str| -> Result<usize, String> {
                usize::try_from(positive(required(r, k)?, k)?).map_err(err)
            };
            let bounds = TransportBounds {
                frame: n("max_frame_bytes")?,
                read: Duration::from_millis(n("read_timeout_ms")? as u64),
                write: Duration::from_millis(n("write_timeout_ms")? as u64),
                response: Duration::from_millis(n("response_timeout_ms")? as u64),
                queue: n("queue_capacity")?,
                connections: n("max_connections")?,
            };
            serve_unix(&config.socket, bounds, validate, |e| {
                handle(&mut c, &config, epoch, e_epoch, &runtime, &e)
            })?;
        }
        _ => return Err("InvalidDomain：未知子命令".into()),
    }
    Ok(())
}
