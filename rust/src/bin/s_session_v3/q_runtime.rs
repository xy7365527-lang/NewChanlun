//! #1394 C §5.5/5.6.1：仅由Q21配置构造的查询运行时。
//! 三条v3路由共用现有审计/分页/内容/Watch判据；不创建八字段消费者Job。
//! 本模块不监听HTTP、不接收客户端资源凭据；正式网络入口和有界错误发送另接。
// #1394 新生入口待驱动：本票后续HTTP/CLI接线完成后移除此豁免。
#![allow(dead_code)]
use super::{
    capture::AuditReader,
    q_content,
    q_executor::{Executor, Pending},
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
    q_snapshot, q_watch,
};
use newchan_rust::{
    s_content::{access::Access, job::Job},
    session_protocol::positive,
};
use std::{path::PathBuf, sync::Arc};

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
        })
    }
    pub(super) fn resource_digest(&self) -> &str {
        self.executor.resources().digest()
    }
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
        |capture, access| match request {
            Request::Snapshot(request) => {
                q_watch::Reply::plain(capture.snapshot(&request, epoch, access)?, resources)
            }
            Request::Content(request) => {
                q_watch::Reply::plain(capture.content(&request, epoch, access)?, resources)
            }
            Request::Watch(request) => capture.watch(&request, leases, epoch, access),
        },
    )
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
}
