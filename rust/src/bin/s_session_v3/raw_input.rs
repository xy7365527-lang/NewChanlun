//! #1394：S2/S3 共用同一原始事件接纳规则。
//!
//! 本模块只操作调用方已经开启的事务；域、写者、profile、完整旧状态与消息/clock
//! 必须先由对应协议入口核验。它不提交事务，不绕过各域的控制记录，也不重做另一套判据。
use super::*;
use rusqlite::Transaction;

/// S2/S3 共用冻结输入 profile 与原始 schema 的绑定门；接纳任何原始事件前调用。
pub(super) fn validate_profile_binding(
    input: &RawInputFile,
    profile_id: &str,
) -> Result<(), String> {
    if (input.schema_revision == tb02a::RAW_SCHEMA) != (profile_id == tb02a::PROFILE) {
        return Err("SchemaUnsupported：绑定profile/schema不一致".into());
    }
    if input.profile != profile_id {
        return Err(format!(
            "InvalidDomain：输入 profile=`{}` 与 --profile 的 profile_id=`{profile_id}` 不一致",
            input.profile
        ));
    }
    Ok(())
}

pub(super) fn accept_events(
    tx: &Transaction<'_>,
    input: &RawInputFile,
    volume_unit: Option<&str>,
) -> Result<Vec<Value>, String> {
    let mut results = Vec::new();
    let mut qrev = tx
        .prepare("SELECT receipt_id, payload_hash, source_coord, seq FROM raw_events WHERE identity_key=?1 AND revision=?2")
        .map_err(|e| format!("prepare 查询失败：{e}"))?;
    let mut qmax = tx
        .prepare("SELECT COALESCE(MAX(revision), 0) FROM raw_events WHERE identity_key=?1")
        .map_err(|e| format!("prepare 查询失败：{e}"))?;
    let mut insert = tx
        .prepare(
            "INSERT INTO raw_events(identity_key, revision, input_revision, payload_hash, receipt_id, seq,
               source_namespace, source_epoch, instrument, event_id, received_at,
               raw_text, price, ts, volume, source_coord, supersedes_revision)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        )
        .map_err(|e| format!("prepare insert 失败：{e}"))?;

    let mut next_seq: i64 = tx
        .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
            r.get(0)
        })
        .map_err(|e| format!("读 max seq 失败：{e}"))?;

    for e in &input.events {
        parse_source_coord(&e.seq).map_err(|e| format!("InvalidDomain：{e}"))?;
        parse_canonical_i64(&e.price, "price")?;
        parse_canonical_i64(&e.timestamp, "timestamp")?;
        parse_canonical_i64(&e.volume, "volume")?;
        let rev = parse_canonical_i64(&e.revision, "revision")?;
        if let Some(vu) = &volume_unit {
            if e.volume != *vu {
                return Err(format!(
                    "InvalidDomain：profile volume_unit=`{vu}`，事件 `{}` volume=`{}` 不在单位成交量域（本片不擅自改成交量类型）",
                    e.event_id, e.volume
                ));
            }
        }
        let ikey = identity_key(
            &input.source_namespace,
            &input.source_epoch,
            &input.instrument,
            &e.event_id,
        );
        // 同一计算序的坐标只能归一个业务身份；该身份的历史 revision 可共享坐标。
        let occupied: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM raw_events WHERE source_coord=?1 AND identity_key<>?2)",
                params![e.seq, ikey],
                |row| row.get(0),
            )
            .map_err(|e| format!("查询源坐标归属失败：{e}"))?;
        if occupied {
            return Err(format!(
                "IdentityConflict：source_coord `{}` 已属于另一业务身份（不覆盖原始事件或见证）",
                e.seq
            ));
        }
        let content = canonical_event_content(e);
        let payload_hash = sha256_hex(content.as_bytes());

        let existing = qrev
            .query_row(params![ikey, rev], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .optional()
            .map_err(|e| format!("查询 identity 失败：{e}"))?;

        if let Some((rec, ph, coord, seq)) = existing {
            if ph == payload_hash {
                results.push(json!({
                    "status": "replay",
                    "identity_key": ikey,
                    "event_id": e.event_id,
                    "revision": rev.to_string(),
                    "receipt_id": rec,
                    "payload_hash": payload_hash,
                    "seq": seq.to_string(),
                    "source_coord": coord,
                }));
            } else {
                results.push(json!({
                    "status": "IdentityConflict",
                    "identity_key": ikey,
                    "event_id": e.event_id,
                    "revision": rev.to_string(),
                    "existing_receipt_id": rec,
                    "existing_payload_hash": ph,
                    "new_payload_hash": payload_hash,
                }));
            }
            continue;
        }

        // 该 (identity, revision) 未见过。
        let max_rev: i64 = qmax
            .query_row(params![ikey], |r| r.get(0))
            .map_err(|e| format!("读 max revision 失败：{e}"))?;
        if rev <= max_rev {
            // 旧版（已有更高 revision）且该 revision 无记录：晚到/跳号旧版不复活、不新接纳。
            results.push(json!({
                "status": "InvalidDomain",
                "identity_key": ikey,
                "event_id": e.event_id,
                "revision": rev.to_string(),
                "max_revision": max_rev.to_string(),
                "reason": "out_of_order_revision_after_later_accepted",
            }));
            continue;
        }
        // 新 revision：supersedes = 上一 revision（如有）。
        let supersedes = if max_rev == 0 { None } else { Some(max_rev) };
        // 源坐标一致性：同一业务身份的新 revision 必须落在同一源坐标。
        if let Some(prev_rev) = supersedes {
            let prev_coord: String = tx
                .query_row(
                    "SELECT source_coord FROM raw_events WHERE identity_key=?1 AND revision=?2",
                    params![ikey, prev_rev],
                    |r| r.get(0),
                )
                .map_err(|e| format!("读前一 revision 源坐标失败：{e}"))?;
            if prev_coord != e.seq {
                return Err(format!(
                    "IdentityConflict：事件 `{}` 新 revision=`{rev}` 源坐标 `{}` 与已接纳 revision=`{prev_rev}` 源坐标 `{prev_coord}` 不一致（同一业务身份不得换源位置）",
                    e.event_id, e.seq
                ));
            }
        }

        next_seq += 1;
        let receipt_id = format!(
            "rcpt-{}",
            &sha256_hex(format!("{ikey}|{payload_hash}").as_bytes())[..16]
        );
        insert
            .execute(params![
                ikey,
                rev,
                e.revision,
                payload_hash,
                receipt_id,
                next_seq,
                input.source_namespace,
                input.source_epoch,
                input.instrument,
                e.event_id,
                e.received_at,
                e.raw_text,
                e.price,
                e.timestamp,
                e.volume,
                e.seq,
                supersedes,
            ])
            .map_err(|e| format!("insert raw_events 失败：{e}"))?;
        if let Some(prices) = &e.ohlc {
            tb02a::save_raw(&tx, &ikey, rev, prices)?;
        }
        results.push(json!({
            "status": "accepted",
            "identity_key": ikey,
            "event_id": e.event_id,
            "revision": rev.to_string(),
            "receipt_id": receipt_id,
            "payload_hash": payload_hash,
            "seq": next_seq.to_string(),
            "source_coord": e.seq,
            "supersedes_revision": supersedes.map(|v| v.to_string()),
        }));
    }

    Ok(results)
}
