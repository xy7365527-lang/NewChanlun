//! #1527：固定 profile 绑定与批次原件读取的 mixed 资源入口。
use super::*;
use crate::v3::{
    audit_budget::LegacyObserver,
    business_json::{MixedJsonError, PlainJson},
    native_shared::NativeArc,
};
use newchan_rust::s_content::{
    capture_allocation::{native_owned::NativeValue, scratch::WorkScope},
    CodecVec,
};
type Text = NativeValue<String>;
pub(crate) fn meta_text(
    conn: &Connection,
    key: &str,
    work: &WorkScope,
) -> Result<Option<Text>, MixedJsonError> {
    let error = |e| format!("读 meta.{key} 失败：{e}");
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        conn,
        c"SELECT value FROM meta WHERE key = ?1",
    )
    .map_err(error)?;
    stmt.bind_text(1, key).map_err(error)?;
    let value = match stmt.next().map_err(error)? {
        Some(mut row) => Some(
            Text::try_from_str(row.text(0).map_err(error)?, Some(work))
                .map_err(MixedJsonError::Resource)?,
        ),
        None => None,
    };
    stmt.finalize().map_err(error)?;
    Ok(value)
}
pub(crate) fn batch_bytes(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
    work: &WorkScope,
) -> Result<NativeArc<[u8]>, MixedJsonError> {
    #[cfg(test)]
    cache::counter(|c| c.batch_full_reads += 1);
    let (blob, len, graph) = crate::read_batch_storage_typed(conn, batch_id, None, Some(work))?;
    let key = Text::try_format(format_args!("batch/{batch_id}"), Some(work))
        .map_err(MixedJsonError::Resource)?;
    if !graph {
        let bytes = NativeArc::<[u8]>::try_copy_slice(&blob, Some(work))
            .map_err(MixedJsonError::Resource)?;
        let digest = crate::blob_digest(&blob);
        cache::handle_put_blob_typed(&key, digest, bytes.clone(), None, Some(work))
            .map_err(MixedJsonError::Resource)?;
        return Ok(bytes);
    }
    let root = reader.read_root_typed(&key, false)?;
    #[cfg(test)]
    cache::counter(|c| {
        if root.reused {
            c.batch_handle_hits += 1;
        } else {
            c.batch_expansions += 1;
        }
    });
    crate::verify_batch_root_binding(batch_id, len, root.declared_length, &root.content_sha256)?;
    Ok(root.bytes)
}
fn profile_hash(
    value: &PlainJson,
) -> Result<newchan_rust::session_protocol::FixedHash, MixedJsonError> {
    newchan_rust::session_protocol::canonical_hash_view_with_owner(value.as_ref().into(), None)
        .map_err(Into::into)
}
pub(crate) fn verify_profile(
    conn: &Connection,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<(Text, Text), MixedJsonError> {
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    let id = meta_text(conn, "profile_id", work)?;
    let hash = meta_text(conn, "profile_hash", work)?;
    let input = meta_text(conn, "input_profile", work)?;
    let definition = meta_text(conn, "profile_definition", work)?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if id.is_none() && hash.is_none() && input.is_none() && definition.is_none() && count == 0 {
        return Ok((
            Text::try_from_str("", Some(work)).map_err(MixedJsonError::Resource)?,
            Text::try_from_str("", Some(work)).map_err(MixedJsonError::Resource)?,
        ));
    }
    let id = id
        .filter(|v| !v.is_empty())
        .ok_or("StorageUnavailable：已绑定profile_id缺失")?;
    let hash = hash
        .filter(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or("StorageUnavailable：已绑定profile_hash缺失或不规范")?;
    if input.as_ref().map(|v| v.as_str()) != Some(id.as_str()) {
        return Err("StorageUnavailable：input_profile与固定profile_id不一致".into());
    }
    if let Some(definition) = definition {
        let profile = crate::json_shape_mixed(&definition, crate::JsonShape::Object, work)?;
        if profile["profile_id"].as_str() != Some(id.as_str())
            || profile_hash(&profile)?.as_str() != hash.as_str()
        {
            return Err("StorageUnavailable：profile定义与固定id/hash不一致".into());
        }
    } else {
        let mut statement=newchan_rust::sqlite_checked::CheckedStatement::prepare(conn,
            c"SELECT b.batch_id FROM batches b JOIN structure_deltas d ON d.index_frontier=b.batch_id ORDER BY d.generation ASC").map_err(|e|e.to_string())?;
        let mut ids = CodecVec::default();
        while let Some(mut row) = statement.next().map_err(|e| e.to_string())? {
            ids.try_reserve_native(work, 1)
                .map_err(MixedJsonError::Resource)?;
            let id = Text::try_from_str(row.text(0).map_err(|e| e.to_string())?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            ids.try_push(id).map_err(MixedJsonError::Resource)?;
        }
        statement.finalize().map_err(|e| e.to_string())?;
        let mut found = false;
        let mut reader =
            evidence_store::Reader::new_mixed(conn, crate::EVIDENCE_READ_BUDGET, work, observer)?;
        for batch_id in &ids {
            let bytes = batch_bytes(conn, &mut reader, batch_id, work)?;
            let batch = crate::v3::business_json::mixed_legacy_json_owner(&bytes, work)?;
            if batch["profile_id"].as_str().unwrap_or("").is_empty() {
                continue;
            }
            if batch["profile_id"].as_str() != Some(id.as_str())
                || batch["profile_hash"].as_str() != Some(hash.as_str())
            {
                return Err("StorageUnavailable：旧profile绑定与可达封存批次不一致".into());
            }
            found = true;
            break;
        }
        if !found {
            let profile = crate::json_shape_mixed(
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../s_session/profiles/testonly_tick_1_1_ohlc.json"
                )),
                crate::JsonShape::Object,
                work,
            )?;
            if profile["profile_id"].as_str() != Some(id.as_str())
                || profile_hash(&profile)?.as_str() != hash.as_str()
            {
                return Err("StorageUnavailable：旧未发布profile缺可核独立来源".into());
            }
        }
    }
    Ok((id, hash))
}
