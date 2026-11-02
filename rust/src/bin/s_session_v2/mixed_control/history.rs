//! #1527：控制历史的 typed 资源传播，复用原业务谓词与 ControlHistory 消费核。
use super::*;
fn from_str(value: &str, work: &WorkScope) -> Result<TextValue, MixedJsonError> {
    Text::try_from_str(value, Some(work))
        .map(TextValue::Plain)
        .map_err(MixedJsonError::Resource)
}
fn text(
    row: &mut CheckedRow<'_>,
    column: usize,
    work: &WorkScope,
) -> Result<TextValue, MixedJsonError> {
    sql_text(row, column, work).map(TextValue::Plain)
}
fn meta(
    conn: &Connection,
    key: &str,
    work: &WorkScope,
) -> Result<Option<TextValue>, MixedJsonError> {
    crate::audit::mixed_profile::meta_text(conn, key, work).map(|v| v.map(TextValue::Plain))
}
fn meta_i64(conn: &Connection, key: &str, work: &WorkScope) -> Result<i64, MixedJsonError> {
    let value =
        meta(conn, key, work)?.ok_or_else(|| format!("StorageUnavailable：meta.{key} 缺失"))?;
    let label = Text::try_format(format_args!("meta.{key}"), Some(work))
        .map_err(MixedJsonError::Resource)?;
    parse_canonical_i64(&value, &label).map_err(|e| {
        crate::v3::business_json::mixed_diagnostic(format_args!("StorageUnavailable：{e}")).into()
    })
}
fn push<T>(values: &mut CodecVec<T>, value: T, work: &WorkScope) -> Result<(), MixedJsonError> {
    control_push(values, value, Some(work)).map_err(MixedJsonError::Resource)
}
fn insert<K: Eq + std::hash::Hash, V>(
    values: &mut BusinessMap<K, V>,
    key: K,
    value: V,
    work: &WorkScope,
) -> Result<Option<V>, MixedJsonError> {
    control_insert(values, key, value, Some(work)).map_err(MixedJsonError::Resource)
}
fn sorted_keys<'a, K: Eq + std::hash::Hash + Ord, V>(
    map: &'a BusinessMap<K, V>,
    work: &WorkScope,
) -> Result<CodecVec<&'a K>, MixedJsonError> {
    let mut keys = CodecVec::default();
    for (key, _) in map.iter() {
        push(&mut keys, key, work)?;
    }
    keys.sort_unstable();
    Ok(keys)
}
fn query_row<T>(
    conn: &Connection,
    sql: &CStr,
    params: &[ControlBind<'_>],
    read: impl FnOnce(&mut CheckedRow<'_>) -> Result<T, MixedJsonError>,
) -> Result<T, MixedJsonError> {
    let mut stmt = CheckedStatement::prepare(conn, sql).map_err(err)?;
    for (i, p) in params.iter().enumerate() {
        match p {
            ControlBind::Text(s) => stmt.bind_text(i as i32 + 1, s),
            ControlBind::Integer(n) => stmt.bind_integer(i as i32 + 1, *n),
        }
        .map_err(err)?;
    }
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
    let result = read(&mut row)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    Ok(result)
}
fn canonical(value: JsonView<'_>, work: &WorkScope) -> Result<TextValue, MixedJsonError> {
    struct Sink<'a> {
        bytes: CodecVec<u8>,
        work: &'a WorkScope,
        failure: Option<newchan_rust::s_content::CodecError>,
    }
    impl std::io::Write for Sink<'_> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if let Err(e) = control_extend(&mut self.bytes, b, Some(self.work)) {
                self.failure = Some(e);
                return Err(std::io::Error::other("mixed canonical allocation"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink {
        bytes: CodecVec::default(),
        work,
        failure: None,
    };
    let result = serde_json::to_writer(&mut sink, &value);
    if let Some(e) = sink.failure {
        return Err(MixedJsonError::Resource(e));
    }
    result.map_err(err)?;
    sink.bytes
        .try_into_native_text()
        .map(TextValue::Plain)
        .map_err(MixedJsonError::Resource)
}
fn read_message(r: &mut CheckedRow<'_>, work: &WorkScope) -> Result<AuditMessage, MixedJsonError> {
    Ok(AuditMessage {
        ns: text(r, 0, work)?,
        se: text(r, 1, work)?,
        id: text(r, 2, work)?,
        hash: text(r, 3, work)?,
        raw: text(r, 4, work)?,
        receipt: text(r, 5, work)?,
        seq: r.integer(6).map_err(err)?,
        status: text(r, 7, work)?,
        published: r.optional_integer(8).map_err(err)?,
        event: text(r, 9, work)?,
        count: r.integer(10).map_err(err)?,
    })
}

fn validate_message(
    conn: &Connection,
    m: &AuditMessage,
    protocol_revision: &str,
    sid: &str,
    inc: &str,
    clock: JsonView<'_>,
    first_pub: Option<i64>,
    work: &WorkScope,
) -> Result<TextValue, MixedJsonError> {
    use control_json::field;
    use newchan_rust::session_protocol::validate_envelope_view_with_owner;
    let AuditMessage {
        ns,
        se,
        id,
        hash,
        raw,
        receipt,
        seq,
        status,
        published,
        event,
        count,
    } = m;
    let (seq, count, published) = (*seq, *count, *published);
    let envelope = super::decode(&raw, work)?;
    let e = validate_envelope_view_with_owner(envelope.as_ref().into(), protocol_revision, None)
        .map_err(err)?;
    if !control_canonical_equal(e, &raw, None)?
        || field(e, "source_namespace").as_str() != Some(ns.as_str())
        || field(e, "source_epoch").as_str() != Some(se.as_str())
        || field(e, "message_id").as_str() != Some(id.as_str())
        || field(e, "payload_hash").as_str() != Some(hash.as_str())
        || field(e, "session_id").as_str() != Some(sid)
        || field(e, "session_generation").as_str() != Some(inc)
        || field(field(e, "payload"), "clock_event_id").as_str() != Some(event.as_str())
        || count < 0
        || seq < 0
    {
        return Err(err("消息独立列/规范原件不一致").into());
    }
    validate_ingest_view(e).map_err(err)?;
    let input = tb02a::parse_input_view(field(field(e, "payload"), "raw_input")).map_err(err)?;
    if input.events_len() != 1 {
        return Err(err("持久输入消息非单事件").into());
    }
    let ev = input.event(0).map_err(err)?;
    let ik = Text::try_format(
        format_args!(
            "{}|{}|{}|{}",
            input.source_namespace, input.source_epoch, input.instrument, ev.event_id
        ),
        Some(work),
    )
    .map_err(MixedJsonError::Resource)?;
    let actual = query_row(
        conn,
        c"SELECT receipt_id,payload_hash,identity_key,revision FROM raw_events WHERE seq=?1",
        &[ControlBind::Integer(seq)],
        |mut r| {
            Ok((
                text(&mut r, 0, work)?,
                text(&mut r, 1, work)?,
                text(&mut r, 2, work)?,
                r.integer(3).map_err(err)?,
            ))
        },
    )?;
    if &actual.0 != receipt
        || actual.1.as_str() != control_event_hash(ev, None).map_err(err)?.as_str()
        || actual.2.as_str() != ik.as_str()
        || actual.3 != parse_canonical_i64(ev.revision, "revision").map_err(err)?
    {
        return Err(err("消息/raw receipt内容或身份不一致").into());
    }
    if published != first_pub
        || status.as_str()
            != if published.is_some() {
                "Committed"
            } else {
                "AcceptedPending"
            }
    {
        return Err(err("消息持久结果与最早已发布前沿不符").into());
    }
    newchan_rust::session_protocol::clock_phase_view(clock, &event, "accept", -1).map_err(err)?;
    let key_fields = [
        JsonView::String(&ns),
        JsonView::String(&se),
        JsonView::String(&id),
    ];
    canonical(JsonView::ArraySlice(&key_fields), work)
}

pub(crate) fn validate(
    conn: &Connection,
    protocol_revision: &str,
    mut epoch_facts: CodecVec<(TextValue, i64)>,
    work: &WorkScope,
) -> Result<ControlHistory, MixedJsonError> {
    if !matches!(protocol_revision, "s-session/2" | "s-session/3") {
        return Err(err(format!(
            "协议版本参数只允许 s-session/2 或 s-session/3，收到 `{protocol_revision}`"
        ))
        .into());
    }
    for (table, count_sql, singleton_sql) in [
        (
            "s_protocol_meta",
            "SELECT COUNT(*) FROM s_protocol_meta",
            "SELECT singleton FROM s_protocol_meta",
        ),
        (
            "s_delivery_policy",
            "SELECT COUNT(*) FROM s_delivery_policy",
            "SELECT singleton FROM s_delivery_policy",
        ),
    ] {
        let count: i64 = conn.query_row(count_sql, [], |r| r.get(0)).map_err(err)?;
        let singleton: i64 = conn
            .query_row(singleton_sql, [], |r| r.get(0))
            .map_err(err)?;
        if count != 1 || singleton != 1 {
            return Err(err(format!("{table}必须全表唯一且singleton=1")).into());
        }
    }
    for key in [
        "profile_id",
        "profile_hash",
        "profile_definition",
        "input_profile",
    ] {
        if meta(conn, key, work)?.is_none_or(|s| s.is_empty()) {
            return Err(err(format!("v2初始化缺完整{key}绑定")).into());
        }
    }
    positive(
        &meta(conn, "writer_epoch", work)?.ok_or_else(|| err("缺writer_epoch"))?,
        "v2 writer_epoch",
    )
    .map_err(err)?;
    let clock = super::clock(conn, work)?;
    let (protocol, inc, next, frontier) = query_row(conn,c"SELECT protocol_revision,session_generation,next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1", &[], |r| Ok((text(r,0,work)?,text(r,1,work)?,r.integer(2).map_err(err)?,text(r,3,work)?)))?;
    if protocol != protocol_revision || next < 1 {
        return Err(err("协议版本/attempt域错误").into());
    }
    positive(&inc, "session_generation").map_err(err)?;
    let root_g = meta_i64(conn, "generation", work)?;
    let sid = meta(conn, "session_id", work)?.ok_or_else(|| err("缺session"))?;
    super::catalog(conn, work)?;
    let (ps,pi,pr,keep,first,head) = query_row(conn,c"SELECT session_id,session_generation,policy_revision,retain_generations,first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1", &[], |r| Ok((text(r,0,work)?,text(r,1,work)?,text(r,2,work)?,r.integer(3).map_err(err)?,r.integer(4).map_err(err)?,r.integer(5).map_err(err)?)))?;
    validate_delivery_policy(&ps, &pi, &pr, keep, first, head, &sid, &inc, root_g)?;
    let mut stmt = conn
        .prepare("SELECT generation FROM s_delivery_refs ORDER BY generation")
        .map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut refs = CodecVec::default();
    while let Some(r) = rows.next().map_err(err)? {
        push(&mut refs, r.get::<_, i64>(0).map_err(err)?, work)?;
    }
    if refs.len() as i128 != (head as i128 - first as i128 + 1).max(0)
        || refs
            .iter()
            .enumerate()
            .any(|(i, g)| *g as i128 != first as i128 + i as i128)
    {
        return Err(err("普通投递引用缺失/未来/越保留界").into());
    }
    let mut publication_frontiers = CodecVec::default();
    {
        let mut q = conn
            .prepare("SELECT generation,input_frontier FROM structure_deltas ORDER BY generation")
            .map_err(err)?;
        let mut rows = q.query([]).map_err(err)?;
        while let Some(r) = rows.next().map_err(err)? {
            push(
                &mut publication_frontiers,
                (
                    r.get::<_, i64>(0).map_err(err)?,
                    r.get::<_, i64>(1).map_err(err)?,
                ),
                work,
            )?;
        }
    }
    if publication_frontiers.windows(2).any(|w| w[0].1 > w[1].1) {
        return Err(err("发布前沿倒退").into());
    }
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count FROM s_input_messages ORDER BY accepted_seq,message_id").map_err(err)?;
    let mut messages = control_json::new_map(None);
    let mut message_sequences = control_json::new_map::<i64, ()>(None);
    while let Some(mut r) = stmt.next().map_err(err)? {
        let m = read_message(&mut r, work)?;
        let first_pub = publication_frontiers
            .get(publication_frontiers.partition_point(|(_, frontier)| *frontier < m.seq))
            .map(|(g, _)| *g);
        let key = validate_message(
            conn,
            &m,
            protocol_revision,
            &sid,
            &inc,
            clock.value.as_ref().into(),
            first_pub,
            work,
        )?;
        let AuditMessage {
            seq,
            event,
            count,
            published,
            ..
        } = m;
        insert(&mut message_sequences, seq, (), work)?;
        if insert(&mut messages, event, (key, count, published), work)?.is_some() {
            return Err(err("重复clock消息身份").into());
        }
    }
    stmt.finalize().map_err(err)?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(err)?;
    if message_sequences.len() as i128 != raw_count as i128
        || sorted_keys(&message_sequences, work)?
            .iter()
            .enumerate()
            .any(|(i, seq)| **seq as i128 != i as i128)
    {
        return Err(err("v2原始接纳序缺持久消息承接").into());
    }
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT clock_event_id,phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events ORDER BY semantic_ns,clock_event_id,phase,attempt_index").map_err(err)?;
    let mut seen = control_json::new_map(None);
    let mut ordinals = control_json::new_map(None);
    let mut commits_by_generation = control_json::new_map(None);
    let mut begins_by_message: BusinessMap<TextValue, BusinessMap<i64, ()>> =
        control_json::new_map(None);
    let mut recover_facts = CodecVec::default();
    let mut max_ns = -1;
    while let Some(mut r) = stmt.next().map_err(err)? {
        let id = text(&mut r, 0, work)?;
        let phase = text(&mut r, 1, work)?;
        let index = r.integer(2).map_err(err)?;
        let ns = text(&mut r, 3, work)?;
        let key = text(&mut r, 4, work)?;
        let ord = r.optional_integer(5).map_err(err)?;
        let g = r.integer(6).map_err(err)?;
        validate_audit_phase(
            &id,
            &phase,
            index,
            &ns,
            &key,
            ord,
            g,
            root_g,
            next,
            clock.value.as_ref().into(),
            messages
                .get(id.as_str())
                .map(|(k, c, p)| (k.as_str(), *c, *p)),
        )?;
        max_ns = max_ns.max(nonnegative(&ns, "semantic_ns").map_err(err)?);
        if phase == "recover" {
            push(
                &mut recover_facts,
                (
                    ns.try_clone_mixed(work).map_err(MixedJsonError::Resource)?,
                    g,
                ),
                work,
            )?;
        } else {
            messages
                .get(id.as_str())
                .ok_or_else(|| err("phase没有原消息"))?;
            if phase == "begin"
                && insert(
                    &mut ordinals,
                    ord.unwrap(),
                    (
                        id.try_clone_mixed(work).map_err(MixedJsonError::Resource)?,
                        index,
                    ),
                    work,
                )?
                .is_some()
            {
                return Err(err("重复全局attempt").into());
            }
            if phase == "begin" {
                if !begins_by_message.contains_key(id.as_str()) {
                    insert(
                        &mut begins_by_message,
                        id.try_clone_mixed(work).map_err(MixedJsonError::Resource)?,
                        control_json::new_map(None),
                        work,
                    )?;
                }
                insert(
                    begins_by_message.get_mut(id.as_str()).unwrap(),
                    index,
                    (),
                    work,
                )?;
            }
        }
        if phase == "commit"
            && insert(
                &mut commits_by_generation,
                g,
                (
                    id.try_clone_mixed(work).map_err(MixedJsonError::Resource)?,
                    index,
                ),
                work,
            )?
            .is_some()
        {
            return Err(err("每代重复Commit phase").into());
        }
        if insert(&mut seen, (id, phase, index), (ord, g), work)?.is_some() {
            return Err(err("重复phase").into());
        }
    }
    stmt.finalize().map_err(err)?;
    if parse_canonical_i64(&frontier, "logical_phase_frontier").map_err(err)? != max_ns
        || next as i128 != ordinals.len() as i128 + 1
        || sorted_keys(&ordinals, work)?
            .iter()
            .enumerate()
            .any(|(i, o)| **o as i128 != i as i128 + 1)
    {
        return Err(err("phase frontier/attempt序列断裂").into());
    }
    for id in &sorted_keys(&messages, work)? {
        let (_, count, published) = messages.get(id.as_str()).unwrap();
        let indexes = begins_by_message.get(id.as_str());
        let mut sorted_indexes = CodecVec::default();
        if let Some(indexes) = indexes {
            for (i, _) in indexes.iter() {
                push(&mut sorted_indexes, *i, work)?;
            }
        }
        sorted_indexes.sort_unstable();
        let same_pairs = sorted_indexes.iter().all(|i| {
            let begin = seen.get(&PhaseKey(id.as_str(), "begin", *i));
            begin.is_some()
                && seen
                    .get(&PhaseKey(id.as_str(), "commit", *i))
                    .is_none_or(|commit| Some(commit) == begin)
        });
        validate_attempt_summary(
            *count,
            *published,
            seen.contains_key(&PhaseKey(id.as_str(), "accept", -1)),
            &sorted_indexes,
            same_pairs,
            seen.contains_key(&PhaseKey(id.as_str(), "commit", count - 1)),
        )?;
    }
    epoch_facts.sort_unstable();
    recover_facts.sort_unstable();
    if *epoch_facts != *recover_facts {
        return Err(err("recover phase与writer换代时间/代际不闭合").into());
    }
    let mut q = CheckedStatement::prepare(
        conn,
        c"SELECT generation,index_frontier FROM structure_deltas ORDER BY generation",
    )
    .map_err(err)?;
    let mut remaining = control_json::new_map(None);
    while let Some(mut r) = q.next().map_err(err)? {
        let g = r.integer(0).map_err(err)?;
        let batch_id = text(&mut r, 1, work)?;
        let (id, index) = commits_by_generation
            .get(&g)
            .ok_or_else(|| err("v2发布缺真实Commit phase"))?;
        insert(
            &mut remaining,
            (g, batch_id),
            ControlHeader {
                commit_ns: from_str(clock.phase(id, "commit", *index).map_err(err)?, work)?,
                consumed: false,
            },
            work,
        )?;
    }
    q.finalize().map_err(err)?;
    Ok(ControlHistory {
        pending: remaining.len(),
        facts: ControlFacts {
            session_generation: inc,
            clock_plan_hash: TextValue::Plain(clock.hash),
            headers: remaining,
        },
    })
}
