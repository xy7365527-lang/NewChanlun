//! #1394 C §5.5/5.6.1：当前已核捕获上的Watch、Gap和完整发送后租约推进。
//! 尚未注册HTTP服务；body发送适配不包含HTTP头，也不代表客户端已安装Delta。
use super::{
    history::PublishedCut,
    projection::Projection,
    q_delta,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
    q_snapshot, q_wire,
};
use newchan_rust::{
    s_content::{
        self as sc,
        job::{Job, JobNodeSource},
        path::{lookup_path, PathStep},
        NodeSource,
    },
    session_protocol::{nonnegative, positive, required, strict_json},
};
use serde_json::{json, Value};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

const CURSOR_FIELDS: [&str; 9] = [
    "session_id",
    "session_generation",
    "catalog_revision",
    "scope",
    "after_generation",
    "base_cut",
    "index_frontier",
    "last_seq",
    "order_version",
];
pub(super) struct Request {
    envelope: Value,
    cursor: Value,
    client: String,
    after: i64,
    maximum: u64,
    bytes: usize,
}
impl Request {
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        let envelope = q_wire::parse(bytes, resources)?;
        let payload = &envelope["payload"];
        exact(payload, &["op", "cursor", "max_batches", "client_id"])?;
        if payload["op"] != "watch" {
            return Err(err("op必须是watch"));
        }
        let cursor = payload["cursor"].clone();
        exact(&cursor, &CURSOR_FIELDS)?;
        required(&cursor, "session_id")?;
        positive(
            required(&cursor, "session_generation")?,
            "session_generation",
        )?;
        for key in ["session_id", "session_generation"] {
            if envelope[key] != cursor[key] {
                return Err(err("外层消息与cursor身份不同"));
            }
        }
        let after = nonnegative(required(&cursor, "after_generation")?, "after_generation")?;
        let maximum = positive(required(payload, "max_batches")?, "max_batches")? as u64;
        let client = required(payload, "client_id")?.to_owned();
        let request = Self {
            envelope,
            cursor,
            client,
            after,
            maximum,
            bytes: bytes.len(),
        };
        request.validate_resources(resources)?;
        Ok(request)
    }
    pub(super) fn validate_resources(&self, resources: &QResources) -> Result<(), String> {
        if self.bytes as u128 > u128::from(resources.limit(Limit::FrameBytes))
            || self.maximum > resources.limit(Limit::WatchBatches)
        {
            return Err(err("Incomplete：Watch请求超过冻结frame/max_batches界"));
        }
        Ok(())
    }
}
pub(super) struct Policy {
    pub revision: String,
    pub retain: i64,
    pub first: i64,
    pub head: i64,
}
/// 完整当前projection与已核历史来自同一捕获；零代没有出版物，旧代cursor仍按实际历史取值。
pub(super) fn cursor_at(
    current: &Projection,
    cuts: &[PublishedCut],
    generation: i64,
) -> Result<Value, String> {
    let fixed = &current.wire["fixed_cut"];
    let (base, index, last) = if generation == 0 {
        ("cut-0".into(), String::new(), "-1".into())
    } else {
        let cut = cuts
            .iter()
            .find(|c| c.metadata.generation == generation)
            .ok_or_else(|| err("cursor代号未发布"))?;
        (
            cut.metadata.structure_cut.clone(),
            cut.batch_id.clone(),
            cut.metadata.input_frontier.to_string(),
        )
    };
    Ok(
        json!({"session_id":fixed["session_id"],"session_generation":fixed["session_generation"],"catalog_revision":fixed["catalog_revision"],
        "scope":fixed["scope"],"after_generation":generation.to_string(),"base_cut":base,"index_frontier":index,"last_seq":last,
        "order_version":current.wire["order_version"]}),
    )
}
pub(super) fn prepare(
    request: &Request,
    current: Projection,
    cuts: &[PublishedCut],
    source: &mut dyn NodeSource,
    policy: Policy,
    capture_digest: &str,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    job: &mut Job,
) -> Result<Reply, String> {
    request.validate_resources(resources)?;
    if leases.resource_digest() != resources.digest() {
        return Err(err("租约与当前读取不是同一冻结资源配置"));
    }
    let fixed = &current.wire["fixed_cut"];
    let session = required(fixed, "session_id")?.to_owned();
    let incarnation = required(fixed, "session_generation")?.to_owned();
    let head = nonnegative(required(fixed, "cut_generation")?, "cut_generation")?;
    if policy.head != head
        || policy.first < 1
        || policy.retain < 1
        || policy.first > head.saturating_add(1)
    {
        return Err(err("已核交付策略与head不一致"));
    }
    job.begin_first_read().map_err(err)?;
    let mut output = json!({"deltas":[],"next_cursor":request.cursor,"observed_head":fixed,
        "delivery_frontier":{"policy_revision":policy.revision,"retain_generations":policy.retain.to_string(),
            "first_available_generation":policy.first.to_string(),"head_generation":policy.head.to_string()},
        "has_more":false,"gap":null,"validated_capture_digest":capture_digest,"ok":true});
    let same = request.cursor["session_id"] == session
        && request.cursor["session_generation"] == incarnation;
    let (reason, missing) = if !same {
        (Some("session_identity_changed"), Value::Null)
    } else if request.after > head {
        return Err(err("同会话cursor超出永久发布历史"));
    } else if request.cursor != cursor_at(&current, cuts, request.after)? {
        return Err(err(
            "cursor base/index/seq/catalog/scope/order与实际切面不符",
        ));
    } else if request.after < policy.first - 1 {
        (
            Some("delivery_retention_gap"),
            json!({"from_generation":(request.after+1).to_string(),"to_generation":(policy.first-1).to_string()}),
        )
    } else if leases.observe(
        &request.client,
        (&session, &incarnation),
        request.after,
        head,
        Instant::now(),
    )? {
        (
            Some("client_backlog_overflow"),
            json!({"from_generation":(request.after+1).to_string(),"to_generation":head.to_string()}),
        )
    } else {
        (None, Value::Null)
    };
    if let Some(reason) = reason {
        output["gap"] = json!({"requested_identity":request.cursor,"reason":reason,"missing_range":missing,"rebuild":fixed,
            "snapshot_token":q_snapshot::token(&current,capture_digest,resources.limit(Limit::PageSize),0)});
        let body = q_wire::reply(
            &request.envelope,
            output,
            &session,
            &incarnation,
            epoch,
            resources,
            job,
        )?;
        return Ok(Reply::new(body, None, resources));
    }
    // 所有拟交付候选先过完整Delta三门，不能用回复装不下掩盖后批原子超限。
    let mut candidates = Vec::new();
    for cut in cuts
        .iter()
        .filter(|c| c.metadata.generation > request.after)
        .take(request.maximum as usize)
    {
        q_delta::measure(source, &cut.delta, resources, job)?;
        let wire = delta_wire(source, cut, job)?;
        candidates.push((
            wire,
            cursor_at(&current, cuts, cut.metadata.generation)?,
            cut.metadata.generation,
        ));
    }
    let mut accepted = None;
    let mut through = request.after;
    if candidates.is_empty() {
        accepted = Some(q_wire::reply(
            &request.envelope,
            output,
            &session,
            &incarnation,
            epoch,
            resources,
            job,
        )?);
    } else {
        for (wire, cursor, generation) in candidates {
            output["deltas"].as_array_mut().unwrap().push(wire);
            output["next_cursor"] = cursor;
            output["has_more"] = json!(generation < head);
            match q_wire::try_reply(
                &request.envelope,
                output.clone(),
                &session,
                &incarnation,
                epoch,
                resources,
                job,
            )? {
                Some(body) => {
                    accepted = Some(body);
                    through = generation;
                }
                None if accepted.is_none() => {
                    return Err(err("Incomplete：首批完整Watch消息体超过max_reply_bytes"))
                }
                None => break,
            }
        }
    }
    job.pre_io_check().map_err(err)?;
    Ok(Reply::new(
        accepted.ok_or_else(|| err("无完整Watch回复"))?,
        Some(Delivery {
            client: request.client.clone(),
            session,
            incarnation,
            after: through,
        }),
        resources,
    ))
}
fn delta_wire(
    source: &mut dyn NodeSource,
    cut: &PublishedCut,
    job: &mut Job,
) -> Result<Value, String> {
    let mut source = JobNodeSource::new(job, source);
    let mut wire = json!({});
    for key in [
        "generation",
        "session_id",
        "catalog_revision",
        "base_cut",
        "next_cut",
        "seq_range",
        "index_frontier",
        "input_frontier",
        "catalog_run_status",
        "catalog_evidence",
    ] {
        let field =
            lookup_path(&mut source, &cut.delta, &[PathStep::Field(key.into())]).map_err(err)?;
        wire[key] = if key == "catalog_evidence" {
            field.to_transport_json()
        } else {
            let mut bytes = Vec::new();
            sc::render_canonical(&mut source, &field, &mut bytes).map_err(err)?;
            strict_json(&bytes)?
        };
    }
    wire["delta"] = cut.delta.to_transport_json();
    Ok(wire)
}
struct Delivery {
    client: String,
    session: String,
    incarnation: String,
    after: i64,
}
pub(super) struct Reply {
    body: Vec<u8>,
    delivery: Option<Delivery>,
    resource_digest: String,
    write_ms: u64,
}
impl Reply {
    fn new(body: Vec<u8>, delivery: Option<Delivery>, resources: &QResources) -> Self {
        Self {
            body,
            delivery,
            resource_digest: resources.digest().into(),
            write_ms: resources.limit(Limit::SocketWriteMs),
        }
    }
    pub(super) fn body(&self) -> &[u8] {
        &self.body
    }
    /// 唯一送达入口：实际写入量逐次计账；完整body及flush成功且未过期才推进。
    pub(super) fn send_body<W: TimedWrite>(
        self,
        writer: &mut W,
        leases: &ClientLeases,
        job: &mut Job,
    ) -> Result<(), String> {
        if leases.resource_digest() != self.resource_digest {
            return Err(err("送达租约与编码时资源不同"));
        }
        let local = Instant::now()
            .checked_add(Duration::from_millis(self.write_ms))
            .ok_or_else(|| err("写期限溢出"))?;
        let deadline = local.min(job.first_read_deadline().map_err(err)?);
        let mut offset = 0;
        while offset < self.body.len() {
            job.pre_io_check().map_err(err)?;
            set_timeout(writer, deadline)?;
            let end = (offset + 65536).min(self.body.len());
            match writer.write(&self.body[offset..end]) {
                Ok(0) => return Err(err("WriteZero：回复未完整写出")),
                Ok(n) => {
                    job.charge_wire_bytes(n as u64).map_err(err)?;
                    offset += n;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(err(e)),
            }
        }
        job.pre_io_check().map_err(err)?;
        set_timeout(writer, deadline)?;
        writer.flush().map_err(err)?;
        job.pre_io_check().map_err(err)?;
        if Instant::now() >= deadline {
            return Err(err("Incomplete：回复写入绝对期限已过"));
        }
        if let Some(delivery) = self.delivery {
            leases.delivered(
                &delivery.client,
                (&delivery.session, &delivery.incarnation),
                delivery.after,
            )?;
        }
        Ok(())
    }
}
pub(super) trait TimedWrite: Write {
    fn set_timeout(&mut self, timeout: Duration) -> io::Result<()>;
}
impl TimedWrite for std::net::TcpStream {
    fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(timeout))
    }
}
#[cfg(unix)]
impl TimedWrite for std::os::unix::net::UnixStream {
    fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(timeout))
    }
}
fn set_timeout<W: TimedWrite>(writer: &mut W, deadline: Instant) -> Result<(), String> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(err("Incomplete：回复写入绝对期限已过"));
    }
    writer.set_timeout(left).map_err(err)
}
fn exact(value: &Value, fields: &[&str]) -> Result<(), String> {
    if value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k.as_str())))
    {
        Ok(())
    } else {
        Err(err("exact字段不符"))
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QWatchInvalid：{e}")
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{
        capture::capture_configured, prepared::tests::Fixture, q_content, q_resources,
    };
    use super::*;
    use newchan_rust::{
        s_content::job::{Existing, JobLimits},
        session_protocol::{canonical_json, sha256_hex},
    };
    use std::{
        io::Read,
        net::Shutdown,
        os::unix::net::UnixStream,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        },
    };
    fn limits() -> JobLimits {
        JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"16777216","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"100","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap()
    }
    fn job() -> Job {
        Job::new(
            limits(),
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn cursor(f: &Fixture, r: &QResources, g: i64) -> Value {
        let p = capture_configured(&f.db_path(), r, &mut job())
            .unwrap()
            .projection(
                g,
                super::super::projection::HistoryMode::AsKnown,
                &mut job(),
            )
            .unwrap();
        let c = &p.wire["fixed_cut"];
        json!({"session_id":c["session_id"],"session_generation":c["session_generation"],"catalog_revision":c["catalog_revision"],"scope":c["scope"],
            "after_generation":g.to_string(),"base_cut":c["structure_cut"],"index_frontier":c["index_frontier"],"last_seq":c["input_frontier"],"order_version":p.wire["order_version"]})
    }
    fn envelope(identity: &Value, payload: Value) -> Value {
        json!({"schema_revision":"s-session/3","session_id":identity["session_id"],"session_generation":identity["session_generation"],
        "source_namespace":"test.watch","source_epoch":"1","message_id":"watch-1","producer_id":"watch-test","producer_epoch":"1", "payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }
    fn request(cursor: &Value, n: u64, client: &str, r: &QResources) -> Request {
        Request::parse(canonical_json(&envelope(cursor,json!({"op":"watch","cursor":cursor,"max_batches":n.to_string(),"client_id":client}))).as_bytes(),r).unwrap()
    }
    fn prepare(
        f: &Fixture,
        r: &QResources,
        leases: &ClientLeases,
        cursor: &Value,
        n: u64,
        client: &str,
        job: &mut Job,
    ) -> Result<Reply, String> {
        capture_configured(&f.db_path(), r, job)?.watch(
            &request(cursor, n, client, r),
            leases,
            "4",
            job,
        )
    }
    fn body(reply: &Reply) -> Value {
        let v = strict_json(reply.body()).unwrap();
        assert_eq!(reply.body(), canonical_json(&v).as_bytes());
        assert_eq!(v.as_object().unwrap().len(), 11);
        assert_eq!(v["payload"].as_object().unwrap().len(), 8);
        v
    }
    fn send(reply: Reply, leases: &ClientLeases, job: &mut Job) -> Vec<u8> {
        let (mut out, mut input) = UnixStream::pair().unwrap();
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            input.read_to_end(&mut bytes).unwrap();
            bytes
        });
        reply.send_body(&mut out, leases, job).unwrap();
        out.shutdown(Shutdown::Write).unwrap();
        reader.join().unwrap()
    }
    #[test]
    fn real_batches_bind_cursor_and_content_and_delivery_follows_actual_complete_write() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let r = q_resources::tests::resources();
        let c = cursor(&f, &r, 0);
        let mut leases = ClientLeases::new(&r);
        let mut j = job();
        let reply = prepare(&f, &r, &mut leases, &c, 1, "reader", &mut j).unwrap();
        let response = body(&reply);
        let p = &response["payload"];
        assert_eq!(p["deltas"].as_array().unwrap().len(), 1);
        assert_eq!(p["next_cursor"], cursor(&f, &r, 1));
        assert_eq!(p["has_more"], true);
        assert!(p["gap"].is_null());
        assert_eq!(leases.delivered_generation("reader"), Some(0));
        assert_eq!(j.snapshot().wire_bytes, 0);
        let d = &p["deltas"][0];
        assert_eq!(d.as_object().unwrap().len(), 11);
        assert_eq!(d["delta"]["format"], "s-content/1");
        assert_eq!(d["catalog_evidence"]["format"], "s-content/1");
        let anchor = json!({"kind":"delta","session_id":d["session_id"],"session_generation":c["session_generation"],"generation":d["generation"],"base_cut":d["base_cut"],"next_cut":d["next_cut"],"index_frontier":d["index_frontier"],"delta_ref":d["delta"]});
        let content = envelope(
            &c,
            json!({"op":"content","anchor":anchor,"path":[{"field":"catalog_evidence"}],"unit":"items","offset":"0","limit":"1"}),
        );
        let request = q_content::Request::parse(canonical_json(&content).as_bytes(), &r).unwrap();
        let bytes = capture_configured(&f.db_path(), &r, &mut job())
            .unwrap()
            .content(&request, "4", &mut job())
            .unwrap();
        assert_eq!(
            strict_json(&bytes).unwrap()["payload"]["target_ref"],
            d["catalog_evidence"]
        );
        let expected = reply.body().to_vec();
        let received = send(reply, &mut leases, &mut j);
        assert_eq!(received, expected);
        assert_eq!(j.snapshot().wire_bytes, received.len() as u128);
        assert_eq!(leases.delivered_generation("reader"), Some(1));
        let next = prepare(
            &f,
            &r,
            &mut leases,
            &p["next_cursor"],
            1,
            "reader",
            &mut job(),
        )
        .unwrap();
        let p = &body(&next)["payload"];
        assert_eq!(p["next_cursor"], cursor(&f, &r, 2));
        assert_eq!(p["has_more"], false);
    }
    #[test]
    fn three_gap_reasons_preserve_cursor_and_rebuild_with_new_snapshot_token() {
        let mut f = Fixture::with_retain(1);
        publish(&mut f, 1);
        publish(&mut f, 2);
        let r = q_resources::tests::resources();
        let old = cursor(&f, &r, 0);
        let mut leases = ClientLeases::new(&r);
        let mut foreign = old.clone();
        foreign["session_id"] = json!("old-session");
        for (c, reason, missing) in [
            (&foreign, "session_identity_changed", Value::Null),
            (
                &old,
                "delivery_retention_gap",
                json!({"from_generation":"1","to_generation":"1"}),
            ),
        ] {
            let reply = prepare(&f, &r, &mut leases, c, 1, "gap", &mut job()).unwrap();
            let response = body(&reply);
            let p = &response["payload"];
            assert_eq!(p["next_cursor"], *c);
            assert_eq!(p["deltas"], json!([]));
            assert_eq!(p["gap"]["reason"], reason);
            assert_eq!(p["gap"]["missing_range"], missing);
            assert_eq!(p["gap"].as_object().unwrap().len(), 5);
            let snap = envelope(
                &p["observed_head"],
                json!({"op":"snapshot","snapshot_token":p["gap"]["snapshot_token"]}),
            );
            let req = q_snapshot::Request::parse(canonical_json(&snap).as_bytes(), &r).unwrap();
            let snap = capture_configured(&f.db_path(), &r, &mut job())
                .unwrap()
                .snapshot(&req, "4", &mut job())
                .unwrap();
            assert_eq!(
                strict_json(&snap).unwrap()["payload"]["fixed_cut"],
                p["gap"]["rebuild"]
            );
            send(reply, &mut leases, &mut job());
            assert_eq!(leases.delivered_generation("gap"), None);
        }
        let mut f = Fixture::new();
        let mut config = q_resources::tests::input();
        config["limits"]["max_client_pending_batches"] = json!("1");
        let r = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        let mut leases = ClientLeases::new(&r);
        let c = cursor(&f, &r, 0);
        let empty = prepare(&f, &r, &mut leases, &c, 1, "slow", &mut job()).unwrap();
        assert!(body(&empty)["payload"]["gap"].is_null());
        publish(&mut f, 1);
        publish(&mut f, 2);
        let gap = prepare(&f, &r, &mut leases, &c, 1, "slow", &mut job()).unwrap();
        let p = &body(&gap)["payload"];
        assert_eq!(p["gap"]["reason"], "client_backlog_overflow");
        assert_eq!(
            p["gap"]["missing_range"],
            json!({"from_generation":"1","to_generation":"2"})
        );
        assert_eq!(p["next_cursor"], c);
        send(gap, &mut leases, &mut job());
        assert_eq!(leases.delivered_generation("slow"), Some(0));
        let rebuilt = cursor(&f, &r, 2);
        let resumed = prepare(&f, &r, &mut leases, &rebuilt, 1, "slow", &mut job()).unwrap();
        assert!(body(&resumed)["payload"]["gap"].is_null());
        assert_eq!(
            leases.delivered_generation("slow"),
            Some(2),
            "原合同允许已核完整重建cursor重置租约"
        );
    }
    #[test]
    fn full_reply_truncation_keeps_whole_batches_but_cannot_hide_later_atomic_failure() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let r = q_resources::tests::resources();
        let c = cursor(&f, &r, 0);
        let one = prepare(&f, &r, &mut ClientLeases::new(&r), &c, 1, "r", &mut job()).unwrap();
        let cap = one.body().len();
        let two = prepare(&f, &r, &mut ClientLeases::new(&r), &c, 2, "r", &mut job()).unwrap();
        assert!(two.body().len() > cap);
        let mut config = q_resources::tests::input();
        config["limits"]["max_reply_bytes"] = json!(cap.to_string());
        let tight = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        let mut leases = ClientLeases::new(&tight);
        let shortened = prepare(&f, &tight, &mut leases, &c, 2, "r", &mut job()).unwrap();
        let p = &body(&shortened)["payload"];
        assert_eq!(p["deltas"].as_array().unwrap().len(), 1);
        assert_eq!(p["has_more"], true);
        assert_eq!(p["next_cursor"], cursor(&f, &tight, 1));
        assert_eq!(leases.delivered_generation("r"), Some(0));
        config["limits"]["max_reply_bytes"] = json!((cap - 1).to_string());
        let short = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(prepare(
            &f,
            &short,
            &mut ClientLeases::new(&short),
            &c,
            1,
            "r",
            &mut job()
        )
        .err()
        .unwrap()
        .contains("max_reply_bytes"));
        let usage = capture_configured(&f.db_path(), &r, &mut job())
            .unwrap()
            .delta_references(0, 2, &mut job())
            .unwrap()
            .usage;
        assert!(usage[1].nodes > usage[0].nodes);
        config["limits"]["max_reply_bytes"] = json!(cap.to_string());
        config["limits"]["max_atomic_delta_nodes"] = json!(usage[0].nodes.to_string());
        let atomic = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        let mut leases = ClientLeases::new(&atomic);
        assert!(prepare(&f, &atomic, &mut leases, &c, 2, "r", &mut job())
            .err()
            .unwrap()
            .contains("max_atomic_delta_nodes"));
        assert_eq!(leases.delivered_generation("r"), Some(0));
    }
    struct FaultStream {
        inner: UnixStream,
        left: Option<usize>,
        flush_fail: bool,
        expire: Option<Arc<AtomicU64>>,
    }
    impl Write for FaultStream {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.left == Some(0) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "test after real prefix",
                ));
            }
            let n = self
                .inner
                .write(&b[..b.len().min(self.left.unwrap_or(usize::MAX))])?;
            if let Some(left) = &mut self.left {
                *left -= n;
            }
            if let Some(expire) = &self.expire {
                expire.store(20000, Ordering::SeqCst);
            }
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.flush_fail {
                Err(io::Error::other("test flush failure"))
            } else {
                self.inner.flush()
            }
        }
    }
    impl TimedWrite for FaultStream {
        fn set_timeout(&mut self, d: Duration) -> io::Result<()> {
            self.inner.set_write_timeout(Some(d))
        }
    }
    #[test]
    fn partial_write_flush_failure_and_expired_job_preserve_lease_and_actual_wire_ledger() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::resources();
        let c = cursor(&f, &r, 0);
        for case in 0..3 {
            let tick = Arc::new(AtomicU64::new(0));
            let clock = tick.clone();
            let base = Instant::now();
            let mut j = Job::new(
                limits(),
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(move || base + Duration::from_millis(clock.load(Ordering::SeqCst))),
            )
            .unwrap();
            let mut leases = ClientLeases::new(&r);
            let reply = prepare(&f, &r, &mut leases, &c, 1, "r", &mut j).unwrap();
            let full = reply.body().len();
            let (out, mut input) = UnixStream::pair().unwrap();
            let reader = std::thread::spawn(move || {
                let mut b = Vec::new();
                input.read_to_end(&mut b).unwrap();
                b
            });
            let mut writer = FaultStream {
                inner: out,
                left: if case == 0 { Some(12) } else { None },
                flush_fail: case == 1,
                expire: if case == 2 { Some(tick) } else { None },
            };
            assert!(reply.send_body(&mut writer, &mut leases, &mut j).is_err());
            writer.inner.shutdown(Shutdown::Write).unwrap();
            let actual = reader.join().unwrap();
            assert_eq!(leases.delivered_generation("r"), Some(0));
            assert_eq!(j.snapshot().wire_bytes, actual.len() as u128);
            if case == 0 {
                assert_eq!(actual.len(), 12);
            } else {
                assert_eq!(actual.len(), full);
            }
        }
    }
    #[test]
    fn forged_cursor_and_outer_identity_are_rejected_before_offer() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::resources();
        let c = cursor(&f, &r, 0);
        let mut leases = ClientLeases::new(&r);
        for (key, value) in [
            ("base_cut", json!("cut-1")),
            ("last_seq", json!("0")),
            ("index_frontier", json!("fake")),
            ("catalog_revision", json!("fake")),
            ("scope", json!({})),
            ("order_version", json!("s-record-order/1")),
            ("after_generation", json!("2")),
        ] {
            let mut forged = c.clone();
            forged[key] = value;
            assert!(
                prepare(&f, &r, &mut leases, &forged, 1, "r", &mut job()).is_err(),
                "{key}"
            );
        }
        let mut e = envelope(
            &c,
            json!({"op":"watch","cursor":c,"max_batches":"1","client_id":"r"}),
        );
        e["session_id"] = json!("mismatched");
        assert!(Request::parse(canonical_json(&e).as_bytes(), &r).is_err());
        assert_eq!(leases.delivered_generation("r"), None);
    }
    #[test]
    fn lease_trace_matches_original_python_for_backlog_rebuild_late_delivery_and_expiry() {
        use std::{path::Path, process::Command};
        let mut config = q_resources::tests::input();
        config["limits"]["max_client_leases"] = json!("2");
        config["limits"]["client_lease_ms"] = json!("10");
        config["limits"]["max_client_pending_batches"] = json!("2");
        let r = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
        let leases = ClientLeases::new(&r);
        // op,client,session,incarnation,after,head,monotonic nanoseconds。
        let trace = json!([
            ["o", "a", "s", "1", 0, 0, 0],
            ["o", "b", "s", "1", 0, 2, 0],
            ["o", "a", "s", "1", 0, 3, 0],
            ["d", "a", "s", "1", 1, 0, 0],
            ["o", "a", "s", "1", 1, 3, 0],
            ["o", "a", "s", "1", 3, 3, 1000000],
            ["d", "b", "s", "1", 5, 0, 1000000],
            ["o", "b", "s", "1", 0, 3, 1000000],
            ["o", "c", "s", "2", 0, 0, 1000000],
            ["o", "a", "s", "2", 0, 0, 1000000],
            ["o", "c", "s", "1", 0, 0, 11000000],
            ["o", "c", "s", "1", 0, 0, 11000001],
            ["d", "missing", "s", "1", 7, 0, 11000001]
        ]);
        let base = Instant::now();
        let mut actual = Vec::new();
        for row in trace.as_array().unwrap() {
            let client = row[1].as_str().unwrap();
            let identity = (row[2].as_str().unwrap(), row[3].as_str().unwrap());
            let after = row[4].as_i64().unwrap();
            let outcome = if row[0] == "o" {
                leases
                    .observe(
                        client,
                        identity,
                        after,
                        row[5].as_i64().unwrap(),
                        base + Duration::from_nanos(row[6].as_u64().unwrap()),
                    )
                    .map(|overflow| if overflow { "overflow" } else { "ok" })
            } else {
                leases.delivered(client, identity, after).map(|_| "sent")
            };
            actual.push(json!({"result":outcome.unwrap_or("error"),"delivered":(["a","b","c","missing"].map(|c|leases.delivered_generation(c)))}));
        }
        let script = r#"import sys,json,types
sys.path.insert(0,sys.argv[1]);import s_query as q
now=[0];q.time.monotonic=lambda:now[0]/1_000_000_000
leases=q.ClientLeases(dict(max_client_leases=2,client_lease_ms=10,max_client_pending_batches=2))
audit=types.SimpleNamespace(QueryBudgetExceeded=RuntimeError);out=[]
for op,client,s,inc,after,head,t in json.loads(sys.argv[2]):
 now[0]=t;cursor=dict(session_id=s,session_generation=inc,after_generation=str(after))
 try:
  if op=='o':
   proof=dict(meta=dict(session_id=s),protocol=dict(session_generation=inc),generation=head)
   result='overflow' if leases.observe(client,cursor,proof,audit) else 'ok'
  else: leases.delivered(client,cursor);result='sent'
 except RuntimeError: result='error'
 out.append(dict(result=result,delivered=[leases.clients.get(c,{}).get('delivered') for c in ['a','b','c','missing']]))
print(json.dumps(out,sort_keys=True,separators=(',',':')))"#;
        let oracle = Command::new("python3")
            .args(["-c", script])
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("s_session"),
            )
            .arg(canonical_json(&trace))
            .output()
            .unwrap();
        assert!(
            oracle.status.success(),
            "{}",
            String::from_utf8_lossy(&oracle.stderr)
        );
        assert_eq!(Value::Array(actual), strict_json(&oracle.stdout).unwrap());
    }
}
