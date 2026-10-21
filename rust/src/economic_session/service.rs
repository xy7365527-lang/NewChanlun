use super::book::{decode, fact_key, hash, target};
use super::store::*;
use crate::s_content::job::Job;
use crate::session_protocol::{
    canonical_json, nonnegative, positive, reply_envelope, required, sha256_hex, validate_envelope,
};
use crate::session_transport::{
    process_lock, serve_unix_with_deadline, EnvelopeValidator, TransportBounds,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

// #1406 C6a：分页是单独、显式授权的只读协议，不接管旧单帧读失败。
const READ_PAGES: &str = "economic-read-pages/1";
const READ_CHUNK: usize = 262_144;
const READ_CACHE: usize = 4 * 1_048_576;
const READ_FRAME: usize = 1_048_576;
const READ_TRANSFERS: usize = 4;
const READ_CHANGES: usize = 32;

struct ReadTransfer {
    open_key: String,
    request_hash: String,
    principal_hash: String,
    descriptor: String,
    descriptor_hash: String,
    document: Option<Vec<u8>>,
    cuts: Vec<StoredImage>,
    deadline: std::time::Instant,
}
impl ReadTransfer {
    fn release_document(&mut self) {
        self.document = None;
        self.descriptor = String::new();
        self.cuts = Vec::new();
    }
    fn retained_bytes(&self, key: &String) -> usize {
        // 原文和所有持有的身份/描述符同账；固定余量覆盖 map 节点等元数据。
        // 这是保守缓存准入，并不冒称进程 RSS 计量。
        std::mem::size_of::<Self>()
            + 512
            + key.capacity()
            + self.open_key.capacity()
            + self.request_hash.capacity()
            + self.principal_hash.capacity()
            + self.descriptor.capacity()
            + self.descriptor_hash.capacity()
            + self.document.as_ref().map_or(0, Vec::capacity)
            + self.cuts.capacity() * std::mem::size_of::<StoredImage>()
            + self
                .cuts
                .iter()
                .map(|c| c.root.capacity() + c.previous_root.capacity() + c.commit_ns.capacity())
                .sum::<usize>()
    }
}
#[derive(Default)]
struct ReadTransfers {
    entries: BTreeMap<String, ReadTransfer>,
}
impl ReadTransfers {
    fn expire(&mut self) {
        let now = std::time::Instant::now();
        for entry in self.entries.values_mut() {
            if now >= entry.deadline {
                entry.release_document();
            }
        }
    }
    fn retained_bytes(&self) -> usize {
        self.entries.iter().map(|(k, v)| v.retained_bytes(k)).sum()
    }
}
fn read_principal(e: &Value) -> String {
    hash(&json!([
        e["session_id"],
        e["session_generation"],
        e["source_namespace"],
        e["source_epoch"],
        e["producer_id"],
        e["producer_epoch"]
    ]))
}
fn read_deadline(p: &Value, ingress: std::time::Instant) -> Result<std::time::Instant, String> {
    let ms = positive(
        required(p, "requested_remaining_ms")?,
        "requested_remaining_ms",
    )?;
    if ms > 30_000 {
        return Err("InvalidDomain：分页剩余窗超过原30秒".into());
    }
    let deadline = std::time::Instant::now()
        .checked_add(Duration::from_millis(ms as u64))
        .ok_or("InvalidDomain：分页截止溢出")?
        .min(ingress);
    check_operation_deadline(None, Some(deadline))?;
    Ok(deadline)
}
fn page_identity(p: &Value) -> Result<(&str, &str), String> {
    let id = required(p, "transfer_id")?;
    let digest = required(p, "descriptor_sha256")?;
    let lower_hex = |s: &str, n| {
        s.len() == n
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if !lower_hex(id, 32) || !lower_hex(digest, 64) {
        return Err("SchemaUnsupported：分页身份不是固定小写hex".into());
    }
    Ok((id, digest))
}
fn random_transfer_id() -> Result<String, String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .map_err(err)?
        .read_exact(&mut bytes)
        .map_err(err)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn encoded_page(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for block in bytes.chunks(3) {
        let a = block[0];
        let b = block.get(1).copied().unwrap_or(0);
        let c = block.get(2).copied().unwrap_or(0);
        out.push(DIGITS[(a >> 2) as usize] as char);
        out.push(DIGITS[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if block.len() > 1 {
            DIGITS[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if block.len() > 2 {
            DIGITS[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
fn append_read_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    let size = out
        .len()
        .checked_add(bytes.len())
        .filter(|n| *n <= READ_CACHE)
        .ok_or("QueryBudgetExceeded：完整读取逻辑文档超过4MiB")?;
    out.try_reserve_exact(size - out.len()).map_err(err)?;
    out.extend_from_slice(bytes);
    Ok(())
}
/// fields 的 raw_field 是唯一直接嵌入的、已核 canonical JSON 字节；其余按原规范核编码。
fn read_object(fields: &Value, raw_field: &str, raw: &[u8]) -> Result<Vec<u8>, String> {
    let map = fields
        .as_object()
        .ok_or("SchemaUnsupported：分页逻辑对象")?;
    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();
    let mut out = Vec::new();
    append_read_bytes(&mut out, b"{")?;
    for (index, key) in keys.iter().enumerate() {
        if index != 0 {
            append_read_bytes(&mut out, b",")?;
        }
        append_read_bytes(&mut out, canonical_json(&json!(key)).as_bytes())?;
        append_read_bytes(&mut out, b":")?;
        if key.as_str() == raw_field {
            append_read_bytes(&mut out, raw)?;
        } else {
            append_read_bytes(&mut out, canonical_json(&map[*key]).as_bytes())?;
        }
    }
    append_read_bytes(&mut out, b"}")?;
    Ok(out)
}
fn selected_image_meta(
    c: &Connection,
    config: &Config,
    query: &Value,
) -> Result<StoredImage, String> {
    if !query["cut"].is_null() && !query["as_known_ns"].is_null() {
        return Err("InvalidDomain：cut与AsKnown不能同时指定".into());
    }
    let selected = if !query["cut"].is_null() {
        let wanted = &query["cut"];
        exact(wanted, &["domain_id", "commit_seq", "root_hash"])?;
        if wanted["domain_id"] != config.domain {
            return Err("IdentityConflict：跨域cut".into());
        }
        nonnegative(required(wanted, "commit_seq")?, "commit_seq")?
    } else if !query["as_known_ns"].is_null() {
        let ns = nonnegative(required(query, "as_known_ns")?, "as_known_ns")?;
        c.query_row(
            "SELECT max(seq) FROM commits WHERE CAST(commit_ns AS INTEGER)<=?",
            [ns],
            |r| r.get::<_, Option<i64>>(0),
        )
        .map_err(err)?
        .ok_or("MissingDependency：AsKnown早于域创世")?
    } else {
        c.query_row("SELECT max(seq) FROM commits", [], |r| {
            r.get::<_, Option<i64>>(0)
        })
        .map_err(err)?
        .ok_or("MissingDependency：域无创世cut")?
    };
    let image = stored_image(c, selected, READ_CACHE)?;
    if !query["cut"].is_null() && query["cut"] != cut(config, image.seq, &image.root) {
        return Err("IdentityConflict：cut根不匹配".into());
    }
    Ok(image)
}
fn fixed_image_bytes(
    c: &Connection,
    image: &StoredImage,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, String> {
    check_operation_deadline(None, Some(deadline))?;
    let raw = stored_image_bytes(c, image, 0, image.bytes)?;
    let parsed = crate::session_protocol::strict_json(&raw)
        .map_err(|e| format!("StorageUnavailable：持久图像解析失败：{e}"))?;
    if canonical_json(&parsed).as_bytes() != raw.as_slice() {
        return Err("StorageUnavailable：持久图像不是原canonical字节".into());
    }
    check_operation_deadline(None, Some(deadline))?;
    Ok(raw)
}
fn fixed_read_document(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    runtime: &Value,
    query: &Value,
    deadline: std::time::Instant,
) -> Result<(Vec<u8>, Vec<StoredImage>, StoredImage, &'static str), String> {
    let tx = c.transaction().map_err(err)?;
    check_operation_deadline(None, Some(deadline))?;
    check_epoch(&tx, epoch)?;
    let operation = required(query, "op")?;
    let mut cuts = Vec::new();
    let mut changes = Vec::new();
    let next = match operation {
        "ReadView" => {
            exact(query, &["op", "cut", "as_known_ns"])?;
            let selected = selected_image_meta(&tx, config, query)?;
            cuts.push(selected.clone());
            selected
        }
        "Watch" => {
            exact(query, &["op", "after_cut"])?;
            let after = if query["after_cut"].is_null() {
                -1
            } else {
                let selected = selected_image_meta(
                    &tx,
                    config,
                    &json!({"cut":query["after_cut"],"as_known_ns":null}),
                )?;
                let seq = selected.seq;
                cuts.push(selected);
                seq
            };
            let next = selected_image_meta(&tx, config, &json!({"cut":null,"as_known_ns":null}))?;
            let count: i64 = tx
                .query_row(
                    "SELECT count(*) FROM commits WHERE seq>? AND seq<=?",
                    params![after, next.seq],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if count < 0 || count > READ_CHANGES as i64 {
                return Err("QueryBudgetExceeded：完整Watch超过32条".into());
            }
            let mut statement = tx
                .prepare("SELECT seq FROM commits WHERE seq>? AND seq<=? ORDER BY seq")
                .map_err(err)?;
            let rows = statement
                .query_map(params![after, next.seq], |r| r.get::<_, i64>(0))
                .map_err(err)?;
            let mut total = 0usize;
            for row in rows {
                let image = stored_image(&tx, row.map_err(err)?, READ_CACHE)?;
                total = total
                    .checked_add(image.bytes)
                    .filter(|n| *n <= READ_CACHE)
                    .ok_or("QueryBudgetExceeded：完整Watch图像超过4MiB")?;
                changes.push(image.clone());
                cuts.push(image);
            }
            if !cuts.iter().any(|image| image.seq == next.seq) {
                cuts.push(next.clone());
            }
            next
        }
        _ => return Err("SchemaUnsupported：分页只接受ReadView/Watch".into()),
    };
    // 每次 open 沿原审计核一次；所有 SQL 的检查共享本读截止。续页不重跑历史。
    let mut job = if config.revision()? == SCHEMA_V2 {
        Some(new_content_job(&tx, config)?)
    } else {
        None
    };
    audit_with_deadline(&tx, config, job.as_mut(), Some(deadline))?;
    let (document, kind) = if operation == "ReadView" {
        let raw = fixed_image_bytes(&tx, &next, deadline)?;
        let fields = json!({"kind":"EconomicView","domain_id":config.domain,"cut":cut(config,next.seq,&next.root),"image_hash":sha256_hex(&raw),"image":null,"commit_ns":next.commit_ns,"writer_epoch":epoch.to_string(),"runtime":runtime});
        (read_object(&fields, "image", &raw)?, "EconomicView")
    } else {
        let mut array = Vec::new();
        append_read_bytes(&mut array, b"[")?;
        for (index, image) in changes.iter().enumerate() {
            let raw = fixed_image_bytes(&tx, image, deadline)?;
            let fields = json!({"cut":cut(config,image.seq,&image.root),"image_hash":sha256_hex(&raw),"image":null,"commit_ns":image.commit_ns});
            let change = read_object(&fields, "image", &raw)?;
            if index != 0 {
                append_read_bytes(&mut array, b",")?;
            }
            append_read_bytes(&mut array, &change)?;
        }
        append_read_bytes(&mut array, b"]")?;
        let fields = json!({"kind":"EconomicChanges","domain_id":config.domain,"cut":cut(config,next.seq,&next.root),"changes":null,"writer_epoch":epoch.to_string()});
        (read_object(&fields, "changes", &array)?, "EconomicChanges")
    };
    check_operation_deadline(None, Some(deadline))?;
    tx.commit().map_err(err)?;
    Ok((document, cuts, next, kind))
}

fn checked_transfer_roots(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    entry: &ReadTransfer,
) -> Result<(), String> {
    check_operation_deadline(None, Some(entry.deadline))?;
    let tx = c.transaction().map_err(err)?;
    check_epoch(&tx, epoch)?;
    for (name, expected) in [
        ("domain_id", &config.domain),
        ("session_id", &config.session),
        ("session_generation", &config.generation),
        ("manifest_hash", &config.manifest_hash),
    ] {
        if meta(&tx, name)? != *expected {
            return Err("IdentityConflict：分页持久域身份改变".into());
        }
    }
    for image in &entry.cuts {
        if stored_image(&tx, image.seq, READ_CACHE)? != *image {
            return Err("StorageUnavailable：分页固定cut身份改变".into());
        }
        check_operation_deadline(None, Some(entry.deadline))?;
    }
    tx.commit().map_err(err)?;
    Ok(())
}
fn read_opened(entry: &ReadTransfer) -> Result<Value, String> {
    if entry.document.is_none() {
        return Err("SnapshotExpired：分页已关闭、失败或过期".into());
    }
    check_operation_deadline(None, Some(entry.deadline))?;
    Ok(
        json!({"schema_revision":READ_PAGES,"kind":"EconomicReadOpened","descriptor":crate::session_protocol::strict_json(entry.descriptor.as_bytes())?,"descriptor_sha256":entry.descriptor_hash}),
    )
}
fn read_response(e: &Value, payload: Value, config: &Config, epoch: i64) -> Result<Value, String> {
    let response = reply_envelope(
        e,
        payload,
        config.revision()?,
        &config.session,
        &config.generation,
        "economic-session/replies",
        &config.producer,
        &epoch.to_string(),
    );
    let configured = usize::try_from(positive(
        required(&config.manifest["resources"], "max_frame_bytes")?,
        "max_frame_bytes",
    )?)
    .map_err(err)?;
    let frame = canonical_json(&response);
    if frame
        .len()
        .checked_add(1)
        .is_none_or(|bytes| bytes > configured.min(READ_FRAME))
    {
        return Err("QueryBudgetExceeded：完整分页信封加LF超过物理帧帽".into());
    }
    Ok(response)
}
fn paged_read(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    runtime: &Value,
    e: &Value,
    ingress: std::time::Instant,
    state: &mut ReadTransfers,
) -> Result<Value, String> {
    config.authorized(e)?;
    let p = &e["payload"];
    if p["schema_revision"] != READ_PAGES {
        return Err("SchemaUnsupported：只读分页版本".into());
    }
    state.expire();
    match required(p, "op")? {
        "EconomicReadOpen" => {
            exact(
                p,
                &["schema_revision", "op", "query", "requested_remaining_ms"],
            )?;
            let deadline = read_deadline(p, ingress)?;
            let principal = read_principal(e);
            let open_key = hash(&json!([principal, e["message_id"]]));
            let request_hash = hash(e);
            let query = &p["query"];
            // 分页权限不隐含扩大原逻辑查询权限。
            let mut logical_request = e.clone();
            logical_request["payload"] = query.clone();
            config.authorized(&logical_request)?;
            if let Some(entry) = state
                .entries
                .values_mut()
                .find(|entry| entry.open_key == open_key)
            {
                if entry.request_hash != request_hash {
                    return Err("IdentityConflict：分页open消息身份内容改变".into());
                }
                entry.deadline = entry.deadline.min(deadline);
                checked_transfer_roots(c, config, epoch, entry)?;
                return read_response(e, read_opened(entry)?, config, epoch);
            }
            if state
                .entries
                .values()
                .filter(|entry| entry.document.is_some())
                .count()
                >= READ_TRANSFERS
            {
                return Err("QueryBudgetExceeded：活动分页逻辑项达到4项".into());
            }
            let id = random_transfer_id()?;
            if state.entries.contains_key(&id) {
                return Err("IdentityConflict：随机分页身份冲突".into());
            }
            let entry = ReadTransfer {
                open_key,
                request_hash,
                principal_hash: principal,
                descriptor: String::new(),
                // 失败后的终态也持有摘要；准入时预留这64字节，避免失败清理后越帽。
                descriptor_hash: "0".repeat(64),
                document: None,
                cuts: Vec::new(),
                deadline,
            };
            if state
                .retained_bytes()
                .checked_add(entry.retained_bytes(&id))
                .is_none_or(|n| n > READ_CACHE)
            {
                return Err("QueryBudgetExceeded：分页共享缓存元数据已满".into());
            }
            // 先签去重身份：任何审计/编码失败都不能用相同 open 消息重开时间窗。
            state.entries.insert(id.clone(), entry);
            let (document, cuts, next, kind) =
                fixed_read_document(c, config, epoch, runtime, query, deadline)?;
            let mut fixed_cuts = serde_json::Map::new();
            fixed_cuts.insert(config.domain.clone(), cut(config, next.seq, &next.root));
            let mut owner_epochs = serde_json::Map::new();
            owner_epochs.insert(config.domain.clone(), json!(epoch.to_string()));
            let descriptor = json!({"schema_revision":READ_PAGES,"transfer_id":id,
                "session_id":config.session,"session_generation":config.generation,"manifest_hash":config.manifest_hash,
                "producer_id":config.producer,"producer_epoch":epoch.to_string(),"query_sha256":hash(query),
                "document_kind":kind,"fixed_cuts":fixed_cuts,"owner_epochs":owner_epochs,
                "document_bytes":document.len().to_string(),"document_sha256":sha256_hex(&document),
                "chunk_bytes":READ_CHUNK.to_string(),"page_count":document.len().div_ceil(READ_CHUNK).to_string()});
            let descriptor_bytes = canonical_json(&descriptor);
            let descriptor_hash = sha256_hex(descriptor_bytes.as_bytes());
            let payload = json!({"schema_revision":READ_PAGES,"kind":"EconomicReadOpened","descriptor":descriptor,"descriptor_sha256":descriptor_hash});
            let response = read_response(e, payload, config, epoch)?;
            check_operation_deadline(None, Some(deadline))?;
            let entry = state.entries.get_mut(&id).unwrap();
            entry.descriptor = descriptor_bytes;
            entry.descriptor_hash = descriptor_hash;
            entry.document = Some(document);
            entry.cuts = cuts;
            if state.retained_bytes() > READ_CACHE {
                state.entries.get_mut(&id).unwrap().release_document();
                return Err("QueryBudgetExceeded：逻辑原文与分页共享元数据超过4MiB".into());
            }
            Ok(response)
        }
        "EconomicReadPage" => {
            exact(
                p,
                &[
                    "schema_revision",
                    "op",
                    "transfer_id",
                    "descriptor_sha256",
                    "offset",
                    "requested_remaining_ms",
                ],
            )?;
            let deadline = read_deadline(p, ingress)?;
            let (id, descriptor_hash) = page_identity(p)?;
            let entry = state
                .entries
                .get_mut(id)
                .ok_or("SnapshotExpired：未知分页身份")?;
            if entry.principal_hash != read_principal(e) || entry.descriptor_hash != descriptor_hash
            {
                return Err("IdentityConflict：分页来源或描述符不匹配".into());
            }
            entry.deadline = entry.deadline.min(deadline);
            checked_transfer_roots(c, config, epoch, entry)?;
            let document = entry
                .document
                .as_ref()
                .ok_or("SnapshotExpired：分页已关闭或过期")?;
            let offset =
                usize::try_from(nonnegative(required(p, "offset")?, "offset")?).map_err(err)?;
            if offset >= document.len() || offset % READ_CHUNK != 0 {
                return Err("InvalidDomain：页offset不是固定步长边界".into());
            }
            let end = offset.saturating_add(READ_CHUNK).min(document.len());
            let raw = &document[offset..end];
            let done = end == document.len();
            let payload = json!({"schema_revision":READ_PAGES,"kind":"EconomicReadPage","transfer_id":id,
                "descriptor_sha256":descriptor_hash,"offset":offset.to_string(),"raw_byte_count":raw.len().to_string(),
                "chunk_sha256":sha256_hex(raw),"data_base64":encoded_page(raw),
                "next_offset":if done { Value::Null } else { json!(end.to_string()) },"done":done});
            let response = read_response(e, payload, config, epoch)?;
            check_operation_deadline(None, Some(entry.deadline))?;
            Ok(response)
        }
        "EconomicReadClose" => {
            exact(
                p,
                &["schema_revision", "op", "transfer_id", "descriptor_sha256"],
            )?;
            let (id, descriptor_hash) = page_identity(p)?;
            if let Some(entry) = state.entries.get_mut(id) {
                if entry.principal_hash != read_principal(e)
                    || entry.descriptor_hash != descriptor_hash
                {
                    return Err("IdentityConflict：关闭来源或描述符不匹配".into());
                }
                entry.release_document();
            }
            read_response(
                e,
                json!({"schema_revision":READ_PAGES,"kind":"EconomicReadClosed","transfer_id":id,"descriptor_sha256":descriptor_hash}),
                config,
                epoch,
            )
        }
        _ => Err("SchemaUnsupported：未知只读分页操作".into()),
    }
}
fn handle_paged_read(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    runtime: &Value,
    e: &Value,
    deadline: std::time::Instant,
    state: &mut ReadTransfers,
) -> Value {
    match paged_read(c, config, epoch, runtime, e, deadline, state) {
        Ok(response) => response,
        Err(error) => {
            let code = if error.starts_with("SchemaUnsupported")
                && (error.contains("只读分页版本")
                    || error.contains("分页只接受")
                    || error.contains("未知只读分页操作"))
            {
                "SchemaUnsupported"
            } else if error.starts_with("IdentityConflict")
                || error.starts_with("StaleWriter")
                || error.contains("来源未获准")
            {
                "IdentityConflict"
            } else if error.starts_with("MissingDependency") {
                "MissingDependency"
            } else if error.starts_with("SnapshotExpired")
                || error.starts_with("Incomplete：经济操作最早绝对期限已过")
            {
                "TransferExpired"
            } else if error.starts_with("QueryBudgetExceeded") || error.starts_with("Incomplete") {
                "QueryBudgetExceeded"
            } else if error.starts_with("StorageUnavailable")
                && (error.contains("域损坏") || error.contains("图像") || error.contains("cut"))
            {
                "IntegrityError"
            } else if error.starts_with("StorageUnavailable") {
                "SourceUnavailable"
            } else {
                "InvalidQuery"
            };
            let transfer_id = page_identity(&e["payload"])
                .ok()
                .map(|(id, _)| json!(id))
                .unwrap_or(Value::Null);
            let payload = json!({"schema_revision":READ_PAGES,"kind":"EconomicReadError","code":code,"detail":error,"transfer_id":transfer_id});
            read_response(e, payload, config, epoch).unwrap_or_else(|_| {
                // 巨大公共身份使合法因果回复自身超帧时，不发送超帽成功或伪造身份。
                // 与 transport 的无合法帧错误同形；接收端必须拒绝它。
                json!({"kind":"Error","error":"QueryBudgetExceeded：分页错误信封也超过物理帧帽"})
            })
        }
    }
}

/// `/1` 与 `/2` 的信封校验各一份，派发只按 `Config::revision()`；没有第三档、没有宽松兜底。
fn validate_v1(v: Value) -> Result<Value, String> {
    validate_envelope(v, SCHEMA_V1)
}
fn validate_v2(v: Value) -> Result<Value, String> {
    validate_envelope(v, SCHEMA_V2)
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
pub(super) fn previous_message(c: &Connection, e: &Value) -> Result<Option<Value>, String> {
    previous_message_metered(c, e, &mut ReadBudget::new(None, None))
}
pub(super) fn previous_message_metered(
    c: &Connection,
    e: &Value,
    budget: &mut ReadBudget<'_>,
) -> Result<Option<Value>, String> {
    let r: Option<(String, String, String)> = budget.optional(
        c,
        "SELECT payload_hash,receipt_key,request_json FROM messages WHERE k=?",
        [message_key(e)],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if let Some((h, k, original)) = r {
        if e["payload_hash"] != h || canonical_json(e) != original {
            return Err("IdentityConflict：原消息内容改变".into());
        }
        return metered_row(c, "receipts", &k, budget)?
            .map(Some)
            .ok_or_else(|| "StorageUnavailable：域损坏：消息缺回执".into());
    }
    Ok(None)
}
pub(super) fn bind_message(c: &Connection, e: &Value, k: &str) -> Result<(), String> {
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
pub(super) fn clocks(
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
    let (doc, event) = decode(raw, pointer, config.book_rules()?)?;
    if event["source"]["source_namespace"] != e["source_namespace"]
        || event["source"]["source_epoch"] != e["source_epoch"]
    {
        return Err("InvalidDomain：来源未获准：原件来源不等于获准连接来源".into());
    }
    let netted = super::tb03c_allocation::is_projected(&event);
    super::tb03c_allocation::check_capture(config, &event)?;
    if config.book_rules()?.is_tb03c() {
        super::tb03c_transfer::check_capture(config, &event)?;
    }
    let domain = super::tb03c_allocation::target_domain(&event)?;
    let id = if netted { "" } else { target(&event)? };
    // #1406 / C06.4：tb03c-book/1 域里目标重不在 manifest 的原件照记（Pending 封账户范围），旧域照旧拒收。
    let unrouted = json!({});
    let route = match config.route(id) {
        Ok(r) => r,
        Err(_) if config.book_rules()?.is_tb03c() => &unrouted,
        Err(e) => return Err(e),
    };
    if let Some(b) = event
        .get("book_identity")
        .filter(|_| !route.is_null() && route != &unrouted)
    {
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
        json!({"fact_key":fact_key(&event)?,"fact_hash":fact_hash,"raw_sha256":raw_hash,"raw_utf8":raw,"json_pointer":pointer,"event":event,"token":required(p,"token")?,"target_domain":domain,"source_event_time":p["source_event_time"],"source_received_time":event["source"]["received_time"],"source_producer":{"producer_id":e["producer_id"],"producer_epoch":e["producer_epoch"]},"origin_message":{"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]},"origin_request_hash":hash(e)}), // DevSkim: ignore DS197836 内容寻址摘要，非对时间值取哈希
    )
}
fn capture(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
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
    check_operation_deadline(None, ingress_response_deadline)?;
    let token = required(&record, "token")?;
    let token_table = if config.book_rules()?.is_tb03c() {
        "facts"
    } else {
        "pending"
    };
    if rows(&tx, token_table)?.iter().any(|r| r["token"] == token) {
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
    let pending = super::tb03c_e::pending_entry(config, &image, &record, seq)?;
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
    check_operation_deadline(None, ingress_response_deadline)?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}

fn resolve_evidence(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    use super::tb03c_e;
    if !config.book_rules()?.is_tb03c() {
        return Err("SchemaUnsupported：旧书账域不支持 ResolveEvidence".into());
    }
    let p = &e["payload"];
    let id = required(p, "evidence_id")?;
    let key = tb03c_e::receipt_key(id);
    if p["frontier"]["source_namespace"] != e["source_namespace"]
        || p["frontier"]["source_epoch"] != e["source_epoch"]
    {
        return Err("InvalidDomain：证据前沿不属于获准连接来源".into());
    }
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    if let Some(r) = previous_message(&tx, e)? {
        return Ok(r);
    }
    let (seq, prev, last_ns, image) = latest(&tx)?;
    if let Some(r) = row(&tx, "receipts", &key)? {
        if tb03c_e::event_for(&image, id)?["evidence"] != tb03c_e::evidence_body(p) {
            return Err("IdentityConflict：Resolve evidence_id 已绑定其他内容".into());
        }
        bind_message(&tx, e, &key)?;
        tx.commit().map_err(err)?;
        return Ok(r);
    }
    let (mut next, covered) = tb03c_e::resolve_image(config, &image, p)?;
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
    let (_, _, ns) = clocks(config, e, &floor, epoch)?;
    let event = tb03c_e::resolve_history(e, &covered, seq, &ns);
    next["history"].as_array_mut().unwrap().push(event.clone());
    sync_pending(&tx, &next)?;
    let cut = append(&tx, config, seq, &prev, &ns, &next)?;
    let receipt = tb03c_e::resolve_receipt(config, &event, cut);
    put(&tx, "receipts", &key, &receipt)?;
    bind_message(&tx, e, &key)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    check_operation_deadline(None, ingress_response_deadline)?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
fn authoritative_fact(config: &Config, key: &str, expected_epoch: i64) -> Result<Value, String> {
    authoritative_fact_inner(config, key, expected_epoch, None, None)
}
fn authoritative_fact_inner(
    config: &Config,
    key: &str,
    expected_epoch: i64,
    mut import: Option<(&mut super::content_store::Stage<'_>, &mut Job)>,
    deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    if let Some((_, job)) = import.as_mut() {
        job.charge_requests(1).map_err(|e| e.to_string())?;
    }
    let mut ec = config.clone();
    ec.domain = "E".into();
    ec.producer = format!("E:{}", ec.session);
    ec.db = PathBuf::from(required(&ec.manifest["e"], "db")?);
    ec.socket = PathBuf::from(required(&ec.manifest["e"], "socket")?);
    let (mut c, _) = open(&ec.db, true)?;
    let tx = c.transaction().map_err(err)?;
    check_epoch(&tx, expected_epoch)?;
    if let Some((_, job)) = import.as_mut() {
        audit_with_deadline(&tx, &ec, Some(&mut **job), deadline)?;
    } else {
        audit(&tx, &ec)?;
    }
    if config.book_rules()?.is_tb03c() {
        super::tb03c_e::check_wait(&tx, key)?;
    }
    let mut f = row(&tx, "facts", key)?.ok_or("MissingDependency：固定E权威不存在该原事实")?;
    let r = row(&tx, "receipts", key)?.ok_or("StorageUnavailable：域损坏：固定E缺原回执")?;
    verify_record(&f, config.book_rules()?)?;
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
    super::tb03c_allocation::attach_resolution(&tx, config, &mut f)?;
    verify_e_copy(&f, config)?;
    if let Some((stage, job)) = import {
        check_operation_deadline(Some(&mut *job), deadline)?;
        job.charge_wire_bytes(u64::try_from(canonical_json(&f).len()).map_err(err)?)
            .map_err(|e| e.to_string())?;
        let refs = super::content_store::image_refs(&json!({"recorded":[f.clone()],"history":[]}))
            .map_err(|e| e.to_string())?;
        super::content_store::import_peer_refs_with_deadline(&tx, &refs, stage, job, deadline)
            .map_err(|e| format!("MissingDependency：E 历史引用导入失败：{e}"))?;
    }
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
    apply_allocation_with_ingress(c, config, epoch, e_epoch, e, None)
}
fn apply_allocation_with_ingress(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e_epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    let mut audited = false;
    match apply_allocation_inner(c, config, epoch, e_epoch, e, &mut audited, ingress_response_deadline) {
        Ok(receipt) => Ok(receipt),
        Err(error) if config.revision()? == SCHEMA_V2 && audited => {
            match super::content_store::discard_unpublished(c, &config.db) {
                Ok((removed, before, after)) => Err(format!("{error}; staged_cleanup rows={removed} allocated_before={before} allocated_after={after}")),
                Err(cleanup) => Err(format!("{error}; staged_cleanup_pending_recover={cleanup}")),
            }
        }
        Err(error) => Err(error),
    }
}
fn apply_allocation_inner(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e_epoch: i64,
    e: &Value,
    audited: &mut bool,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    let p = &e["payload"];
    exact(
        p,
        &["op", "fact_key", "allocation_id", "expected_fact_hash"],
    )?;
    let key = required(p, "fact_key")?;
    let allocation = required(p, "allocation_id")?;
    required(p, "expected_fact_hash")?;
    let mut import_job = if config.revision()? == SCHEMA_V2 {
        Some(new_content_job(c, config)?)
    } else {
        None
    };
    let deadline = if let Some(job) = import_job.as_mut() {
        Some(operation_deadline(config, job, ingress_response_deadline)?)
    } else {
        None
    };
    // #1374：已提交身份先从本域查回，不让当前E故障抹掉原成功。
    {
        let tx = immediate(c)?;
        check_epoch(&tx, epoch)?;
        audit_with_job(&tx, config, import_job.as_mut())?;
        *audited = true;
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
    check_operation_deadline(import_job.as_mut(), deadline)?;
    // 固定 E 只读审计发生在 B 写事务之外；无跨域写锁、无 ATTACH。
    // 新域在同一只读捕获内把该 E 提交图像全部历史引用导入本 B owner。
    let mut record = if let Some(job) = import_job.as_mut() {
        job.begin_first_read().map_err(|e| e.to_string())?;
        let token = format!(
            "stg-{}",
            hash(&json!([config.domain, epoch.to_string(), allocation]))
        );
        let mut stage =
            super::content_store::Stage::new(c, &config.db, job.snapshot().limits, token)
                .map_err(|e| format!("Incomplete：B 内容暂存不可用：{e}"))?;
        authoritative_fact_inner(config, key, e_epoch, Some((&mut stage, job)), deadline)?
    } else {
        authoritative_fact(config, key, e_epoch)?
    };
    if record["fact_hash"] != p["expected_fact_hash"]
        || super::tb03c_allocation::application_id(&record)? != allocation
        || super::tb03c_allocation::application_domain(&record)? != config.domain
    {
        return Err("IdentityConflict：实读E事实/金额/归属与请求不匹配".into());
    }
    // #1406 C5：固定源 B 只读证明仍在目标 B 写事务之外；不跨 owner 持写锁。
    if config.book_rules()?.is_tb03c() && super::tb03c_transfer::is_in(&record["event"]) {
        if let Some(job) = import_job.as_mut() {
            let token = format!(
                "stg-{}",
                hash(&json!([config.domain, epoch.to_string(), allocation]))
            );
            let mut stage =
                super::content_store::Stage::new(c, &config.db, job.snapshot().limits, token)
                    .map_err(|e| format!("Incomplete：源证明内容暂存不可用：{e}"))?;
            super::tb03c_transfer::attach_source_proof(
                config,
                &mut record,
                Some((&mut stage, job)),
                deadline.or(ingress_response_deadline),
            )?;
        } else {
            super::tb03c_transfer::attach_source_proof(
                config,
                &mut record,
                None,
                deadline.or(ingress_response_deadline),
            )?;
        }
    }
    let final_budget = if let Some(job) = import_job.as_mut() {
        Some(
            super::content_store::reserve_final(c, &config.db, job)
                .map_err(|e| format!("Incomplete：ApplyAllocation 物理预留失败：{e}"))?,
        )
    } else {
        None
    };
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit_with_deadline(&tx, config, import_job.as_mut(), deadline)?;
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
    let (next, unmatched) = super::book::apply_in(&config.manifest, &image, &record, seq, &ns)?;
    // H1：在内容发布或任何目标行写入之前核完整包装后的读合同。
    super::store::readable_image(&next)?;
    if let Some(job) = import_job.as_mut() {
        let refs = super::content_store::image_refs(&next)
            .map_err(|e| format!("MissingDependency：ApplyAllocation 引用路径非法：{e}"))?;
        super::content_store::publish(&tx, &refs, job)
            .map_err(|e| format!("MissingDependency：B 缺完整 E 历史闭包：{e}"))?;
        job.pre_io_check()
            .map_err(|e| format!("Incomplete：ApplyAllocation 提交前期限：{e}"))?;
    }
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
    check_operation_deadline(import_job.as_mut(), deadline)?;
    tx.commit().map_err(err)?;
    if let Some((grant, before, ceiling)) = final_budget {
        super::content_store::reconcile_final(
            c,
            &config.db,
            import_job
                .as_mut()
                .ok_or("DeliveryUnknown：Apply 已提交但 job 缺失")?,
            &grant,
            before,
            ceiling,
        )
        .map_err(|e| format!("DeliveryUnknown：Apply 已提交，物理对账待核：{e}"))?;
    }
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
fn bind_voice(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
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
            let original = super::voice::original_request(
                &tx,
                event,
                config,
                &mut ReadBudget::new(None, ingress_response_deadline),
            )?;
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
    check_operation_deadline(None, ingress_response_deadline)?;
    // 读取固定证据在 B 写事务之外，绝不持生命周期控制锁等待外部依据。
    let verified = super::evidence::load(
        config,
        required(p, "evidence_id")?,
        required(&p["carrier_ref"], "authority_id")?,
    )?;
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit(&tx, config)?;
    check_operation_deadline(None, ingress_response_deadline)?;
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
    check_operation_deadline(None, ingress_response_deadline)?;
    tx.commit().map_err(err)?;
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(receipt)
}
pub(super) fn select_cut(
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
        "QueryDecision" | "QueryGrant" => super::finance::query(&tx, config, p)?,
        "QueryFact" => {
            exact(p, &["op", "fact_key"])?;
            let k = required(p, "fact_key")?;
            let f = row(&tx, "facts", k)?.ok_or("MissingDependency：原事实不存在")?;
            let receipt_key = if config.domain == "E" {
                k
            } else {
                super::tb03c_allocation::application_id(&f)?
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
    handle_with_deadline(c, config, epoch, e_epoch, runtime, e, None)
}
fn handle_with_deadline(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e_epoch: i64,
    runtime: &Value,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Value {
    let result = (|| {
        config.authorized(e)?;
        if config.revision()? == SCHEMA_V2 {
            let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
                &config.manifest["finance"]["structure"]["reader_resources"],
            )?;
            super::content_store::configure(c, &config.db, resources.job_limits)
                .map_err(|error| format!("Incomplete：新域物理存储门未成立：{error}"))?;
        }
        let op = required(&e["payload"], "op")?;
        match op {
            "ChongAdvance" if config.domain == "E" => {
                // Once admitted, the multi-owner responsibility workflow may
                // continue after the reply times out. An already expired
                // queued request must not start new cross-domain work.
                check_operation_deadline(None, ingress_response_deadline)?;
                super::finance::advance(c, config, epoch, e)
            }
            "PrepareDecision" | "CommitDecision" | "AbortDecision"
                if config.domain.starts_with("B:") =>
            {
                super::finance::write_with_ingress(c, config, epoch, e, ingress_response_deadline)
            }
            "ImportResponsibility" | "Reserve" | "CloseApplicants" if config.domain == "E" => {
                super::finance::write_with_ingress(c, config, epoch, e, ingress_response_deadline)
            }
            "CaptureExternal" if config.domain == "E" => {
                capture(c, config, epoch, e, ingress_response_deadline)
            }
            "ResolveEvidence" if config.domain == "E" => {
                resolve_evidence(c, config, epoch, e, ingress_response_deadline)
            }
            "ApplyAllocation" if config.domain.starts_with("B:") => apply_allocation_with_ingress(
                c,
                config,
                epoch,
                e_epoch,
                e,
                ingress_response_deadline,
            ),
            "BindVoice" if config.domain.starts_with("B:") => {
                bind_voice(c, config, epoch, e, ingress_response_deadline)
            }
            "ReadView" | "Watch" | "QueryFact" | "QueryAllocation" | "QueryVoice"
            | "QueryDecision" | "QueryGrant" => read(c, config, epoch, runtime, &e["payload"]),
            _ => Err("SchemaUnsupported：操作不属于本域".into()),
        }
    })();
    let payload = match result {
        Ok(v) => v,
        Err(error) => json!({"kind":"Error","domain_id":config.domain,"error":error}),
    };
    match config.revision() {
        Ok(revision) => reply_envelope(
            e,
            payload,
            revision,
            &config.session,
            &config.generation,
            "economic-session/replies",
            &config.producer,
            &epoch.to_string(),
        ),
        Err(error) => json!({"kind":"Error","domain_id":config.domain,"error":error}),
    }
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
            // 信封校验按本域登记的版本派发；未知版本在 Config::load 就已拒绝。
            let validator: EnvelopeValidator = if config.revision()? == SCHEMA_V2 {
                validate_v2
            } else {
                validate_v1
            };
            let mut reads = ReadTransfers::default();
            let revision = config.revision()?;
            serve_unix_with_deadline(&config.socket, bounds, validator, |e, deadline| {
                if matches!(
                    e["payload"]["op"].as_str(),
                    Some("EconomicReadOpen" | "EconomicReadPage" | "EconomicReadClose")
                ) {
                    handle_paged_read(&mut c, &config, epoch, &runtime, e, deadline, &mut reads)
                } else if revision == SCHEMA_V2 {
                    handle_with_deadline(
                        &mut c,
                        &config,
                        epoch,
                        e_epoch,
                        &runtime,
                        e,
                        Some(deadline),
                    )
                } else {
                    // /1 金融写入口保持原有语义；只读分页使用新 transport 截止。
                    handle(&mut c, &config, epoch, e_epoch, &runtime, e)
                }
            })?;
        }
        _ => return Err("InvalidDomain：未知子命令".into()),
    }
    Ok(())
}

#[cfg(test)]
mod read_page_tests {
    use super::*;
    use crate::economic_session::tb03c_book_tests::Space;
    use std::time::Instant;

    fn fixture(tag: &str) -> (Space, Config, Connection) {
        let space = Space::new(tag);
        let manifest = json!({"schema_revision":"economic-manifest/1","session_id":"pages","session_generation":"1","book_revision":"tb03c-book/1",
            "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
            "chongs":[{"chong_id":"one","symbol":"TEST","operation_level":"1","fixed_q":"10","db":space.0.join("b.db"),"socket":space.0.join("b.sock")}],
            "authorized_sources":[{"source_namespace":"pages-test","source_epoch":"1","producer_id":"reader","producer_epoch":"1","operations":["ReadView","Watch","EconomicReadOpen","EconomicReadPage","EconomicReadClose"]}],
            "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"30000","queue_capacity":"2","max_connections":"2"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000"],"messages":{}}}});
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest)).unwrap();
        let config = Config::load(&path, &space.0.join("e.db"), None).unwrap();
        init(&config).unwrap();
        let connection = open(&config.db, false).unwrap().0;
        (space, config, connection)
    }
    fn request(payload: Value, id: &str) -> Value {
        json!({"schema_revision":SCHEMA_V1,"session_id":"pages","session_generation":"1",
            "source_namespace":"pages-test","source_epoch":"1","producer_id":"reader","producer_epoch":"1",
            "message_id":id,"causal_refs":[],"payload_hash":hash(&payload),"payload":payload})
    }
    fn open_request(query: Value, id: &str) -> Value {
        request(
            json!({"schema_revision":READ_PAGES,"op":"EconomicReadOpen","query":query,"requested_remaining_ms":"30000"}),
            id,
        )
    }
    fn view_request(id: &str) -> Value {
        open_request(json!({"op":"ReadView","cut":null,"as_known_ns":null}), id)
    }
    fn page_request(opened: &Value, offset: usize, id: &str) -> Value {
        request(
            json!({"schema_revision":READ_PAGES,"op":"EconomicReadPage","transfer_id":opened["descriptor"]["transfer_id"],"descriptor_sha256":opened["descriptor_sha256"],"offset":offset.to_string(),"requested_remaining_ms":"30000"}),
            id,
        )
    }
    fn close_request(opened: &Value) -> Value {
        request(
            json!({"schema_revision":READ_PAGES,"op":"EconomicReadClose","transfer_id":opened["descriptor"]["transfer_id"],"descriptor_sha256":opened["descriptor_sha256"]}),
            "close",
        )
    }
    fn send(
        c: &mut Connection,
        config: &Config,
        runtime: &Value,
        request: &Value,
        state: &mut ReadTransfers,
    ) -> Value {
        handle_paged_read(
            c,
            config,
            1,
            runtime,
            request,
            Instant::now() + Duration::from_secs(30),
            state,
        )["payload"]
            .clone()
    }
    fn decode_base64_for_test(encoded: &str) -> Vec<u8> {
        // 独立的逐位解码，只用于断言编码输出；不参与生产组装。
        let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bits = 0u32;
        let mut held = 0;
        let mut out = Vec::new();
        for b in encoded.bytes().take_while(|b| *b != b'=') {
            bits = (bits << 6) | alphabet.bytes().position(|c| c == b).unwrap() as u32;
            held += 6;
            if held >= 8 {
                held -= 8;
                out.push((bits >> held) as u8);
            }
        }
        out
    }

    #[test]
    fn page_encoding_matches_independent_rfc4648_vectors() {
        for (raw, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("汉", "5rGJ"),
            ("{\"ok\":true}", "eyJvayI6dHJ1ZX0="),
        ] {
            assert_eq!(encoded_page(raw.as_bytes()), encoded);
            assert_eq!(decode_base64_for_test(encoded), raw.as_bytes());
        }
    }

    #[test]
    fn blob_reader_counts_and_slices_utf8_bytes_and_rechecks_cut_identity() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE commits(seq INTEGER,root TEXT,prev_root TEXT,commit_ns TEXT,image TEXT)",
        )
        .unwrap();
        let raw = "{\"x\":\"汉😀\"}";
        c.execute("INSERT INTO commits VALUES(0,'root','','1',?)", [raw])
            .unwrap();
        let image = stored_image(&c, 0, 100).unwrap();
        assert_eq!(image.bytes, raw.len());
        assert_eq!(
            stored_image_bytes(&c, &image, 7, 2).unwrap(),
            &raw.as_bytes()[7..9]
        );
        assert!(stored_image(&c, 0, raw.len() - 1).is_err());
        assert!(stored_image_bytes(&c, &image, raw.len(), 1).is_err());
        c.execute("UPDATE commits SET root='changed'", []).unwrap();
        assert!(stored_image_bytes(&c, &image, 0, 1).is_err());
    }

    #[test]
    fn open_view_and_watch_preserve_original_canonical_payload() {
        let (_space, config, mut c) = fixture("c6a-page-original");
        let runtime = json!({"diagnostic":"UTF-8/汉"});
        let (seq, root, _, _) = latest(&c).unwrap();
        for (i, query) in [
            json!({"op":"ReadView","cut":null,"as_known_ns":null}),
            json!({"op":"ReadView","cut":cut(&config,seq,&root),"as_known_ns":null}),
            json!({"op":"ReadView","cut":null,"as_known_ns":"1"}),
            json!({"op":"Watch","after_cut":null}),
            json!({"op":"Watch","after_cut":cut(&config,seq,&root)}),
        ]
        .into_iter()
        .enumerate()
        {
            let expected = read(&mut c, &config, 1, &runtime, &query).unwrap();
            let mut state = ReadTransfers::default();
            let opened = send(
                &mut c,
                &config,
                &runtime,
                &open_request(query.clone(), &format!("open-{i}")),
                &mut state,
            );
            assert_eq!(opened["kind"], "EconomicReadOpened", "{opened}");
            let descriptor = &opened["descriptor"];
            let id = descriptor["transfer_id"].as_str().unwrap();
            let document = state.entries[id].document.as_ref().unwrap();
            assert_eq!(document, canonical_json(&expected).as_bytes());
            assert_eq!(descriptor["query_sha256"], hash(&query));
            assert_eq!(descriptor["document_sha256"], sha256_hex(document));
            assert_eq!(descriptor["fixed_cuts"]["E"], expected["cut"]);
            assert_eq!(descriptor["owner_epochs"]["E"], "1");
            assert!(state.retained_bytes() <= READ_CACHE);
        }
    }

    #[test]
    fn oversized_logical_view_crosses_utf8_pages_with_bounded_full_frames() {
        let (_space, config, mut c) = fixture("c6a-page-large");
        // 只扩大诊断字段，验证字节协议；不冒充完整金融41步正控。
        let runtime = json!({"padding":"汉😀\\\"".repeat(150_000)});
        let mut state = ReadTransfers::default();
        let opened = send(
            &mut c,
            &config,
            &runtime,
            &view_request("large"),
            &mut state,
        );
        assert_eq!(opened["kind"], "EconomicReadOpened", "{opened}");
        let id = opened["descriptor"]["transfer_id"].as_str().unwrap();
        let original = state.entries[id].document.as_ref().unwrap().clone();
        assert!(original.len() > READ_FRAME);
        let mut rebuilt = Vec::new();
        for offset in (0..original.len()).step_by(READ_CHUNK) {
            let req = page_request(&opened, offset, &format!("page-{offset}"));
            let response = handle_paged_read(
                &mut c,
                &config,
                1,
                &runtime,
                &req,
                Instant::now() + Duration::from_secs(30),
                &mut state,
            );
            assert!(canonical_json(&response).len() + 1 <= READ_FRAME);
            let page = &response["payload"];
            assert_eq!(page["kind"], "EconomicReadPage", "{page}");
            let raw = decode_base64_for_test(page["data_base64"].as_str().unwrap());
            assert_eq!(page["chunk_sha256"], sha256_hex(&raw));
            assert_eq!(page["offset"], offset.to_string());
            assert_eq!(page["raw_byte_count"], raw.len().to_string());
            rebuilt.extend_from_slice(&raw);
            assert_eq!(page["done"], rebuilt.len() == original.len());
            assert_eq!(
                page["next_offset"],
                if rebuilt.len() == original.len() {
                    Value::Null
                } else {
                    json!(rebuilt.len().to_string())
                }
            );
        }
        assert_eq!(rebuilt, original);
        let decoded = crate::session_protocol::strict_json(&rebuilt).unwrap();
        assert_eq!(decoded["runtime"], runtime);
    }

    #[test]
    fn open_and_page_retransmission_do_not_renew_deadline() {
        let (_space, config, mut c) = fixture("c6a-page-retry");
        let mut state = ReadTransfers::default();
        let req = view_request("same-open");
        let opened = send(&mut c, &config, &json!({}), &req, &mut state);
        let id = opened["descriptor"]["transfer_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deadline = state.entries[&id].deadline;
        assert_eq!(send(&mut c, &config, &json!({}), &req, &mut state), opened);
        assert_eq!(state.entries[&id].deadline, deadline);
        let page = page_request(&opened, 0, "same-page");
        let first = send(&mut c, &config, &json!({}), &page, &mut state);
        assert_eq!(send(&mut c, &config, &json!({}), &page, &mut state), first);
        assert_eq!(state.entries[&id].deadline, deadline);
        let changed = open_request(json!({"op":"Watch","after_cut":null}), "same-open");
        assert_eq!(
            send(&mut c, &config, &json!({}), &changed, &mut state)["code"],
            "IdentityConflict"
        );
        state.entries.get_mut(&id).unwrap().deadline = Instant::now();
        assert_eq!(
            send(&mut c, &config, &json!({}), &page, &mut state)["code"],
            "TransferExpired"
        );
        assert_eq!(
            send(&mut c, &config, &json!({}), &req, &mut state)["code"],
            "TransferExpired"
        );
        assert!(state.entries[&id].document.is_none());
        assert_eq!(state.entries.len(), 1);
    }

    #[test]
    fn close_releases_document_keeps_tombstone_and_is_idempotent() {
        let (_space, config, mut c) = fixture("c6a-page-close");
        let mut state = ReadTransfers::default();
        let req = view_request("close-open");
        let opened = send(&mut c, &config, &json!({}), &req, &mut state);
        let before = state.retained_bytes();
        let close = close_request(&opened);
        let closed = send(&mut c, &config, &json!({}), &close, &mut state);
        assert_eq!(closed["kind"], "EconomicReadClosed");
        assert!(state.retained_bytes() < before);
        assert!(state.retained_bytes() > 0);
        assert_eq!(
            send(&mut c, &config, &json!({}), &close, &mut state),
            closed
        );
        assert_eq!(
            send(&mut c, &config, &json!({}), &req, &mut state)["code"],
            "TransferExpired"
        );
    }

    #[test]
    fn malformed_offset_wrong_identity_epoch_and_changed_cut_fail_closed() {
        let (_space, config, mut c) = fixture("c6a-page-negative");
        let mut state = ReadTransfers::default();
        let opened = send(
            &mut c,
            &config,
            &json!({}),
            &view_request("negative"),
            &mut state,
        );
        let page = page_request(&opened, 1, "unaligned");
        assert_eq!(
            send(&mut c, &config, &json!({}), &page, &mut state)["code"],
            "InvalidQuery"
        );
        let mut wrong = page_request(&opened, 0, "wrong-descriptor");
        wrong["payload"]["descriptor_sha256"] = json!("f".repeat(64));
        assert_eq!(
            send(&mut c, &config, &json!({}), &wrong, &mut state)["code"],
            "IdentityConflict"
        );
        let mut wrong = page_request(&opened, 0, "wrong-source");
        wrong["source_epoch"] = json!("2");
        assert_eq!(
            send(&mut c, &config, &json!({}), &wrong, &mut state)["code"],
            "IdentityConflict"
        );
        set_meta(&c, "writer_epoch", "2").unwrap();
        let page = page_request(&opened, 0, "changed-epoch");
        assert_eq!(
            send(&mut c, &config, &json!({}), &page, &mut state)["code"],
            "IdentityConflict"
        );
        set_meta(&c, "writer_epoch", "1").unwrap();
        c.execute("UPDATE commits SET root='corrupt'", []).unwrap();
        assert_eq!(
            send(&mut c, &config, &json!({}), &page, &mut state)["code"],
            "IntegrityError"
        );
    }

    #[test]
    fn cache_pins_and_tombstone_metadata_share_the_original_budget() {
        let (_space, config, mut c) = fixture("c6a-page-cache");
        let mut state = ReadTransfers::default();
        let mut first = Value::Null;
        for n in 0..READ_TRANSFERS {
            let opened = send(
                &mut c,
                &config,
                &json!({}),
                &view_request(&format!("pin-{n}")),
                &mut state,
            );
            assert_eq!(opened["kind"], "EconomicReadOpened", "{opened}");
            if n == 0 {
                first = opened;
            }
        }
        assert_eq!(
            send(
                &mut c,
                &config,
                &json!({}),
                &view_request("pin-fifth"),
                &mut state
            )["code"],
            "QueryBudgetExceeded"
        );
        send(
            &mut c,
            &config,
            &json!({}),
            &close_request(&first),
            &mut state,
        );
        assert_eq!(
            send(
                &mut c,
                &config,
                &json!({}),
                &view_request("pin-new"),
                &mut state
            )["kind"],
            "EconomicReadOpened"
        );
        assert_eq!(state.entries.len(), READ_TRANSFERS + 1);
        assert!(state.retained_bytes() <= READ_CACHE);
        let mut too_big = Vec::new();
        assert!(append_read_bytes(&mut too_big, &vec![b'x'; READ_CACHE + 1]).is_err());
        assert!(too_big.is_empty());
    }

    #[test]
    fn page_acl_does_not_grant_unlisted_logical_watch() {
        let (_space, mut config, mut c) = fixture("c6a-page-acl");
        config.manifest["authorized_sources"][0]["operations"]
            .as_array_mut()
            .unwrap()
            .retain(|op| op != "Watch");
        let mut state = ReadTransfers::default();
        let error = send(
            &mut c,
            &config,
            &json!({}),
            &open_request(json!({"op":"Watch","after_cut":null}), "forbidden"),
            &mut state,
        );
        assert_eq!(error["kind"], "EconomicReadError");
        assert_eq!(error["code"], "IdentityConflict");
        assert!(state.entries.is_empty());
    }
}
