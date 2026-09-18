//! #1394 C §5.5：固定切面引用分页、snapshot-token/2与完整Q回复编码。
//! 内部协议核心；尚未注册HTTP服务。编码字节不是已经发生的wire传输。
use super::{
    projection::{HistoryMode, Projection},
    q_resources::{Limit, QResources},
};
use newchan_rust::{
    s_content::job::Job,
    session_protocol::{
        canonical_json, nonnegative, positive, reply_envelope, required, sha256_hex, strict_json,
        validate_envelope,
    },
};
use serde_json::{json, Value};
use std::io::{self, Write};

const TOKEN_FIELDS: [&str; 17] = [
    "token_schema",
    "session_id",
    "session_generation",
    "cut_generation",
    "scope",
    "history_mode",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "profile_hash",
    "rule_revision",
    "order_version",
    "page_size",
    "offset",
    "cut_projection_digest",
    "issued_capture_digest",
    "token_checksum",
];
const FIRST_FIELDS: [&str; 8] = [
    "op",
    "session_id",
    "session_generation",
    "scope",
    "history_mode",
    "as_of_generation",
    "page_size",
    "order_version",
];

pub(super) struct Request {
    envelope: Value,
    bytes: usize,
    generation: Option<i64>,
    mode: HistoryMode,
    page_size: u64,
    offset: u64,
    issued: Option<String>,
}
impl Request {
    /// 先限制完整消息体，再走共同strict envelope/hash校验；旧schema或token不进入v3。
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        if bytes.len() as u128 > u128::from(resources.limit(Limit::FrameBytes)) {
            return Err(err("Incomplete：完整请求超过max_frame_bytes"));
        }
        let envelope = validate_envelope(strict_json(bytes)?, "s-session/3")?;
        let payload = &envelope["payload"];
        if payload["op"] != "snapshot" {
            return Err(err("op必须是snapshot"));
        }
        let (generation, mode, page_size, offset, issued) =
            if payload.get("snapshot_token").is_some() {
                exact(payload, &["op", "snapshot_token"])?;
                let token = &payload["snapshot_token"];
                exact(token, &TOKEN_FIELDS)?;
                if token["token_schema"] != "s-snapshot-token/2" {
                    return Err(err("snapshot_token版本不符"));
                }
                let issued = required(token, "issued_capture_digest")?;
                if issued.len() != 64
                    || !issued
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(err("签发捕获摘要不是64位小写hex"));
                }
                let mut unsigned = token.clone();
                unsigned.as_object_mut().unwrap().remove("token_checksum");
                if token["token_checksum"] != sha256_hex(canonical_json(&unsigned).as_bytes()) {
                    return Err(err("snapshot_token checksum不符"));
                }
                (
                    Some(nonnegative(
                        required(token, "cut_generation")?,
                        "cut_generation",
                    )?),
                    mode(required(token, "history_mode")?)?,
                    positive(required(token, "page_size")?, "page_size")? as u64,
                    nonnegative(required(token, "offset")?, "offset")? as u64,
                    Some(issued.to_owned()),
                )
            } else {
                exact(payload, &FIRST_FIELDS)?;
                let generation = if payload["as_of_generation"].is_null() {
                    None
                } else {
                    Some(nonnegative(
                        required(payload, "as_of_generation")?,
                        "as_of_generation",
                    )?)
                };
                (
                    generation,
                    mode(required(payload, "history_mode")?)?,
                    positive(required(payload, "page_size")?, "page_size")? as u64,
                    0,
                    None,
                )
            };
        let request = Self {
            envelope,
            bytes: bytes.len(),
            generation,
            mode,
            page_size,
            offset,
            issued,
        };
        request.validate_resources(resources)?;
        Ok(request)
    }
    pub(super) fn validate_resources(&self, resources: &QResources) -> Result<(), String> {
        if self.bytes as u128 > u128::from(resources.limit(Limit::FrameBytes))
            || self.page_size > resources.limit(Limit::PageSize)
        {
            return Err(err("Incomplete：请求超过冻结frame/page_size上限"));
        }
        Ok(())
    }
    pub(super) fn generation(&self, head: i64) -> i64 {
        self.generation.unwrap_or(head)
    }
    pub(super) fn history_mode(&self) -> HistoryMode {
        self.mode
    }
}
fn mode(value: &str) -> Result<HistoryMode, String> {
    match value {
        "AsKnown" => Ok(HistoryMode::AsKnown),
        "RecomputedWithRevision" => Ok(HistoryMode::RecomputedWithRevision),
        _ => Err(err("history_mode不符")),
    }
}
fn exact(value: &Value, fields: &[&str]) -> Result<(), String> {
    if value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k.as_str())))
    {
        Ok(())
    } else {
        Err(err("exact字段不完整或多余"))
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QSnapshotInvalid：{e}")
}

/// 校验和重建采用原Q配方；page_size/offset不是签名授权，客户端另核连续性。
fn token(projection: &Projection, issued: &str, page_size: u64, offset: u64) -> Value {
    let fixed = &projection.wire["fixed_cut"];
    let mut result = json!({});
    for key in [
        "session_id",
        "session_generation",
        "cut_generation",
        "scope",
        "history_mode",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "profile_hash",
        "rule_revision",
    ] {
        result[key] = fixed[key].clone();
    }
    result["token_schema"] = json!("s-snapshot-token/2");
    result["order_version"] = projection.wire["order_version"].clone();
    result["page_size"] = json!(page_size.to_string());
    result["offset"] = json!(offset.to_string());
    result["cut_projection_digest"] = json!(projection.digest);
    result["issued_capture_digest"] = json!(issued);
    result["token_checksum"] = json!(sha256_hex(canonical_json(&result).as_bytes()));
    result
}

pub(super) fn reply(
    request: &Request,
    mut projection: Projection,
    capture_digest: &str,
    resources: &QResources,
    producer_epoch: &str,
    job: &mut Job,
) -> Result<Vec<u8>, String> {
    request.validate_resources(resources)?;
    positive(producer_epoch, "Q producer_epoch")?;
    job.pre_io_check().map_err(err)?;
    let fixed = &projection.wire["fixed_cut"];
    let session = required(fixed, "session_id")?.to_owned();
    let incarnation = required(fixed, "session_generation")?.to_owned();
    for key in ["session_id", "session_generation"] {
        if request.envelope[key] != fixed[key] {
            return Err(err("外层session/化身与当前已核会话不符"));
        }
    }
    let issued = request.issued.as_deref().unwrap_or(capture_digest);
    if request.issued.is_some() {
        if request.envelope["payload"]["snapshot_token"]
            != token(&projection, issued, request.page_size, request.offset)
        {
            return Err(err("snapshot_token与本次已核指定历史切面的实际身份/根不符"));
        }
    } else {
        let payload = &request.envelope["payload"];
        for key in ["session_id", "session_generation", "scope"] {
            if payload[key] != fixed[key] {
                return Err(err(format!("首请求{key}与当前已核会话不符")));
            }
        }
        if payload["order_version"] != projection.wire["order_version"] {
            return Err(err("order_version不符"));
        }
    }
    let total = projection.wire["rows"]
        .as_array()
        .ok_or_else(|| err("投影rows不是数组"))?
        .len() as u64;
    if request.offset > total {
        return Err(err("offset超过完整切面"));
    }
    let end = request
        .offset
        .checked_add(request.page_size)
        .ok_or_else(|| err("offset+page_size溢出"))?
        .min(total);
    let done = end == total;
    let next = if done {
        Value::Null
    } else {
        token(&projection, issued, request.page_size, end)
    };
    let rows = projection.wire["rows"].as_array_mut().unwrap();
    rows.drain(..request.offset as usize);
    rows.truncate((end - request.offset) as usize);
    let mut page = projection.wire;
    page["offset"] = json!(request.offset.to_string());
    page["done"] = json!(done);
    page["next_token"] = next;
    page["cut_projection_digest"] = json!(projection.digest);
    page["validated_capture_digest"] = json!(capture_digest);
    page["ok"] = json!(true);
    // 先界住payload，随后核包含完整11字段头、因果引用和消息ID的实际消息体。
    // 无LF/HTTP头；编码失败丢弃整个缓冲，不把它记为已发送wire字节。
    encode(&page, resources.limit(Limit::ReplyBytes), job)?;
    let envelope = reply_envelope(
        &request.envelope,
        page,
        "s-session/3",
        &session,
        &incarnation,
        "s-observe/replies",
        &format!("Q:{session}"),
        producer_epoch,
    );
    encode(&envelope, resources.limit(Limit::ReplyBytes), job)
}

fn encode(value: &Value, limit: u64, job: &mut Job) -> Result<Vec<u8>, String> {
    struct Buffer<'a> {
        bytes: Vec<u8>,
        limit: u64,
        job: &'a mut Job,
    }
    impl Write for Buffer<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.job.pre_io_check().map_err(io::Error::other)?;
            if self.bytes.len() as u128 + bytes.len() as u128 > u128::from(self.limit) {
                return Err(io::Error::other(
                    "Incomplete：完整Q消息体超过max_reply_bytes",
                ));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    // 与共同canonical_json相同的排序和标量编码，逐段写入显式上界，避免先展开整份输出。
    fn write(value: &Value, out: &mut impl Write) -> io::Result<()> {
        match value {
            Value::Object(map) => {
                out.write_all(b"{")?;
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for (i, key) in keys.into_iter().enumerate() {
                    if i > 0 {
                        out.write_all(b",")?;
                    }
                    serde_json::to_writer(&mut *out, key).map_err(io::Error::other)?;
                    out.write_all(b":")?;
                    write(&map[key], out)?;
                }
                out.write_all(b"}")
            }
            Value::Array(values) => {
                out.write_all(b"[")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        out.write_all(b",")?;
                    }
                    write(value, out)?;
                }
                out.write_all(b"]")
            }
            _ => serde_json::to_writer(out, value).map_err(io::Error::other),
        }
    }
    let mut out = Buffer {
        bytes: Vec::new(),
        limit,
        job,
    };
    write(value, &mut out).map_err(err)?;
    out.job.pre_io_check().map_err(err)?;
    Ok(out.bytes)
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{capture::capture_configured, prepared::tests::Fixture, q_resources};
    use super::*;
    use newchan_rust::s_content::job::{Existing, JobLimits};
    use std::{path::Path, process::Command, time::Instant};
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"100","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn envelope(fixed: &Value, payload: Value) -> Value {
        json!({"schema_revision":"s-session/3","session_id":fixed["session_id"],"session_generation":fixed["session_generation"],
            "source_namespace":"test.query/界","source_epoch":"1","message_id":"m-\n\"1","producer_id":"q-test",
            "producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }
    fn first(projection: &Projection, size: u64, mode: &str, as_of: Value) -> Value {
        let fixed = &projection.wire["fixed_cut"];
        envelope(
            fixed,
            json!({"op":"snapshot","session_id":fixed["session_id"],"session_generation":fixed["session_generation"],
            "scope":fixed["scope"],"history_mode":mode,"as_of_generation":as_of,"page_size":size.to_string(),
            "order_version":projection.wire["order_version"]}),
        )
    }
    fn request(value: &Value, resources: &QResources) -> Request {
        Request::parse(canonical_json(value).as_bytes(), resources).unwrap()
    }
    fn resign(token: &mut Value) {
        token.as_object_mut().unwrap().remove("token_checksum");
        token["token_checksum"] = json!(sha256_hex(canonical_json(token).as_bytes()));
    }
    fn fixture_projection(f: &Fixture, resources: &QResources, g: i64) -> Projection {
        capture_configured(&f.db_path(), resources, &mut job())
            .unwrap()
            .projection(g, HistoryMode::AsKnown, &mut job())
            .unwrap()
    }
    #[test]
    fn real_pages_rebuild_old_cut_after_commit_and_restart_preserving_issued_digest() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let expected = fixture_projection(&f, &resources, 1);
        let wanted = expected.wire["rows"].as_array().unwrap();
        assert!(wanted.len() > 1);
        let mut incoming = first(&expected, 1, "AsKnown", Value::Null);
        let mut job = job();
        let mut rows = Vec::new();
        let mut issued = String::new();
        let mut latest = String::new();
        let mut pages = 0;
        loop {
            // 每一页新捕获；前一个实例销毁，模拟Q重启，不靠服务端token缓存。
            let mut capture = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
            let bytes = capture
                .snapshot(&request(&incoming, &resources), "7", &mut job)
                .unwrap();
            let response = strict_json(&bytes).unwrap();
            assert_eq!(bytes, canonical_json(&response).as_bytes());
            assert_eq!(response.as_object().unwrap().len(), 11);
            assert_eq!(response["schema_revision"], "s-session/3");
            let page = &response["payload"];
            assert_eq!(page.as_object().unwrap().len(), 13);
            assert!(page.get("projection_ref").is_none());
            assert_eq!(
                response["payload_hash"],
                sha256_hex(canonical_json(page).as_bytes())
            );
            assert_eq!(
                response["causal_refs"][0]["payload_hash"],
                incoming["payload_hash"]
            );
            assert_eq!(page["fixed_cut"], expected.wire["fixed_cut"]);
            assert_eq!(page["counts"], expected.wire["counts"]);
            assert_eq!(page["cut_projection_digest"], expected.digest);
            assert_eq!(page["offset"], rows.len().to_string());
            rows.extend(page["rows"].as_array().unwrap().iter().cloned());
            pages += 1;
            if pages == 1 {
                issued = page["validated_capture_digest"].as_str().unwrap().into();
                drop(capture);
                publish(&mut f, 2);
            } else {
                let current = page["validated_capture_digest"].as_str().unwrap();
                assert_ne!(current, issued);
                if latest.is_empty() {
                    latest = current.into();
                } else {
                    assert_eq!(current, latest);
                }
            }
            if page["done"] == true {
                assert!(page["next_token"].is_null());
                break;
            }
            let next = &page["next_token"];
            assert_eq!(next.as_object().unwrap().len(), 17);
            assert_eq!(next["issued_capture_digest"], issued);
            assert_eq!(next["offset"], rows.len().to_string());
            incoming = envelope(
                &expected.wire["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":next}),
            );
            assert!(pages <= wanted.len());
        }
        assert_eq!(&rows, wanted);
        assert_eq!(job.snapshot().records, (pages * wanted.len()) as u64);
        assert_eq!(job.snapshot().requests, pages as u64);
        assert_eq!(job.snapshot().wire_bytes, 0, "编码不冒充真实发送");
    }
    #[test]
    fn zero_cut_and_original_mode_generation_combinations_are_preserved() {
        let mut f = Fixture::new();
        let resources = q_resources::tests::resources();
        let zero = fixture_projection(&f, &resources, 0);
        let bytes = capture_configured(&f.db_path(), &resources, &mut job())
            .unwrap()
            .snapshot(
                &request(&first(&zero, 31, "AsKnown", Value::Null), &resources),
                "1",
                &mut job(),
            )
            .unwrap();
        let page = &strict_json(&bytes).unwrap()["payload"];
        assert_eq!(page["rows"], json!([]));
        assert_eq!(page["counts"]["total"], "0");
        assert_eq!(page["offset"], "0");
        assert_eq!(page["done"], true);
        assert!(page["next_token"].is_null());
        assert_eq!(page["fixed_cut"]["as_of_generation"], "0");
        publish(&mut f, 1);
        let bytes = capture_configured(&f.db_path(), &resources, &mut job())
            .unwrap()
            .snapshot(
                &request(
                    &first(&zero, 31, "RecomputedWithRevision", json!("0")),
                    &resources,
                ),
                "1",
                &mut job(),
            )
            .unwrap();
        let response = strict_json(&bytes).unwrap();
        assert_eq!(response["payload"]["fixed_cut"]["cut_generation"], "0");
        assert!(response["payload"]["fixed_cut"]["as_of_generation"].is_null());
    }
    #[test]
    fn checksummed_forged_identity_and_root_do_not_replace_actual_cut_validation() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let expected = fixture_projection(&f, &resources, 1);
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let bytes = capture
            .snapshot(
                &request(&first(&expected, 1, "AsKnown", json!("1")), &resources),
                "1",
                &mut job(),
            )
            .unwrap();
        let token = strict_json(&bytes).unwrap()["payload"]["next_token"].clone();
        assert!(token.is_object());
        for (key, value) in [
            ("session_id", json!("foreign")),
            ("session_generation", json!("99")),
            ("profile_hash", json!("bad")),
            ("scope", json!({})),
            ("structure_cut", json!("cut-0")),
            ("order_version", json!("s-record-order/1")),
            (
                "cut_projection_digest",
                json!(format!("qcut3-{}", "0".repeat(64))),
            ),
        ] {
            let mut forged = token.clone();
            forged[key] = value;
            resign(&mut forged);
            let envelope = envelope(
                &expected.wire["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":forged}),
            );
            assert!(
                capture
                    .snapshot(&request(&envelope, &resources), "1", &mut job())
                    .is_err(),
                "{key}"
            );
        }
        let mut incoming = first(&expected, 1, "AsKnown", Value::Null);
        incoming["session_id"] = json!("foreign");
        assert!(capture
            .snapshot(&request(&incoming, &resources), "1", &mut job())
            .unwrap_err()
            .contains("外层session"));
        let mut corrupted = token.clone();
        corrupted["offset"] = json!("0");
        let incoming = envelope(
            &expected.wire["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":corrupted}),
        );
        assert!(Request::parse(canonical_json(&incoming).as_bytes(), &resources).is_err());
        let mut old = token.clone();
        old["token_schema"] = json!("s-snapshot-token/1");
        resign(&mut old);
        let incoming = envelope(
            &expected.wire["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":old}),
        );
        assert!(Request::parse(canonical_json(&incoming).as_bytes(), &resources).is_err());
        // 原合同允许合法非整页倍数作为独立区间，不能将校验和升级为服务端授权签名。
        let mut standalone = token.clone();
        standalone["page_size"] = json!("2");
        standalone["offset"] = json!("1");
        resign(&mut standalone);
        let incoming = envelope(
            &expected.wire["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":standalone}),
        );
        let page = strict_json(
            &capture
                .snapshot(&request(&incoming, &resources), "1", &mut job())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(page["payload"]["offset"], "1");
        let total = expected.wire["rows"].as_array().unwrap().len();
        let mut terminal = token;
        terminal["offset"] = json!(total.to_string());
        resign(&mut terminal);
        let incoming = envelope(
            &expected.wire["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":terminal}),
        );
        let page = strict_json(
            &capture
                .snapshot(&request(&incoming, &resources), "1", &mut job())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(page["payload"]["rows"], json!([]));
        assert_eq!(page["payload"]["done"], true);
    }
    #[test]
    fn full_envelope_exact_byte_limits_and_original_python_reply_recipe_match() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let expected = fixture_projection(&f, &resources, 1);
        let incoming = first(&expected, 1, "AsKnown", Value::Null);
        let request_bytes = canonical_json(&incoming).into_bytes();
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let bytes = capture
            .snapshot(&request(&incoming, &resources), "3", &mut job())
            .unwrap();
        let response = strict_json(&bytes).unwrap();
        let script = r#"import sys,json,types
sys.path.insert(0,sys.argv[1]);import s_query as q
canonical=lambda v:json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
e=json.loads(sys.argv[2]);p=json.loads(sys.argv[3]);proof={'meta':{'session_id':e['session_id']},'protocol':{'session_generation':e['session_generation']}}
r=q.response_envelope(e,p,proof,3,types.SimpleNamespace(canonical=canonical));r['schema_revision']='s-session/3'
sys.stdout.buffer.write(canonical(r))"#;
        let oracle = Command::new("python3")
            .args(["-c", script])
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("s_session"),
            )
            .arg(canonical_json(&incoming))
            .arg(canonical_json(&response["payload"]))
            .output()
            .unwrap();
        assert!(
            oracle.status.success(),
            "{}",
            String::from_utf8_lossy(&oracle.stderr)
        );
        assert_eq!(bytes, oracle.stdout);
        assert!(!bytes.ends_with(b"\n"));
        for (name, n) in [
            ("max_frame_bytes", request_bytes.len()),
            ("max_reply_bytes", bytes.len()),
        ] {
            let mut config = q_resources::tests::input();
            config["limits"][name] = json!(n.to_string());
            let exact = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
            let parsed = Request::parse(&request_bytes, &exact).unwrap();
            let encoded = capture_configured(&f.db_path(), &exact, &mut job())
                .unwrap()
                .snapshot(&parsed, "3", &mut job())
                .unwrap();
            assert_eq!(encoded, bytes);
            config["limits"][name] = json!((n - 1).to_string());
            let short = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
            if name == "max_frame_bytes" {
                assert!(Request::parse(&request_bytes, &short)
                    .err()
                    .unwrap()
                    .contains("max_frame_bytes"));
            } else {
                assert!(canonical_json(&response["payload"]).len() < n - 1);
                assert!(capture_configured(&f.db_path(), &short, &mut job())
                    .unwrap()
                    .snapshot(&request(&incoming, &short), "3", &mut job())
                    .unwrap_err()
                    .contains("max_reply_bytes"));
            }
        }
        let duplicate =
            canonical_json(&incoming).replacen("{", "{\"schema_revision\":\"s-session/3\",", 1);
        assert!(Request::parse(duplicate.as_bytes(), &resources).is_err());
        let mut legacy = incoming;
        legacy["schema_revision"] = json!("s-session/2");
        assert!(Request::parse(canonical_json(&legacy).as_bytes(), &resources).is_err());
    }
}
