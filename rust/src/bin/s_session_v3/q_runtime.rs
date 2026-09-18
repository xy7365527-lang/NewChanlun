//! #1394 C §5.5/5.6.1：仅由Q21配置构造的查询运行时。
//! 三条v3路由共用现有审计/分页/内容/Watch判据；不创建八字段消费者Job。
//! 已接受的TCP连接先占槽，再读有界HTTP请求；本模块不自行bind/监听或加载部署默认值。
use super::{
    capture::{AuditReader, ConfiguredCapture},
    q_content,
    q_executor::{Executor, Pending},
    q_http_input,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
    q_snapshot, q_watch, q_wire,
};
use newchan_rust::{
    s_content::{access::Access, job::Job},
    session_protocol::positive,
};
use serde_json::{json, Value};
use std::{
    io,
    net::TcpStream,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

/// 可信Q调用层没有消费者累计账。Q的各phase、单包、Delta、worker/cache/lease门
/// 仍由冻结resources及各实际调用点执行；不能把None称作无限Job。
struct ServerAccess<'a> {
    _resources: &'a QResources,
}
impl<'a> ServerAccess<'a> {
    fn new(resources: &'a QResources) -> Self {
        Self {
            _resources: resources,
        }
    }
}
impl Access for ServerAccess<'_> {
    fn owner_job(&mut self) -> Option<&mut Job> {
        None
    }
}

enum Request {
    Snapshot(q_snapshot::Request),
    Content(q_content::Request),
    Watch(q_watch::Request),
}
pub(super) struct Runtime {
    executor: Executor,
    epoch: String,
    control_instance_id: String,
}
impl Runtime {
    pub(super) fn new(
        path: PathBuf,
        resources: Arc<QResources>,
        epoch: &str,
    ) -> Result<Self, String> {
        positive(epoch, "Q producer_epoch")?;
        Ok(Self {
            executor: Executor::new(path, resources)?,
            epoch: epoch.into(),
            control_instance_id: String::new(),
        })
    }
    pub(super) fn with_control_instance_id(mut self, id: String) -> Self {
        self.control_instance_id = id;
        self
    }
    #[cfg(test)]
    pub(super) fn resource_digest(&self) -> &str {
        self.executor.resources().digest()
    }
    /// 连接尚未读取时即交给固定worker；接纳失败仍由本调用发送有界503并关闭连接。
    /// 本口消费连接所有权；返回Err只供记录，调用者不得再发送第二份HTTP响应。
    pub(super) fn accept_http(&self, connection: TcpStream) -> Result<Pending<()>, String> {
        let epoch = self.epoch.clone();
        let control_instance_id = self.control_instance_id.clone();
        match self.executor.try_submit_owned(
            connection,
            move |mut connection, reader, resources, leases, _queued_at| {
                serve_http(
                    &mut connection,
                    reader,
                    resources,
                    leases,
                    &epoch,
                    &control_instance_id,
                )
            },
        ) {
            Ok(pending) => Ok(pending),
            Err((reason, mut connection)) => {
                let resources = self.executor.resources();
                let body = error_body(
                    resources,
                    &self.epoch,
                    None,
                    None,
                    json!({"ok":false,"status":"Incomplete","error":"QueryBudgetExceeded"}),
                )?;
                let reply = q_watch::Reply::plain(body, resources)?;
                send_http(
                    &mut connection,
                    reply,
                    503,
                    resources,
                    self.executor.leases(),
                )?;
                Err(reason)
            }
        }
    }
    #[cfg(test)]
    pub(super) fn dispatch(
        &self,
        path: &str,
        body: &[u8],
    ) -> Result<Pending<q_watch::Reply>, String> {
        let request = parse_request(path, body, self.executor.resources())?;
        let epoch = self.epoch.clone();
        self.executor
            .try_submit(move |reader, resources, leases, _queued_at| {
                execute(reader, resources, leases, &epoch, request)
            })
    }
    /// 网络适配使用本口：完整body写出或失败前一直占用该worker和active+queued槽位。
    /// 请求body的HTTP读取/头处理仍属上层；本口不创建监听器。
    #[cfg(test)]
    pub(super) fn dispatch_to<W: q_watch::TimedWrite + Send + 'static>(
        &self,
        path: &str,
        body: &[u8],
        mut writer: W,
    ) -> Result<Pending<()>, String> {
        let request = parse_request(path, body, self.executor.resources())?;
        let epoch = self.epoch.clone();
        self.executor
            .try_submit(move |reader, resources, leases, _queued_at| {
                let reply = execute(reader, resources, leases, &epoch, request)?;
                reply.send_body(&mut writer, leases, &mut ServerAccess::new(resources))
            })
    }

    /// 实际消息体写出仍取原socket_write_timeout；Watch只在完整送达后推进。
    /// HTTP头不在此接口；消费者若在本地调用并要累计自身wire，应另走其Job适配。
    #[cfg(test)]
    pub(super) fn send<W: q_watch::TimedWrite>(
        &self,
        reply: q_watch::Reply,
        writer: &mut W,
    ) -> Result<(), String> {
        let r = self.executor.resources();
        if reply.body().len() as u128 > u128::from(r.limit(Limit::ReplyBytes)) {
            return Err("Incomplete：完整Q回复超过max_reply_bytes".into());
        }
        reply.send_body(writer, self.executor.leases(), &mut ServerAccess::new(r))
    }
}

fn parse_request(path: &str, body: &[u8], resources: &QResources) -> Result<Request, String> {
    // 每个解析器先核完整frame和exact envelope/payload。无资源配置来自请求。
    Ok(match path {
        "/api/v3/snapshot" => Request::Snapshot(q_snapshot::Request::parse(body, resources)?),
        "/api/v3/content" => Request::Content(q_content::Request::parse(body, resources)?),
        "/api/v3/watch" => Request::Watch(q_watch::Request::parse(body, resources)?),
        _ => return Err("QRouteInvalid：不是已声明的v3查询路由".into()),
    })
}
#[cfg(test)]
fn execute(
    reader: &mut AuditReader,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Request,
) -> Result<q_watch::Reply, String> {
    reader.with_capture(
        resources,
        &mut ServerAccess::new(resources),
        |capture, access| execute_captured(capture, resources, leases, epoch, request, access),
    )
}
fn execute_captured(
    capture: &mut ConfiguredCapture<'_>,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Request,
    access: &mut dyn Access,
) -> Result<q_watch::Reply, String> {
    match request {
        Request::Snapshot(request) => {
            q_watch::Reply::plain(capture.snapshot(&request, epoch, access)?, resources)
        }
        Request::Content(request) => {
            q_watch::Reply::plain(capture.content(&request, epoch, access)?, resources)
        }
        Request::Watch(request) => capture.watch(&request, leases, epoch, access),
    }
}

fn serve_http<S: q_http_input::TimedRead + q_watch::TimedWrite>(
    stream: &mut S,
    reader: &mut AuditReader,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    control_instance_id: &str,
) -> Result<(), String> {
    let mut input = match q_http_input::Input::new(stream, resources) {
        Ok(input) => input,
        Err(failure) => return send_failure(stream, resources, leases, epoch, None, None, failure),
    };
    let head = match input.head() {
        Ok(head) => head,
        Err(failure) => return send_failure(stream, resources, leases, epoch, None, None, failure),
    };
    let (get_path, query) = head
        .path
        .split_once('?')
        .map_or((head.path.as_str(), ""), |(path, query)| (path, query));
    if head.method == "GET" {
        if query.split('&').any(|part| !part.is_empty()) {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                q_http_input::Failure {
                    status: 400,
                    error: "InvalidQuery",
                    detail: "就绪控制读口不接受查询参数".into(),
                },
            );
        }
        if get_path != "/api/ready" {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                q_http_input::Failure {
                    status: 404,
                    error: "not found",
                    detail: "未知查询路径".into(),
                },
            );
        }
        let result = reader.with_capture(
            resources,
            &mut ServerAccess::new(resources),
            |capture, _| {
                let payload = json!({
                    "ok": true,
                    "cut": capture.ready_cut()?,
                    "producer_epoch": epoch,
                    "control_instance_id": control_instance_id,
                });
                let mut access = ServerAccess::new(resources);
                let body =
                    q_wire::encode(&payload, resources.limit(Limit::ReplyBytes), &mut access)?
                        .ok_or_else(|| "Incomplete：ready回复超过max_reply_bytes".to_string())?;
                q_watch::Reply::plain(body, resources)
            },
        );
        return match result {
            Ok(reply) => send_http(stream, reply, 200, resources, leases),
            Err(detail) => send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure(detail, true),
            ),
        };
    }
    if head.method != "POST" {
        return send_failure(
            stream,
            resources,
            leases,
            epoch,
            None,
            None,
            q_http_input::Failure {
                status: 501,
                error: "UnsupportedMethod",
                detail: "v3查询要求POST".into(),
            },
        );
    }
    // POST沿用旧HTTP路由的完整路径匹配；查询串不能被静默剥掉后再放行。
    let path = head.path.as_str();
    if !["/api/v3/snapshot", "/api/v3/content", "/api/v3/watch"].contains(&path) {
        return send_failure(
            stream,
            resources,
            leases,
            epoch,
            None,
            None,
            q_http_input::Failure {
                status: 404,
                error: "not found",
                detail: "未知查询路径".into(),
            },
        );
    }
    let body = match input.body(&head) {
        Ok(body) => body,
        Err(failure) => return send_failure(stream, resources, leases, epoch, None, None, failure),
    };
    let path = path.to_string();
    let envelope = match q_wire::parse(&body, resources) {
        Ok(envelope) => envelope,
        Err(detail) => {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure(detail, false),
            )
        }
    };
    let request = match parse_request(&path, &body, resources) {
        Ok(request) => request,
        Err(detail) => {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                Some(&envelope),
                None,
                failure(detail, false),
            )
        }
    };
    let mut identity = None;
    let result = reader.with_capture(
        resources,
        &mut ServerAccess::new(resources),
        |capture, access| {
            identity = Some(capture.identity()?);
            execute_captured(capture, resources, leases, epoch, request, access)
        },
    );
    match result {
        Ok(reply) => send_http(stream, reply, 200, resources, leases),
        Err(detail) => send_failure(
            stream,
            resources,
            leases,
            epoch,
            Some(&envelope),
            identity.as_ref(),
            failure(detail, identity.is_none()),
        ),
    }
}

/// 这里只选择HTTP错误表示；成功/拒绝的业务判据仍全部由原Q层执行。
fn failure(detail: String, capture_failed: bool) -> q_http_input::Failure {
    let budget = [
        "Incomplete",
        "CaptureDeadline",
        "VerifyDeadline",
        "CaptureBytes",
        "atomic_delta_limit",
    ]
    .iter()
    .any(|code| detail.contains(code));
    let (status, error) = if budget {
        (503, "QueryBudgetExceeded")
    } else if capture_failed {
        (503, "StorageUnavailable")
    } else {
        (400, "InvalidQuery")
    };
    q_http_input::Failure {
        status,
        error,
        detail,
    }
}

fn send_failure<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Option<&Value>,
    identity: Option<&(String, String)>,
    failure: q_http_input::Failure,
) -> Result<(), String> {
    let mut payload = json!({"ok":false,"error":failure.error,"detail":failure.detail});
    if failure.error == "QueryBudgetExceeded" {
        payload["status"] = json!("Incomplete");
    }
    let body = error_body(resources, epoch, request, identity, payload)?;
    send_http(
        stream,
        q_watch::Reply::plain(body, resources)?,
        failure.status,
        resources,
        leases,
    )
}

fn error_body(
    resources: &QResources,
    epoch: &str,
    request: Option<&Value>,
    identity: Option<&(String, String)>,
    mut payload: Value,
) -> Result<Vec<u8>, String> {
    let mut access = ServerAccess::new(resources);
    if let Some(request) = request {
        payload["identity_binding"] = json!(if identity.is_some() {
            "verified_current_capture"
        } else {
            "request_echo_unverified"
        });
        let (session, incarnation) = identity
            .map(|(session, incarnation)| (session.as_str(), incarnation.as_str()))
            .unwrap_or((
                request["session_id"].as_str().ok_or("已核请求缺session")?,
                request["session_generation"]
                    .as_str()
                    .ok_or("已核请求缺化身")?,
            ));
        q_wire::reply(
            request,
            payload,
            session,
            incarnation,
            epoch,
            resources,
            &mut access,
        )
    } else {
        q_wire::encode(&payload, resources.limit(Limit::ReplyBytes), &mut access)?
            .ok_or_else(|| "Incomplete：错误消息体超过max_reply_bytes，关闭连接".into())
    }
}

fn send_http<W: q_watch::TimedWrite>(
    writer: &mut W,
    reply: q_watch::Reply,
    status: u16,
    resources: &QResources,
    leases: &ClientLeases,
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        414 => "Request-URI Too Long",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => return Err("Q HTTP状态码未声明".into()),
    };
    let header = format!("HTTP/1.0 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", reply.body().len());
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(resources.limit(Limit::SocketWriteMs)))
        .ok_or("Q HTTP写期限溢出")?;
    let mut offset = 0;
    while offset < header.len() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("Incomplete：HTTP头写期限已过")?;
        writer.set_timeout(remaining).map_err(|e| e.to_string())?;
        match writer.write(&header.as_bytes()[offset..]) {
            Ok(0) => return Err("HTTP头WriteZero".into()),
            Ok(n) => offset += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("HTTP头写入失败：{e}")),
        }
    }
    if Instant::now() >= deadline {
        return Err("Incomplete：HTTP响应头写入超过本阶段期限".into());
    }
    // 原Python end_headers与body各调用一次sendall；保留各自同值超时，
    // 不把两次写操作改成更短的共享全响应期限。
    reply.send_body(writer, leases, &mut ServerAccess::new(resources))
}

#[cfg(test)]
mod tests {
    use super::super::{
        capture::AuditReader,
        prepared::tests::Fixture,
        projection::{HistoryMode, Projection},
        q_resources,
    };
    use super::*;
    use newchan_rust::session_protocol::{canonical_json, sha256_hex, strict_json};
    use serde_json::{json, Value};
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
        time::Duration,
    };
    fn resources() -> Arc<QResources> {
        let mut v = q_resources::tests::input();
        v["limits"]["max_query_workers"] = json!("1");
        Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap())
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn projection(f: &Fixture, r: &QResources) -> Projection {
        let mut reader = AuditReader::new(f.db_path(), r);
        reader
            .with_capture(r, &mut ServerAccess::new(r), |c, a| {
                c.projection(1, HistoryMode::AsKnown, a)
            })
            .unwrap()
    }
    fn envelope(fixed: &Value, payload: Value) -> Vec<u8> {
        canonical_json(&json!({"schema_revision":"s-session/3","session_id":fixed["session_id"],"session_generation":fixed["session_generation"],"source_namespace":"q-runtime-test","source_epoch":"1","message_id":"request","producer_id":"test-client","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})).into_bytes()
    }
    fn snapshot(p: &Projection) -> Vec<u8> {
        let f = &p.wire["fixed_cut"];
        envelope(
            f,
            json!({"op":"snapshot","session_id":f["session_id"],"session_generation":f["session_generation"],"scope":f["scope"],"history_mode":"AsKnown","as_of_generation":"1","page_size":"1","order_version":p.wire["order_version"]}),
        )
    }
    fn watch(p: &Projection) -> Vec<u8> {
        let f = &p.wire["fixed_cut"];
        envelope(
            f,
            json!({"op":"watch","cursor":{"session_id":f["session_id"],"session_generation":f["session_generation"],"catalog_revision":f["catalog_revision"],"scope":f["scope"],"after_generation":"0","base_cut":"cut-0","index_frontier":"","last_seq":"-1","order_version":p.wire["order_version"]},"max_batches":"1","client_id":"runtime-client"}),
        )
    }
    fn call(rt: &Runtime, path: &str, body: &[u8]) -> q_watch::Reply {
        rt.dispatch(path, body).unwrap().recv().unwrap()
    }
    #[test]
    fn three_routes_use_only_frozen_q21_and_actual_worker_capture() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        assert_eq!(
            strict_json(r.canonical().as_bytes()).unwrap()["limits"]
                .as_object()
                .unwrap()
                .len(),
            21
        );
        let rt = Runtime::new(f.db_path(), Arc::clone(&r), "7").unwrap();
        assert_eq!(rt.resource_digest(), r.digest());
        let first = call(&rt, "/api/v3/snapshot", &snapshot(&p));
        let page = strict_json(first.body()).unwrap();
        assert_eq!(page["payload"]["fixed_cut"], p.wire["fixed_cut"]);
        assert_eq!(page["producer_epoch"], "7");
        let token = page["payload"]["next_token"].clone();
        assert!(token.is_object());
        publish(&mut f, 2);
        let next = call(
            &rt,
            "/api/v3/snapshot",
            &envelope(
                &p.wire["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":token}),
            ),
        );
        let next = strict_json(next.body()).unwrap();
        assert_eq!(next["payload"]["fixed_cut"], p.wire["fixed_cut"]);
        assert_ne!(
            next["payload"]["validated_capture_digest"],
            page["payload"]["validated_capture_digest"]
        );
        let content = envelope(
            &p.wire["fixed_cut"],
            json!({"op":"content","anchor":p.anchor(),"path":[{"field":"rows"}],"unit":"items","offset":"0","limit":"1"}),
        );
        let content = call(&rt, "/api/v3/content", &content);
        let content = strict_json(content.body()).unwrap();
        assert_eq!(content["payload"]["offset"], "0");
        assert_eq!(content["payload"]["end"], "1");
        assert!(content["payload"]["nodes"].as_array().unwrap().len() > 0);
        let response = call(&rt, "/api/v3/watch", &watch(&p));
        let body = response.body().to_vec();
        assert_eq!(
            strict_json(&body).unwrap()["payload"]["deltas"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(0)
        );
        let (writer, mut peer) = UnixStream::pair().unwrap();
        let receiver = std::thread::spawn(move || {
            let mut b = Vec::new();
            peer.read_to_end(&mut b).unwrap();
            b
        });
        drop(response);
        rt.dispatch_to("/api/v3/watch", &watch(&p), writer)
            .unwrap()
            .recv()
            .unwrap();
        assert_eq!(receiver.join().unwrap(), body);
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(1)
        );
    }
    #[test]
    fn server_resource_gates_and_exact_requests_do_not_depend_on_client_job_configuration() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        let incoming = snapshot(&p);
        let rt = Runtime::new(f.db_path(), Arc::clone(&r), "1").unwrap();
        assert!(rt.dispatch("/api/v2/snapshot", &incoming).is_err());
        let mut forged: Value = strict_json(&incoming).unwrap();
        forged["job_limits"] = json!({"max_total_nodes":"999999"});
        assert!(rt
            .dispatch("/api/v3/snapshot", canonical_json(&forged).as_bytes())
            .is_err());
        for (key, value, path, input) in [
            (
                "max_frame_bytes",
                (incoming.len() - 1).to_string(),
                "/api/v3/snapshot",
                incoming.clone(),
            ),
            (
                "max_reply_bytes",
                "128".into(),
                "/api/v3/snapshot",
                incoming.clone(),
            ),
            (
                "max_capture_bytes",
                "512".into(),
                "/api/v3/snapshot",
                incoming.clone(),
            ),
            (
                "max_atomic_delta_nodes",
                "1".into(),
                "/api/v3/watch",
                watch(&p),
            ),
            (
                "max_atomic_delta_node_bytes",
                "1".into(),
                "/api/v3/watch",
                watch(&p),
            ),
            (
                "max_atomic_delta_records",
                "1".into(),
                "/api/v3/watch",
                watch(&p),
            ),
        ] {
            let mut config = strict_json(r.canonical().as_bytes()).unwrap();
            config["limits"][key] = json!(value);
            let small = Arc::new(QResources::parse(canonical_json(&config).as_bytes()).unwrap());
            let runtime = Runtime::new(f.db_path(), small, "1").unwrap();
            let result = match runtime.dispatch(path, &input) {
                Ok(p) => p.recv(),
                Err(e) => Err(e),
            };
            assert!(result.is_err(), "{key}");
        }
    }
    #[test]
    fn runtime_write_failure_never_advances_lease() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        let rt = Runtime::new(f.db_path(), r, "1").unwrap();
        let response = call(&rt, "/api/v3/watch", &watch(&p));
        struct Reject;
        impl Write for Reject {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected write failure"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl q_watch::TimedWrite for Reject {
            fn set_timeout(&mut self, left: Duration) -> std::io::Result<()> {
                assert!(left > Duration::ZERO);
                Ok(())
            }
        }
        assert!(rt.send(response, &mut Reject).is_err());
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(0)
        );
    }
    #[test]
    fn response_write_holds_worker_slot_until_completion() {
        use std::sync::mpsc;
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let mut config = q_resources::tests::input();
        config["limits"]["max_query_workers"] = json!("1");
        config["limits"]["max_query_queue"] = json!("1");
        let r = Arc::new(QResources::parse(canonical_json(&config).as_bytes()).unwrap());
        let p = projection(&f, &r);
        let runtime = Runtime::new(f.db_path(), r, "1").unwrap();
        let (started_rx_send, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        struct Held {
            started: Option<mpsc::Sender<()>>,
            wait: mpsc::Receiver<()>,
        }
        impl Write for Held {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                if let Some(s) = self.started.take() {
                    s.send(()).unwrap();
                    self.wait.recv().unwrap();
                }
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl q_watch::TimedWrite for Held {
            fn set_timeout(&mut self, _: Duration) -> std::io::Result<()> {
                Ok(())
            }
        }
        let first = runtime
            .dispatch_to(
                "/api/v3/snapshot",
                &snapshot(&p),
                Held {
                    started: Some(started_rx_send),
                    wait,
                },
            )
            .unwrap();
        started.recv().unwrap();
        let queued = runtime.dispatch("/api/v3/snapshot", &snapshot(&p)).unwrap();
        assert!(runtime.dispatch("/api/v3/snapshot", &snapshot(&p)).is_err());
        release.send(()).unwrap();
        first.recv().unwrap();
        queued.recv().unwrap();
        call(&runtime, "/api/v3/snapshot", &snapshot(&p));
    }

    fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        (client, server)
    }
    fn http_request(path: &str, body: &[u8]) -> Vec<u8> {
        let mut wire = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        wire.extend_from_slice(body);
        wire
    }
    fn exchange(rt: &Runtime, wire: &[u8]) -> (Result<(), String>, Vec<u8>) {
        let (mut client, server) = tcp_pair();
        client.write_all(wire).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let pending = rt.accept_http(server);
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        let result = pending.and_then(|p| p.recv());
        (result, response)
    }
    fn http_parts(wire: &[u8]) -> (u16, Value) {
        let end = wire.windows(4).position(|v| v == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&wire[..end]).unwrap();
        assert!(head.contains("Cache-Control: no-store"));
        assert!(head.contains("Connection: close"));
        let length: usize = head
            .lines()
            .find_map(|s| s.strip_prefix("Content-Length: "))
            .unwrap()
            .parse()
            .unwrap();
        let body = &wire[end + 4..];
        assert_eq!(length, body.len());
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, strict_json(body).unwrap())
    }
    fn error_reply_matches_original_python(request: &[u8], reply: &Value, proof: Value) {
        let script = r#"import sys,json,types
sys.path.insert(0,sys.argv[1]);import s_query as q
canonical=lambda v:json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
e=json.loads(sys.argv[2]);p=json.loads(sys.argv[3]);proof=json.loads(sys.argv[4])
r=q.response_envelope(e,p,proof,2,types.SimpleNamespace(canonical=canonical));r['schema_revision']='s-session/3'
sys.stdout.buffer.write(canonical(r))"#;
        let oracle = std::process::Command::new("python3")
            .args(["-c", script])
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("s_session"),
            )
            .arg(std::str::from_utf8(request).unwrap())
            .arg(canonical_json(&reply["payload"]))
            .arg(canonical_json(&proof))
            .output()
            .unwrap();
        assert!(
            oracle.status.success(),
            "{}",
            String::from_utf8_lossy(&oracle.stderr)
        );
        assert_eq!(oracle.stdout, canonical_json(reply).as_bytes());
    }
    #[test]
    fn real_tcp_http_routes_share_actual_capture_and_watch_delivery() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        let rt = Runtime::new(f.db_path(), r, "1").unwrap();
        let content = envelope(
            &p.wire["fixed_cut"],
            json!({"op":"content","anchor":p.anchor(),"path":[{"field":"rows"}],"unit":"items","offset":"0","limit":"1"}),
        );
        for (path, body) in [
            ("/api/v3/snapshot", snapshot(&p)),
            ("/api/v3/content", content),
            ("/api/v3/watch", watch(&p)),
        ] {
            let expected = call(&rt, path, &body);
            let (result, wire) = exchange(&rt, &http_request(path, &body));
            result.unwrap();
            let (status, actual) = http_parts(&wire);
            assert_eq!(status, 200);
            assert_eq!(actual, strict_json(expected.body()).unwrap());
        }
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(1)
        );
    }
    #[test]
    fn http_plain_and_bound_errors_do_not_claim_unverified_identity() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        let rt = Runtime::new(f.db_path(), Arc::clone(&r), "2").unwrap();
        for (wire,status,error) in [
            (b"POST /api/v3/snapshot HTTP/1.1\r\nContent-Length: 1\r\ncontent-length: 1\r\n\r\nx".to_vec(),400,"InvalidQuery"),
            (b"POST /api/v3/snapshot HTTP/1.1\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\nx".to_vec(),400,"InvalidQuery"),
            (http_request("/api/v3/snapshot?x=1", b""),404,"not found"),
            (b"POST /api/v2/snapshot HTTP/1.1\r\n\r\n".to_vec(),404,"not found"),
            (b"GET /api/v3/snapshot HTTP/1.1\r\n\r\n".to_vec(),404,"not found"),
            (http_request("/api/v3/snapshot",b"{\"x\":1,\"x\":2}"),400,"InvalidQuery"),
        ] {
            let (result,response)=exchange(&rt,&wire);result.unwrap();
            let (actual,payload)=http_parts(&response);
            assert_eq!(actual,status);assert_eq!(payload["error"],error);
            assert!(payload.get("identity_binding").is_none());
        }
        let invalid = envelope(&p.wire["fixed_cut"], json!({"op":"watch"}));
        let (result, wire) = exchange(&rt, &http_request("/api/v3/snapshot", &invalid));
        result.unwrap();
        let (status, reply) = http_parts(&wire);
        assert_eq!(status, 400);
        // 请求校验器要求payload.op，不能拿它核无op的错误响应。用原Python回复配方独立对拍。
        error_reply_matches_original_python(&invalid, &reply, Value::Null);
        assert_eq!(
            reply["payload"]["identity_binding"],
            "request_echo_unverified"
        );
        let mut payload = strict_json(&snapshot(&p)).unwrap()["payload"].clone();
        payload["as_of_generation"] = json!("999");
        let unknown_generation = envelope(&p.wire["fixed_cut"], payload);
        let (result, wire) = exchange(&rt, &http_request("/api/v3/snapshot", &unknown_generation));
        result.unwrap();
        let (status, reply) = http_parts(&wire);
        assert_eq!(status, 400);
        assert_eq!(
            reply["payload"]["identity_binding"],
            "verified_current_capture"
        );
        error_reply_matches_original_python(
            &unknown_generation,
            &reply,
            json!({"meta":{"session_id":p.wire["fixed_cut"]["session_id"]},"protocol":{"session_generation":p.wire["fixed_cut"]["session_generation"]}}),
        );
        let unavailable = Runtime::new(f.db_path().with_extension("absent"), r, "2").unwrap();
        let (result, wire) = exchange(
            &unavailable,
            &http_request("/api/v3/snapshot", &snapshot(&p)),
        );
        result.unwrap();
        let (status, reply) = http_parts(&wire);
        assert_eq!(status, 503);
        assert_eq!(reply["payload"]["error"], "StorageUnavailable");
        assert_eq!(
            reply["payload"]["identity_binding"],
            "request_echo_unverified"
        );
    }

    #[test]
    fn ready_is_source_backed_bare_json_and_query_is_rejected_before_capture() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let rt = Runtime::new(f.db_path(), Arc::clone(&r), "7")
            .unwrap()
            .with_control_instance_id("control-test".into());
        let ready = b"GET /api/ready HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (result, wire) = exchange(&rt, ready);
        result.unwrap();
        let (status, body) = http_parts(&wire);
        assert_eq!(status, 200);
        assert_eq!(body.as_object().unwrap().len(), 4);
        assert_eq!(body["ok"], true);
        assert_eq!(body["producer_epoch"], "7");
        assert_eq!(body["control_instance_id"], "control-test");
        let cut = body["cut"].as_object().unwrap();
        assert_eq!(cut.len(), 6);
        let p = projection(&f, &r);
        let fixed = &p.wire["fixed_cut"];
        assert_eq!(
            body["cut"],
            json!({"session_id":fixed["session_id"],
            "session_generation":fixed["session_generation"],"generation":"1",
            "structure_cut":"cut-1","catalog_revision":fixed["catalog_revision"],
            "index_frontier":fixed["index_frontier"]})
        );
        let first_generation = cut["generation"].as_str().unwrap().to_owned();

        publish(&mut f, 2);
        let (result, wire) = exchange(&rt, ready);
        result.unwrap();
        let (_, body) = http_parts(&wire);
        assert_ne!(
            body["cut"]["generation"].as_str().unwrap(),
            first_generation
        );

        // 查询串在数据库打开前即拒绝；即使路径不存在，也不会变成503。
        let unavailable =
            Runtime::new(f.db_path().with_extension("absent"), Arc::clone(&r), "7").unwrap();
        let query = b"GET /api/ready?x=1 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (result, wire) = exchange(&unavailable, query);
        result.unwrap();
        assert_eq!(http_parts(&wire).0, 400);

        let corrupt = f.db_path().with_extension("corrupt-ready");
        std::fs::write(&corrupt, b"not sqlite").unwrap();
        let corrupt_rt = Runtime::new(corrupt.clone(), Arc::clone(&r), "7").unwrap();
        let (result, wire) = exchange(&corrupt_rt, ready);
        result.unwrap();
        assert_eq!(http_parts(&wire).0, 503);
        let _ = std::fs::remove_file(corrupt);

        let mut limited = strict_json(r.canonical().as_bytes()).unwrap();
        limited["limits"]["max_reply_bytes"] = json!("1");
        let limited = Arc::new(QResources::parse(canonical_json(&limited).as_bytes()).unwrap());
        let limited_rt = Runtime::new(f.db_path(), limited, "7").unwrap();
        let (result, wire) = exchange(&limited_rt, ready);
        assert!(result.is_err());
        assert!(wire.is_empty());

        // 控制身份仍完好但业务目录被篡改，ready也必须重新完整核验并拒绝。
        f.conn
            .execute(
                "UPDATE catalog SET title='tampered-ready' WHERE catalog_id='CC-001'",
                [],
            )
            .unwrap();
        let (result, wire) = exchange(&rt, ready);
        result.unwrap();
        assert_eq!(http_parts(&wire).0, 503);
    }
    #[test]
    fn http_frame_includes_headers_and_all_errors_obey_reply_bound() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = resources();
        let p = projection(&f, &r);
        let wire = http_request("/api/v3/snapshot", &snapshot(&p));
        for (limit, status) in [(wire.len(), 200), (wire.len() - 1, 503)] {
            let mut v = strict_json(r.canonical().as_bytes()).unwrap();
            v["limits"]["max_frame_bytes"] = json!(limit.to_string());
            let rt = Runtime::new(
                f.db_path(),
                Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap()),
                "1",
            )
            .unwrap();
            let (result, response) = exchange(&rt, &wire);
            result.unwrap();
            assert_eq!(http_parts(&response).0, status);
        }
        let mut v = strict_json(r.canonical().as_bytes()).unwrap();
        v["limits"]["max_reply_bytes"] = json!("1");
        let rt = Runtime::new(
            f.db_path(),
            Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap()),
            "1",
        )
        .unwrap();
        for request in [wire, b"POST /unknown HTTP/1.1\r\n\r\n".to_vec()] {
            let (result, response) = exchange(&rt, &request);
            assert!(result.is_err());
            assert!(response.is_empty());
        }
    }
    #[test]
    fn capacity_rejection_sends_bounded_503_before_reading_socket() {
        use std::sync::mpsc;
        let mut v = q_resources::tests::input();
        v["limits"]["max_query_workers"] = json!("1");
        v["limits"]["max_query_queue"] = json!("1");
        let rt = Runtime::new(
            PathBuf::from("not-opened-for-rejection"),
            Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap()),
            "1",
        )
        .unwrap();
        let (started_send, started) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let first = rt
            .executor
            .try_submit(move |_, _, _, _| {
                started_send.send(()).unwrap();
                held.recv().unwrap();
                Ok(())
            })
            .unwrap();
        started.recv().unwrap();
        let queued = rt.executor.try_submit(|_, _, _, _| Ok(())).unwrap();
        let (mut client, server) = tcp_pair();
        assert!(rt.accept_http(server).is_err());
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        let (status, payload) = http_parts(&response);
        assert_eq!(status, 503);
        assert_eq!(payload["error"], "QueryBudgetExceeded");
        release.send(()).unwrap();
        first.recv().unwrap();
        queued.recv().unwrap();
        let (result, response) = exchange(&rt, b"POST /unknown HTTP/1.1\r\n\r\n");
        result.unwrap();
        assert_eq!(http_parts(&response).0, 404);
    }
    #[test]
    fn http_header_and_body_preserve_original_separate_sendall_timeouts() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let mut v = q_resources::tests::input();
        v["limits"]["max_query_workers"] = json!("1");
        v["limits"]["socket_write_timeout_ms"] = json!("100");
        let r = Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap());
        let p = projection(&f, &r);
        let rt = Runtime::new(f.db_path(), Arc::clone(&r), "1").unwrap();
        let reply = call(&rt, "/api/v3/watch", &watch(&p));
        struct PartialHeader(usize);
        impl Write for PartialHeader {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                if self.0 == 0 {
                    self.0 = 12;
                    Ok(12)
                } else {
                    Err(io::ErrorKind::BrokenPipe.into())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                panic!("头部失败不能进入body/flush")
            }
        }
        impl q_watch::TimedWrite for PartialHeader {
            fn set_timeout(&mut self, _: Duration) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(send_http(&mut PartialHeader(0), reply, 200, &r, rt.executor.leases()).is_err());
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(0)
        );
        let reply = call(&rt, "/api/v3/watch", &watch(&p));
        struct SlowHeader {
            bytes: Vec<u8>,
            calls: usize,
        }
        impl Write for SlowHeader {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                self.bytes.extend_from_slice(b);
                self.calls += 1;
                if self.calls <= 2 {
                    std::thread::sleep(Duration::from_millis(60));
                }
                Ok(b.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        impl q_watch::TimedWrite for SlowHeader {
            fn set_timeout(&mut self, _: Duration) -> io::Result<()> {
                Ok(())
            }
        }
        let mut out = SlowHeader {
            bytes: Vec::new(),
            calls: 0,
        };
        send_http(&mut out, reply, 200, &r, rt.executor.leases()).unwrap();
        assert_eq!(http_parts(&out.bytes).0, 200);
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(1)
        );
    }

    #[test]
    fn actual_socket_read_timeout_sends_503_and_releases_worker() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let mut v = q_resources::tests::input();
        v["limits"]["max_query_workers"] = json!("1");
        v["limits"]["socket_read_timeout_ms"] = json!("20");
        let r = Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap());
        let p = projection(&f, &r);
        let rt = Runtime::new(f.db_path(), r, "1").unwrap();
        let (mut client, server) = tcp_pair();
        client
            .write_all(b"POST /api/v3/snapshot HTTP/1.1\r\nContent-Length: 8\r\n\r\n{")
            .unwrap();
        // 不发送EOF，迫使实际内核socket读超时，而非把截断当超时。
        let pending = rt.accept_http(server).unwrap();
        let mut wire = Vec::new();
        client.read_to_end(&mut wire).unwrap();
        pending.recv().unwrap();
        let (status, payload) = http_parts(&wire);
        assert_eq!(status, 503);
        assert_eq!(payload["error"], "QueryBudgetExceeded");
        let (result, wire) = exchange(&rt, &http_request("/api/v3/snapshot", &snapshot(&p)));
        result.unwrap();
        assert_eq!(http_parts(&wire).0, 200);
    }
}
