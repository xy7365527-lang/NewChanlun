//! #1394 C §5.5/5.6.1：仅由Q21配置构造的查询运行时。
//! 三条v3路由共用现有审计/分页/内容/Watch判据；不创建八字段消费者Job。
//! 已接受的TCP连接先占槽，再读有界HTTP请求；本模块不自行bind/监听或加载部署默认值。
use super::business_json::TextValue;
use super::{
    capture::{AuditReader, ConfiguredCapture},
    q_content,
    q_executor::{Executor, Pending},
    q_http_input,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
    q_snapshot, q_watch, q_wire,
};
use newchan_rust::s_content::capture_allocation::{audit_json::JsonView, OwnedBudget};
use newchan_rust::{
    s_content::{access::Access, job::Job},
    session_protocol::positive,
};
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
    allocation: Option<&'a newchan_rust::s_content::capture_allocation::OwnedBudget>,
}
impl<'a> ServerAccess<'a> {
    #[cfg(test)]
    fn new(resources: &'a QResources) -> Self {
        Self::with_allocation(resources, None)
    }
    fn with_allocation(resources: &'a QResources, allocation: Option<&'a OwnedBudget>) -> Self {
        Self {
            _resources: resources,
            allocation,
        }
    }
}
impl Access for ServerAccess<'_> {
    fn allocation_budget(
        &self,
    ) -> Option<&newchan_rust::s_content::capture_allocation::OwnedBudget> {
        self.allocation
    }
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
    epoch: TextValue,
    control_instance_id: TextValue,
    allocation: Option<super::q_allocation::Profile>,
}
impl Runtime {
    pub(super) fn new(
        path: PathBuf,
        resources: Arc<QResources>,
        epoch: &str,
    ) -> Result<Self, String> {
        Self::new_with_allocation(path, resources, epoch, None)
    }
    pub(crate) fn new_with_allocation(
        path: PathBuf,
        resources: Arc<QResources>,
        epoch: &str,
        allocation: Option<super::q_allocation::Profile>,
    ) -> Result<Self, String> {
        positive(epoch, "Q producer_epoch")?;
        if let Some(profile) = &allocation {
            profile.validate()?;
        }
        let budget = allocation.as_ref().map(|profile| &profile.budget);
        let epoch = TextValue::from_str(epoch, budget)?;
        let control_instance_id = TextValue::from_str("", budget)?;
        Ok(Self {
            executor: Executor::new_with_allocation(path, resources, allocation.clone())?,
            epoch,
            control_instance_id,
            allocation,
        })
    }
    pub(super) fn with_control_instance_id(mut self, id: String) -> Result<Self, String> {
        self.control_instance_id = match self.allocation.as_ref() {
            Some(profile) => TextValue::from_str(&id, Some(&profile.budget))?,
            None => TextValue::from(id),
        };
        Ok(self)
    }
    pub(crate) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.allocation.as_ref().map(|p| &p.budget)
    }
    #[cfg(test)]
    pub(super) fn resource_digest(&self) -> &str {
        self.executor.resources().digest()
    }
    /// 连接尚未读取时即交给固定worker；接纳失败仍由本调用发送有界503并关闭连接。
    /// 本口消费连接所有权；返回Err只供记录，调用者不得再发送第二份HTTP响应。
    pub(super) fn accept_http(&self, connection: TcpStream) -> Result<Pending<()>, String> {
        let metadata = self.epoch.try_clone().and_then(|epoch| {
            self.control_instance_id
                .try_clone()
                .map(|control| (epoch, control))
        });
        let (epoch, control_instance_id) = match metadata {
            Ok(metadata) => metadata,
            Err(detail) => {
                let mut connection = connection;
                send_error_payload(
                    &mut connection,
                    self.executor.resources(),
                    self.executor.leases(),
                    &self.epoch,
                    None,
                    None,
                    "QueryBudgetExceeded",
                    Some(&detail),
                    503,
                    self.allocation_budget(),
                )?;
                return Err(detail);
            }
        };
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
            Err((reason, connection)) => {
                self.reject_http(connection, reason)?;
                unreachable!("reject_http发送后返回原拒绝原因")
            }
        }
    }
    /// Listener/调度实际allocator拒绝保留内因；Q21满队列仍用原裸503字节。
    pub(super) fn reject_http(
        &self,
        mut connection: TcpStream,
        reason: String,
    ) -> Result<(), String> {
        let detail = allocation_rejection_detail(&reason);
        send_error_payload(
            &mut connection,
            self.executor.resources(),
            self.executor.leases(),
            &self.epoch,
            None,
            None,
            "QueryBudgetExceeded",
            detail,
            503,
            self.allocation_budget(),
        )?;
        Err(reason)
    }
    #[cfg(test)]
    pub(super) fn dispatch(
        &self,
        path: &str,
        body: &[u8],
    ) -> Result<Pending<q_watch::Reply>, String> {
        let request = parse_request_with_allocation(
            path,
            body,
            self.executor.resources(),
            self.allocation_budget(),
        )?;
        let epoch = self.epoch.try_clone()?;
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
        let request = parse_request_with_allocation(
            path,
            body,
            self.executor.resources(),
            self.allocation_budget(),
        )?;
        let epoch = self.epoch.try_clone()?;
        self.executor
            .try_submit(move |reader, resources, leases, _queued_at| {
                let reply = execute(reader, resources, leases, &epoch, request)?;
                let allocation = reader.allocation_profile().cloned();
                reply.send_body(
                    &mut writer,
                    leases,
                    &mut ServerAccess::with_allocation(
                        resources,
                        allocation.as_ref().map(|p| &p.budget),
                    ),
                )
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
        reply.send_body(
            writer,
            self.executor.leases(),
            &mut ServerAccess::with_allocation(r, self.allocation_budget()),
        )
    }
}

#[cfg(test)]
fn parse_request_with_allocation(
    path: &str,
    body: &[u8],
    resources: &QResources,
    allocation: Option<&OwnedBudget>,
) -> Result<Request, String> {
    if !["/api/v3/snapshot", "/api/v3/content", "/api/v3/watch"].contains(&path) {
        return Err(q_wire::RouteFixed::Route.to_string());
    }
    let owner = q_wire::parse_owner(body, resources, allocation)?;
    let envelope = super::business_json::SharedEnvelope::try_new(owner, allocation)
        .map_err(|(e, _)| e.to_string())?;
    parse_admitted(path, &envelope, body.len(), resources, allocation).map_err(|e| e.to_string())
}
fn parse_admitted<'a>(
    path: &str,
    envelope: &'a super::business_json::SharedEnvelope,
    bytes: usize,
    resources: &QResources,
    allocation: Option<&OwnedBudget>,
) -> Result<Request, q_wire::AdmissionError<'a>> {
    Ok(match path {
        "/api/v3/snapshot" => Request::Snapshot(q_snapshot::Request::parse_admitted(
            envelope, bytes, resources, allocation,
        )?),
        "/api/v3/content" => Request::Content(q_content::Request::parse_admitted(
            envelope, bytes, resources, allocation,
        )?),
        "/api/v3/watch" => Request::Watch(q_watch::Request::parse_admitted(
            envelope, bytes, resources, allocation,
        )?),
        _ => return Err(q_wire::AdmissionError::Fixed(q_wire::RouteFixed::Route)),
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
    let allocation = reader.allocation_profile().cloned();
    reader.with_capture(
        resources,
        &mut ServerAccess::with_allocation(resources, allocation.as_ref().map(|p| &p.budget)),
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
    let allocation = capture.allocation_budget().cloned();
    let wrap = |body: newchan_rust::s_content::CodecVec<u8>| match allocation.as_ref() {
        Some(b) => q_watch::Reply::owned(body, resources, b),
        None => q_watch::Reply::plain(body.try_into_plain().map_err(|e| e.to_string())?, resources),
    };
    match request {
        Request::Snapshot(request) => wrap(capture.snapshot(&request, epoch, access)?),
        Request::Content(request) => wrap(capture.content(&request, epoch, access)?),
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
    let allocation = reader.allocation_profile().cloned();
    let mut input = match q_http_input::Input::with_allocation(
        stream,
        resources,
        allocation.as_ref().map(|p| &p.budget),
    ) {
        Ok(input) => input,
        Err(failure) => {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure,
                allocation.as_ref().map(|p| &p.budget),
            )
        }
    };
    let head = match input.head() {
        Ok(head) => head,
        Err(failure) => {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure,
                allocation.as_ref().map(|p| &p.budget),
            )
        }
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
                allocation.as_ref().map(|p| &p.budget),
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
                allocation.as_ref().map(|p| &p.budget),
            );
        }
        let result = reader.with_capture(
            resources,
            &mut ServerAccess {
                _resources: resources,
                allocation: allocation.as_ref().map(|p| &p.budget),
            },
            |capture, _| {
                use newchan_rust::s_content::capture_allocation::audit_json::JsonView;
                let cut = capture.ready_cut()?;
                let fields = [
                    ("ok", JsonView::Bool(true)),
                    ("cut", cut.view()),
                    ("producer_epoch", JsonView::String(epoch)),
                    ("control_instance_id", JsonView::String(control_instance_id)),
                ];
                let payload = JsonView::object_fields(&fields).map_err(|e| e.to_string())?;
                let budget = capture.allocation_budget().cloned();
                let mut access = ServerAccess {
                    _resources: resources,
                    allocation: budget.as_ref(),
                };
                let body = q_wire::encode_view(
                    payload,
                    resources.limit(Limit::ReplyBytes),
                    &mut access,
                    budget.as_ref(),
                )?
                .ok_or("Incomplete：ready回复超过max_reply_bytes")?;
                match budget.as_ref() {
                    Some(b) => q_watch::Reply::owned(body, resources, b),
                    None => q_watch::Reply::plain(
                        body.try_into_plain().map_err(|e| e.to_string())?,
                        resources,
                    ),
                }
            },
        );
        return match result {
            Ok(reply) => send_http(
                stream,
                reply,
                200,
                resources,
                leases,
                allocation.as_ref().map(|p| &p.budget),
            ),
            Err(detail) => send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure(detail, true),
                allocation.as_ref().map(|p| &p.budget),
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
            allocation.as_ref().map(|p| &p.budget),
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
            allocation.as_ref().map(|p| &p.budget),
        );
    }
    let body = match input.body(&head) {
        Ok(body) => body,
        Err(failure) => {
            return send_failure(
                stream,
                resources,
                leases,
                epoch,
                None,
                None,
                failure,
                allocation.as_ref().map(|p| &p.budget),
            )
        }
    };
    // Head 持有路径owner至本调用结束，不再复制为std String。
    let path = head.path.as_str();
    let owner = match q_wire::decode_owner(&body, resources, allocation.as_ref().map(|p| &p.budget))
    {
        Ok(owner) => owner,
        Err(cause) => {
            return send_parse_failure(
                stream,
                resources,
                leases,
                epoch,
                cause,
                allocation.as_ref().map(|p| &p.budget),
            )
        }
    };
    if let Err(cause) = newchan_rust::session_protocol::validate_envelope_typed(
        owner.view(),
        "s-session/3",
        allocation.as_ref().map(|p| &p.budget),
    ) {
        let cause = q_wire::AdmissionError::Protocol(cause);
        return send_admission_failure(
            stream,
            resources,
            leases,
            epoch,
            None,
            &cause,
            allocation.as_ref().map(|p| &p.budget),
        );
    }
    let envelope = match super::business_json::SharedEnvelope::try_new(
        owner,
        allocation.as_ref().map(|p| &p.budget),
    ) {
        Ok(envelope) => envelope,
        Err((cause, owner)) => {
            let cause = q_wire::AdmissionError::Allocation(cause);
            return send_admission_failure(
                stream,
                resources,
                leases,
                epoch,
                Some(owner.view()),
                &cause,
                allocation.as_ref().map(|p| &p.budget),
            );
        }
    };
    let request = match parse_admitted(
        path,
        &envelope,
        body.len(),
        resources,
        allocation.as_ref().map(|p| &p.budget),
    ) {
        Ok(request) => request,
        Err(cause) => {
            return send_admission_failure(
                stream,
                resources,
                leases,
                epoch,
                Some(envelope.view()),
                &cause,
                allocation.as_ref().map(|p| &p.budget),
            )
        }
    };
    let mut identity = None;
    let result = reader.with_capture(
        resources,
        &mut ServerAccess {
            _resources: resources,
            allocation: allocation.as_ref().map(|p| &p.budget),
        },
        |capture, access| {
            identity = Some(capture.identity()?);
            execute_captured(capture, resources, leases, epoch, request, access)
        },
    );
    match result {
        Ok(reply) => send_http(
            stream,
            reply,
            200,
            resources,
            leases,
            allocation.as_ref().map(|p| &p.budget),
        ),
        Err(detail) => send_failure(
            stream,
            resources,
            leases,
            epoch,
            Some(envelope.view()),
            identity.as_ref().map(|(s, g)| (s.as_str(), g.as_str())),
            failure(detail, identity.is_none()),
            allocation.as_ref().map(|p| &p.budget),
        ),
    }
}

/// 仅内部allocator返回的原始code是新实验额度拒绝；不可把它称作Q21满队列。
fn allocation_rejection_detail(reason: &str) -> Option<&str> {
    [
        newchan_rust::s_content::capture_allocation::LIMIT,
        newchan_rust::s_content::capture_allocation::INCOMPLETE,
    ]
    .iter()
    .any(|code| reason.starts_with(code))
    .then_some(reason)
}

/// 只剥实际内部错误包装链；SchemaUnsupported中的键名或诊断内容不是预算来源。
fn prototype_allocation_cause(mut detail: &str) -> bool {
    const WRAPPERS: [&str; 6] = [
        "QReplyInvalid：",
        "Q3CaptureInvalid：",
        "S3StateInvalid：",
        "QSnapshotInvalid：",
        "QContentInvalid：",
        "QWatchInvalid：",
    ];
    while let Some(inner) = WRAPPERS
        .iter()
        .find_map(|prefix| detail.strip_prefix(prefix))
    {
        detail = inner;
    }
    allocation_rejection_detail(detail).is_some()
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
    .any(|code| detail.contains(code))
        // 新prototype原因只认内部CodecError包装链；Strict键名/语法诊断不可冒充额度拒绝。
        || prototype_allocation_cause(&detail);
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
        detail: detail.into(),
    }
}

fn send_failure<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Option<JsonView<'_>>,
    identity: Option<(&str, &str)>,
    failure: q_http_input::Failure,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    send_error_payload(
        stream,
        resources,
        leases,
        epoch,
        request,
        identity,
        failure.error,
        Some(failure.detail.as_str()),
        failure.status,
        allocation,
    )
}

/// 借用typed原因直到完整HTTP回包flush；来源owner留在调用栈或共享envelope中。
fn send_admission_failure<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Option<JsonView<'_>>,
    cause: &q_wire::AdmissionError<'_>,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    let budget = cause.typed_budget() || q_wire::legacy_budget(cause);
    send_error_payload_display(
        stream,
        resources,
        leases,
        epoch,
        request,
        None,
        if budget {
            "QueryBudgetExceeded"
        } else {
            "InvalidQuery"
        },
        Some(cause),
        if budget { 503 } else { 400 },
        allocation,
    )
}
fn send_parse_failure<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    cause: q_wire::ParseOwnerError,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    let budget =
        matches!(&cause, q_wire::ParseOwnerError::Allocation(_)) || q_wire::legacy_budget(&cause);
    send_error_payload_display(
        stream,
        resources,
        leases,
        epoch,
        None,
        None,
        if budget {
            "QueryBudgetExceeded"
        } else {
            "InvalidQuery"
        },
        Some(&cause),
        if budget { 503 } else { 400 },
        allocation,
    )
}

struct DisplayDetail<'a>(&'a dyn std::fmt::Display);
impl serde::Serialize for DisplayDetail<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // 锁版serde_json 1.0.151直接向escaped writer写Display各段，无临时to_string。
        serializer.collect_str(self.0)
    }
}
struct ErrorPayload<'a> {
    error: &'a str,
    detail: Option<&'a dyn std::fmt::Display>,
    identity_binding: Option<&'a str>,
}
impl serde::Serialize for ErrorPayload<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let budget = self.error == "QueryBudgetExceeded";
        let len = 2
            + usize::from(self.detail.is_some())
            + usize::from(self.identity_binding.is_some())
            + usize::from(budget);
        let mut map = serializer.serialize_map(Some(len))?;
        // 原JsonView对象UTF8键排序；不添加公共字段，不截断完整诊断。
        if let Some(detail) = self.detail {
            map.serialize_entry("detail", &DisplayDetail(detail))?;
        }
        map.serialize_entry("error", self.error)?;
        if let Some(binding) = self.identity_binding {
            map.serialize_entry("identity_binding", binding)?;
        }
        map.serialize_entry("ok", &false)?;
        if budget {
            map.serialize_entry("status", "Incomplete")?;
        }
        map.end()
    }
}

fn send_error_payload<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    request: Option<JsonView<'_>>,
    identity: Option<(&str, &str)>,
    error: &str,
    detail: Option<&str>,
    status: u16,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    send_error_payload_display(
        stream,
        resources,
        leases,
        epoch,
        request,
        identity,
        error,
        detail.as_ref().map(|value| value as &dyn std::fmt::Display),
        status,
        allocation,
    )
}

fn send_error_payload_display<S: q_watch::TimedWrite>(
    stream: &mut S,
    resources: &QResources,
    _leases: &ClientLeases,
    epoch: &str,
    request: Option<JsonView<'_>>,
    identity: Option<(&str, &str)>,
    error: &str,
    detail: Option<&dyn std::fmt::Display>,
    status: u16,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    let payload = ErrorPayload {
        error,
        detail,
        identity_binding: request.map(|_| {
            if identity.is_some() {
                "verified_current_capture"
            } else {
                "request_echo_unverified"
            }
        }),
    };
    let mut access = ServerAccess::with_allocation(resources, allocation);
    if let Some(request) = request {
        let (session, incarnation) = match identity {
            Some(identity) => identity,
            None => (
                super::business_json::field(request, "session_id")
                    .as_str()
                    .ok_or("已核请求缺session")?,
                super::business_json::field(request, "session_generation")
                    .as_str()
                    .ok_or("已核请求缺化身")?,
            ),
        };
        let envelope = q_wire::ErrorEnvelope::new(request, &payload, session, incarnation, epoch)?;
        send_serialized(
            stream,
            &envelope,
            status,
            resources,
            &mut access,
            "QReplyInvalid：Incomplete：完整Q消息体超过max_reply_bytes",
        )
    } else {
        send_serialized(
            stream,
            &payload,
            status,
            resources,
            &mut access,
            "Incomplete：错误消息体超过max_reply_bytes，关闭连接",
        )
    }
}

/// 先计完整规范字节再发头；值、已核公头和诊断文本借用活到flush。
/// 两遍都走受控compact JSON同核，栈writer不另造body owner或默认资源数。
fn send_serialized<W: q_watch::TimedWrite>(
    writer: &mut W,
    value: &impl serde::Serialize,
    status: u16,
    resources: &QResources,
    access: &mut dyn Access,
    too_large: &str,
) -> Result<(), String> {
    struct Count {
        len: u128,
    }
    impl io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            // 计完整长度不以io错误短路，避免max_reply拒绝在serde错误箱中另配heap。
            self.len += bytes.len() as u128;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count { len: 0 };
    newchan_rust::s_content::json_writer::write_json(&mut count, value)
        .map_err(|e| format!("QReplyInvalid：{e}"))?;
    if count.len > resources.limit(Limit::ReplyBytes) as u128 || count.len > usize::MAX as u128 {
        return Err(too_large.into());
    }
    send_header(writer, status, count.len as usize, resources)?;
    enum BodyFailure {
        Deadline,
        Zero,
        Codec(newchan_rust::s_content::CodecError),
        Io(io::Error),
    }
    impl std::fmt::Display for BodyFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Deadline => f.write_str("Incomplete：回复写入绝对期限已过"),
                Self::Zero => f.write_str("WriteZero：回复未完整写出"),
                Self::Codec(error) => std::fmt::Display::fmt(error, f),
                Self::Io(error) => std::fmt::Display::fmt(error, f),
            }
        }
    }
    struct Body<'a, W> {
        writer: &'a mut W,
        access: &'a mut dyn Access,
        deadline: Instant,
        first: Option<BodyFailure>,
    }
    impl<W: q_watch::TimedWrite> Body<'_, W> {
        fn fail<T>(&mut self, cause: BodyFailure) -> io::Result<T> {
            if self.first.is_none() {
                self.first = Some(cause);
            }
            // 原错误保留在stack enum，serde收到非分配的kind占位后立即停止。
            Err(io::ErrorKind::Other.into())
        }
        fn prepare(&mut self) -> io::Result<()> {
            if let Err(error) = self.access.pre_io_check() {
                return self.fail(BodyFailure::Codec(error));
            }
            let remaining = match self
                .deadline
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
            {
                Some(remaining) => remaining,
                None => return self.fail(BodyFailure::Deadline),
            };
            match self.writer.set_timeout(remaining) {
                Ok(()) => Ok(()),
                Err(error) => self.fail(BodyFailure::Io(error)),
            }
        }
    }
    impl<W: q_watch::TimedWrite> io::Write for Body<'_, W> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.prepare()?;
            let n = match self.writer.write(bytes) {
                Ok(0) if !bytes.is_empty() => return self.fail(BodyFailure::Zero),
                Ok(n) => n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
                Err(error) => return self.fail(BodyFailure::Io(error)),
            };
            if let Err(error) = self.access.charge_wire_bytes(n as u64) {
                return self.fail(BodyFailure::Codec(error));
            }
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.prepare()?;
            match self.writer.flush() {
                Ok(()) => Ok(()),
                Err(error) => self.fail(BodyFailure::Io(error)),
            }
        }
    }
    let local = Instant::now()
        .checked_add(Duration::from_millis(resources.limit(Limit::SocketWriteMs)))
        .ok_or("QWatchInvalid：写期限溢出")?;
    let deadline = access
        .read_deadline()
        .map_err(|e| format!("QWatchInvalid：{e}"))?
        .map_or(local, |d| local.min(d));
    let mut body = Body {
        writer,
        access,
        deadline,
        first: None,
    };
    let encoded = newchan_rust::s_content::json_writer::write_json(&mut body, value);
    if let Some(cause) = body.first.take() {
        return Err(format!("QWatchInvalid：{cause}"));
    }
    encoded.map_err(|e| format!("QWatchInvalid：{e}"))?;
    let flushed = io::Write::flush(&mut body);
    if let Some(cause) = body.first.take() {
        return Err(format!("QWatchInvalid：{cause}"));
    }
    flushed.map_err(|e| format!("QWatchInvalid：{e}"))?;
    body.access
        .pre_io_check()
        .map_err(|e| format!("QWatchInvalid：{e}"))?;
    if Instant::now() >= deadline {
        return Err("QWatchInvalid：Incomplete：回复写入绝对期限已过".into());
    }
    // 错误回复无Delivery；partial header/body/flush绝不调用leases.delivered。
    Ok(())
}

fn send_header<W: q_watch::TimedWrite>(
    writer: &mut W,
    status: u16,
    body_len: usize,
    resources: &QResources,
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
    // 固定字面量 + status 3位 +最长reason 31字节 + usize十进制最大位数。
    // 用BITS作位数保守上界，每一位十进制至少需要一位二进制，含最末CRLF。
    const FIXED: &str = "HTTP/1.0  \r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: \r\nCache-Control: no-store\r\nConnection: close\r\n\r\n";
    const HEADER_MAX: usize =
        FIXED.len() + 3 + "Request Header Fields Too Large".len() + usize::BITS as usize;
    struct Header {
        bytes: [u8; HEADER_MAX],
        len: usize,
    }
    impl std::fmt::Write for Header {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let end = self.len.checked_add(value.len()).ok_or(std::fmt::Error)?;
            let target = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
            target.copy_from_slice(value.as_bytes());
            self.len = end;
            Ok(())
        }
    }
    let mut header = Header {
        bytes: [0; HEADER_MAX],
        len: 0,
    };
    use std::fmt::Write;
    write!(&mut header, "HTTP/1.0 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {body_len}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n")
        .map_err(|_| "Q HTTP头栈编码上界错误")?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(resources.limit(Limit::SocketWriteMs)))
        .ok_or("Q HTTP写期限溢出")?;
    let mut offset = 0;
    while offset < header.len {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("Incomplete：HTTP头写期限已过")?;
        writer.set_timeout(remaining).map_err(|e| e.to_string())?;
        match writer.write(&header.bytes[offset..header.len]) {
            Ok(0) => return Err("HTTP头WriteZero".into()),
            Ok(n) => offset += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("HTTP头写入失败：{e}")),
        }
    }
    if Instant::now() >= deadline {
        return Err("Incomplete：HTTP响应头写入超过本阶段期限".into());
    }
    Ok(())
}

fn send_http<W: q_watch::TimedWrite>(
    writer: &mut W,
    reply: q_watch::Reply,
    status: u16,
    resources: &QResources,
    leases: &ClientLeases,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    // reply owner在栈上始终活到send_body完整flush；两阶段各取原同值期限。
    send_header(writer, status, reply.body().len(), resources)?;
    reply.send_body(
        writer,
        leases,
        &mut ServerAccess::with_allocation(resources, allocation),
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
    pub(super) fn resources() -> Arc<QResources> {
        let mut v = q_resources::tests::functional_input();
        v["limits"]["max_query_workers"] = json!("1");
        Arc::new(QResources::parse(canonical_json(&v).as_bytes()).unwrap())
    }
    pub(super) fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    pub(super) fn projection(f: &Fixture, r: &QResources) -> Projection {
        let mut reader = AuditReader::new(f.db_path(), r);
        reader
            .with_capture(r, &mut ServerAccess::new(r), |c, a| {
                c.projection(1, HistoryMode::AsKnown, a)
            })
            .unwrap()
    }
    pub(super) fn envelope(fixed: &Value, payload: Value) -> Vec<u8> {
        canonical_json(&json!({"schema_revision":"s-session/3","session_id":fixed["session_id"],"session_generation":fixed["session_generation"],"source_namespace":"q-runtime-test","source_epoch":"1","message_id":"request","producer_id":"test-client","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})).into_bytes()
    }
    fn plain_wire(p: &Projection) -> &Value {
        match &p.wire {
            super::super::business_json::JsonOwner::Plain(value) => value.as_ref(),
            #[cfg(feature = "s_allocation_json_testonly")]
            super::super::business_json::JsonOwner::Owned(_) => {
                panic!("测试oracle只接普通投影")
            }
        }
    }
    pub(super) fn snapshot(p: &Projection) -> Vec<u8> {
        let f = &plain_wire(p)["fixed_cut"];
        envelope(
            f,
            json!({"op":"snapshot","session_id":f["session_id"],"session_generation":f["session_generation"],"scope":f["scope"],"history_mode":"AsKnown","as_of_generation":"1","page_size":"1","order_version":plain_wire(p)["order_version"]}),
        )
    }
    pub(super) fn watch(p: &Projection) -> Vec<u8> {
        let f = &plain_wire(p)["fixed_cut"];
        envelope(
            f,
            json!({"op":"watch","cursor":{"session_id":f["session_id"],"session_generation":f["session_generation"],"catalog_revision":f["catalog_revision"],"scope":f["scope"],"after_generation":"0","base_cut":"cut-0","index_frontier":"","last_seq":"-1","order_version":plain_wire(p)["order_version"]},"max_batches":"1","client_id":"runtime-client"}),
        )
    }
    fn call(rt: &Runtime, path: &str, body: &[u8]) -> q_watch::Reply {
        rt.dispatch(path, body).unwrap().recv().unwrap()
    }
    #[test]
    fn historical_layout1_published_session_asknown_q() {
        let history = super::super::generation::tests::issue1419_history();
        let path = history.path().join("historical-published-layout1.sqlite");
        let baseline: Value = serde_json::from_slice(
            &std::fs::read(
                path.parent()
                    .unwrap()
                    .join("historical-published-q-baseline.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let r = resources();
        let rt = Runtime::new(path.clone(), Arc::clone(&r), "7").unwrap();
        let mut reader = AuditReader::new(path, &r);
        for generation in 1..=4 {
            let p = reader
                .with_capture(&r, &mut ServerAccess::new(&r), |c, a| {
                    c.projection(generation, HistoryMode::AsKnown, a)
                })
                .unwrap();
            let fixed = &plain_wire(&p)["fixed_cut"];
            let request = envelope(
                fixed,
                json!({"op":"snapshot", "session_id":fixed["session_id"],
                "session_generation":fixed["session_generation"], "scope":fixed["scope"],
                "history_mode":"AsKnown", "as_of_generation":generation.to_string(),
                "page_size":"1", "order_version":plain_wire(&p)["order_version"]}),
            );
            let response = call(&rt, "/api/v3/snapshot", &request);
            let page = strict_json(response.body()).unwrap();
            assert_eq!(page["payload"]["fixed_cut"], *fixed);
            let expected = &baseline[(generation - 1) as usize];
            assert_eq!(plain_wire(&p), &expected["projection"]);
            assert_eq!(page, expected["snapshot"]);
            let content = envelope(
                fixed,
                json!({"op":"content","anchor":p.anchor(),
                "path":[{"field":"rows"}],"unit":"items","offset":"0","limit":"1"}),
            );
            let content = call(&rt, "/api/v3/content", &content);
            let body = strict_json(content.body()).unwrap();
            assert!(!body["payload"]["nodes"].as_array().unwrap().is_empty());
            assert_eq!(body, expected["content"]);
        }
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
        assert_eq!(page["payload"]["fixed_cut"], plain_wire(&p)["fixed_cut"]);
        assert_eq!(page["producer_epoch"], "7");
        let token = page["payload"]["next_token"].clone();
        assert!(token.is_object());
        publish(&mut f, 2);
        let next = call(
            &rt,
            "/api/v3/snapshot",
            &envelope(
                &plain_wire(&p)["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":token}),
            ),
        );
        let next = strict_json(next.body()).unwrap();
        assert_eq!(next["payload"]["fixed_cut"], plain_wire(&p)["fixed_cut"]);
        assert_ne!(
            next["payload"]["validated_capture_digest"],
            page["payload"]["validated_capture_digest"]
        );
        let content = envelope(
            &plain_wire(&p)["fixed_cut"],
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
        let mut config = q_resources::tests::functional_input();
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
    pub(super) fn http_request(path: &str, body: &[u8]) -> Vec<u8> {
        let mut wire = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        wire.extend_from_slice(body);
        wire
    }
    pub(super) fn exchange(rt: &Runtime, wire: &[u8]) -> (Result<(), String>, Vec<u8>) {
        let (mut client, server) = tcp_pair();
        client.write_all(wire).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let pending = rt.accept_http(server);
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        let result = pending.and_then(|p| p.recv());
        (result, response)
    }
    pub(super) fn http_parts(wire: &[u8]) -> (u16, Value) {
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
            &plain_wire(&p)["fixed_cut"],
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
        let invalid = envelope(&plain_wire(&p)["fixed_cut"], json!({"op":"watch"}));
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
        let unknown_generation = envelope(&plain_wire(&p)["fixed_cut"], payload);
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
            json!({"meta":{"session_id":plain_wire(&p)["fixed_cut"]["session_id"]},"protocol":{"session_generation":plain_wire(&p)["fixed_cut"]["session_generation"]}}),
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
            .with_control_instance_id("control-test".into())
            .unwrap();
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
        let fixed = &plain_wire(&p)["fixed_cut"];
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
        let mut v = q_resources::tests::functional_input();
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
        let mut v = q_resources::tests::functional_input();
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
        assert!(send_http(
            &mut PartialHeader(0),
            reply,
            200,
            &r,
            rt.executor.leases(),
            None
        )
        .is_err());
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
        send_http(&mut out, reply, 200, &r, rt.executor.leases(), None).unwrap();
        assert_eq!(http_parts(&out.bytes).0, 200);
        assert_eq!(
            rt.executor.leases().delivered_generation("runtime-client"),
            Some(1)
        );
    }

    #[test]
    fn borrowed_error_envelope_matches_original_bytes_with_escaped_giant_identity() {
        let r = resources();
        let session = "中文😀\"\\\n".repeat(8192);
        let request = json!({"schema_revision":"s-session/3", "session_id":session,
            "session_generation":"化身\\\"😀", "source_namespace":"请求😀",
            "source_epoch":"1", "message_id":"公头\\\"\n", "producer_id":"caller",
            "producer_epoch":"1", "payload_hash":"abc", "causal_refs":[], "payload":{}});
        for identity in [None, Some(("已核😀\\\"", "当前化身"))] {
            let binding = if identity.is_some() {
                "verified_current_capture"
            } else {
                "request_echo_unverified"
            };
            let payload = json!({"ok":false,"error":"InvalidQuery","detail":"首错中文\\\"\n😀","identity_binding":binding});
            let (session, incarnation) = identity.unwrap_or((
                request["session_id"].as_str().unwrap(),
                request["session_generation"].as_str().unwrap(),
            ));
            let expected = q_wire::reply(
                &request,
                payload.clone(),
                session,
                incarnation,
                "7",
                &r,
                &mut ServerAccess::new(&r),
            )
            .unwrap();
            let envelope =
                q_wire::ErrorEnvelope::new((&request).into(), &payload, session, incarnation, "7")
                    .unwrap();
            assert_eq!(serde_json::to_vec(&envelope).unwrap(), expected);
        }
    }

    #[test]
    fn stack_http_header_preserves_all_declared_statuses_and_usize_max_length() {
        struct Out(Vec<u8>);
        impl Write for Out {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        impl q_watch::TimedWrite for Out {
            fn set_timeout(&mut self, _: Duration) -> io::Result<()> {
                Ok(())
            }
        }
        let r = resources();
        for (status, reason) in [
            (200, "OK"),
            (400, "Bad Request"),
            (404, "Not Found"),
            (414, "Request-URI Too Long"),
            (431, "Request Header Fields Too Large"),
            (501, "Not Implemented"),
            (503, "Service Unavailable"),
        ] {
            for len in [0, usize::MAX] {
                let mut out = Out(Vec::new());
                send_header(&mut out, status, len, &r).unwrap();
                let expected = format!("HTTP/1.0 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {len}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n");
                assert_eq!(out.0, expected.as_bytes());
            }
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn zero_quota_error_stream_matches_none_and_checks_complete_length_before_header() {
        struct Out {
            bytes: Vec<u8>,
            flushed: bool,
            fail_flush: bool,
        }
        impl Write for Out {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                self.flushed = true;
                if self.fail_flush {
                    Err(io::ErrorKind::BrokenPipe.into())
                } else {
                    Ok(())
                }
            }
        }
        impl q_watch::TimedWrite for Out {
            fn set_timeout(&mut self, _: Duration) -> io::Result<()> {
                Ok(())
            }
        }
        let r = resources();
        let leases = ClientLeases::new(&r);
        let budget = OwnedBudget::new(0);
        let request = json!({"session_id":"中文😀\\\"", "session_generation":"化身", "source_namespace":"caller",
            "source_epoch":"1", "message_id":"id", "payload_hash":"hash"});
        let mut none = Out {
            bytes: Vec::new(),
            flushed: false,
            fail_flush: false,
        };
        send_error_payload(
            &mut none,
            &r,
            &leases,
            "1",
            Some((&request).into()),
            None,
            "QueryBudgetExceeded",
            Some("first failure\\\"😀"),
            503,
            None,
        )
        .unwrap();
        let mut owned = Out {
            bytes: Vec::new(),
            flushed: false,
            fail_flush: false,
        };
        let before = budget.snapshot();
        send_error_payload(
            &mut owned,
            &r,
            &leases,
            "1",
            Some((&request).into()),
            None,
            "QueryBudgetExceeded",
            Some("first failure\\\"😀"),
            503,
            Some(&budget),
        )
        .unwrap();
        assert_eq!(none.bytes, owned.bytes);
        assert!(owned.flushed);
        let after = budget.snapshot();
        assert_eq!(after.allocation_attempts, before.allocation_attempts);
        assert_eq!(after.allocated_layouts, before.allocated_layouts);
        assert_eq!(after.rejected_layouts, before.rejected_layouts);
        assert_eq!(
            after.live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        let body_len = owned.bytes.len()
            - owned
                .bytes
                .windows(4)
                .position(|v| v == b"\r\n\r\n")
                .unwrap()
            - 4;
        for limit in [body_len - 1, body_len] {
            let mut values = q_resources::tests::functional_input();
            values["limits"]["max_reply_bytes"] = json!(limit.to_string());
            let limited = QResources::parse(canonical_json(&values).as_bytes()).unwrap();
            let mut out = Out {
                bytes: Vec::new(),
                flushed: false,
                fail_flush: false,
            };
            let result = send_error_payload(
                &mut out,
                &limited,
                &leases,
                "1",
                Some((&request).into()),
                None,
                "QueryBudgetExceeded",
                Some("first failure\\\"😀"),
                503,
                Some(&budget),
            );
            assert_eq!(result.is_ok(), limit == body_len);
            if limit < body_len {
                assert!(out.bytes.is_empty());
                assert!(!out.flushed);
            }
        }
        let mut out = Out {
            bytes: Vec::new(),
            flushed: false,
            fail_flush: true,
        };
        assert!(send_error_payload(
            &mut out,
            &r,
            &leases,
            "1",
            Some((&request).into()),
            None,
            "QueryBudgetExceeded",
            Some("first failure\\\"😀"),
            503,
            Some(&budget)
        )
        .is_err());
        assert!(out.flushed);
        assert_eq!(leases.delivered_generation("runtime-client"), None);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_dispatch_layout_refusal_keeps_cause_and_q21_full_queue_keeps_bare_bytes() {
        use super::super::q_allocation::Profile;
        use newchan_rust::s_content::capture_allocation::{CaptureSlab, LIMIT};
        let fixture = Fixture::new();
        let r = resources();
        let normal = Runtime::new(fixture.db_path(), Arc::clone(&r), "1").unwrap();
        let (mut client, server) = tcp_pair();
        let reason = "Incomplete：Q executor active+queued 达到冻结上限".to_owned();
        assert_eq!(
            normal.reject_http(server, reason.clone()).unwrap_err(),
            reason
        );
        let mut wire = Vec::new();
        client.read_to_end(&mut wire).unwrap();
        let (status, payload) = http_parts(&wire);
        assert_eq!(status, 503);
        assert_eq!(
            payload,
            json!({"ok":false,"status":"Incomplete","error":"QueryBudgetExceeded"})
        );
        let end = wire.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        assert_eq!(&wire[end..], canonical_json(&payload).as_bytes());

        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let owned = Runtime::new_with_allocation(
            fixture.db_path(),
            Arc::clone(&r),
            "1",
            Some(Profile {
                budget: budget.clone(),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            }),
        )
        .unwrap();
        // 留够真实metadata clone，拒绝点因此落在空queue的实际Layout。
        let epoch_clone = owned.epoch.try_clone().unwrap();
        let epoch_layout = budget.allocation_request_probe().last_request_bytes;
        drop(epoch_clone);
        let live = budget.snapshot().live_requested_layout_bytes;
        let fill = CaptureSlab::new(
            0,
            budget.snapshot().limit_bytes - live - epoch_layout,
            &budget,
        )
        .unwrap();
        let (mut client, server) = tcp_pair();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let reason = match owned.accept_http(server) {
            Ok(_) => panic!("queue Layout应被拒绝"),
            Err(reason) => reason,
        };
        assert!(reason.starts_with(LIMIT), "{reason}");
        let mut wire = Vec::new();
        client.read_to_end(&mut wire).unwrap();
        let (status, payload) = http_parts(&wire);
        assert_eq!(status, 503);
        assert_eq!(
            payload,
            json!({"ok":false,"status":"Incomplete","error":"QueryBudgetExceeded","detail":reason})
        );
        let end = wire.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        assert_eq!(&wire[end..], canonical_json(&payload).as_bytes());
        assert!(budget.allocation_request_probe().refused_bytes > 0);
        assert_eq!(
            owned
                .executor
                .leases()
                .delivered_generation("runtime-client"),
            None
        );
        drop(fill);
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn prototype_failure_classification_keeps_strict_keys_400_and_trusted_causes_503() {
        use newchan_rust::s_content::{
            capture_allocation::{INCOMPLETE, LIMIT},
            CodecError,
        };
        for marker in [LIMIT, INCOMPLETE] {
            let bytes = format!("{{\"{marker}\":0,\"{marker}\":1}}");
            let detail = strict_json(bytes.as_bytes()).unwrap_err();
            let rejected = failure(detail.clone(), false);
            assert_eq!((rejected.status, rejected.error), (400, "InvalidQuery"));
            assert_eq!(rejected.detail.as_str().as_bytes(), detail.as_bytes());
            let syntax_wrapper = format!("QReplyInvalid：{detail}");
            let rejected = failure(syntax_wrapper.clone(), false);
            assert_eq!((rejected.status, rejected.error), (400, "InvalidQuery"));
            assert_eq!(rejected.detail.as_str(), syntax_wrapper);

            let cause = CodecError::new(marker, "真实预算原因").to_string();
            for prefix in [
                "",
                "QReplyInvalid：",
                "Q3CaptureInvalid：",
                "S3StateInvalid：",
                "QSnapshotInvalid：",
                "QContentInvalid：",
                "QWatchInvalid：",
                "Q3CaptureInvalid：S3StateInvalid：QReplyInvalid：",
                "QWatchInvalid：QContentInvalid：QSnapshotInvalid：",
            ] {
                let detail = format!("{prefix}{cause}");
                for capture_failed in [false, true] {
                    let rejected = failure(detail.clone(), capture_failed);
                    assert_eq!(
                        (rejected.status, rejected.error),
                        (503, "QueryBudgetExceeded")
                    );
                    assert_eq!(rejected.detail.as_str().as_bytes(), detail.as_bytes());
                }
            }
        }
        // 原五条contains规则和Q21首错不由本补丁重新定义。
        for marker in [
            "Incomplete",
            "CaptureDeadline",
            "VerifyDeadline",
            "CaptureBytes",
            "atomic_delta_limit",
        ] {
            let detail = format!("既有上下文 {marker} 原原因");
            let rejected = failure(detail.clone(), false);
            assert_eq!(
                (rejected.status, rejected.error),
                (503, "QueryBudgetExceeded")
            );
            assert_eq!(rejected.detail.as_str(), detail);
        }
        let detail = "Incomplete：Q executor active+queued 达到冻结上限".to_owned();
        let rejected = failure(detail.clone(), false);
        assert_eq!(
            (rejected.status, rejected.error),
            (503, "QueryBudgetExceeded")
        );
        assert_eq!(rejected.detail.as_str(), detail);
        // Q21裸HTTP reply仍由原reject_http测试逐字核验。
    }

    #[test]
    fn actual_socket_read_timeout_sends_503_and_releases_worker() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let mut v = q_resources::tests::functional_input();
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

#[cfg(all(
    test,
    feature = "s_allocation_json_testonly",
    feature = "s_allocation_sqlite_testonly"
))]
mod actual_owned_http_probe {
    use super::super::{prepared::tests::Fixture, q_allocation::Profile};
    use super::*;
    use newchan_rust::s_content::capture_allocation::{
        sqlite::TestOnlySqliteAllocator, OwnedBudget,
    };
    #[test]
    #[ignore = "only this exact test in a fresh SQLite process"]
    fn actual_owned_tcp_three_routes_preserve_complete_replies_cache_refusal_and_release() {
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        // SAFETY: fresh exact process before fixture/open; all worker/fixture SQL drops before finish.
        let guard = unsafe { TestOnlySqliteAllocator::install(&budget) }.unwrap();
        let control = guard.control();
        let mut fixture = Fixture::new();
        tests::publish(&mut fixture, 1);
        tests::publish(&mut fixture, 2);
        let resources = tests::resources();
        let projection = tests::projection(&fixture, &resources);
        let fixed = match &projection.wire {
            super::super::business_json::JsonOwner::Plain(v) => &v["fixed_cut"],
            _ => panic!("oracle must be Plain"),
        };
        let content = tests::envelope(
            fixed,
            serde_json::json!({"op":"content","anchor":projection.anchor(),"path":[{"field":"rows"}],"unit":"items","offset":"0","limit":"1"}),
        );
        let routes = [
            ("/api/v3/snapshot", tests::snapshot(&projection)),
            ("/api/v3/content", content),
            ("/api/v3/watch", tests::watch(&projection)),
        ];
        let normal = Runtime::new(fixture.db_path(), Arc::clone(&resources), "1").unwrap();
        let owned = Runtime::new_with_allocation(
            fixture.db_path(),
            Arc::clone(&resources),
            "1",
            Some(Profile {
                budget: budget.clone(),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            }),
        )
        .unwrap();
        for (path, request) in &routes {
            let input = tests::http_request(path, request);
            let (n, nwire) = tests::exchange(&normal, &input);
            n.unwrap();
            let (o, owire) = tests::exchange(&owned, &input);
            o.unwrap();
            let (ns, nbody) = tests::http_parts(&nwire);
            let (os, obody) = tests::http_parts(&owire);
            assert_eq!((ns, os), (200, 200));
            assert_eq!(nbody, obody);
            assert_eq!(
                &nwire[nwire.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4..],
                &owire[owire.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4..]
            );
        }
        // 复用同50MiB/TCP档；错误与ready同样比较完整HTTP字节。
        let invalid = tests::envelope(fixed, serde_json::json!({"op":"watch"}));
        let malformed = tests::http_request("/api/v3/snapshot", b"{\"x\":1,\"x\":2}");
        let mut unknown =
            newchan_rust::session_protocol::strict_json(&routes[0].1).unwrap()["payload"].clone();
        unknown["as_of_generation"] = serde_json::json!("999");
        let unknown = tests::envelope(fixed, unknown);
        for input in [
            tests::http_request("/api/v3/snapshot", &invalid),
            malformed,
            tests::http_request("/api/v3/snapshot", &unknown),
            b"GET /api/ready?x=1 HTTP/1.1\r\n\r\n".to_vec(),
            b"GET /api/ready HTTP/1.1\r\n\r\n".to_vec(),
        ] {
            let (n, nwire) = tests::exchange(&normal, &input);
            let (o, owire) = tests::exchange(&owned, &input);
            n.unwrap();
            o.unwrap();
            assert_eq!(nwire, owire);
        }
        // 原Strict独立oracle锁住None旧taxonomy/行列；实际TCP两模式逐字对拍。
        for bytes in super::super::business_json::strict_owner_protocol_matrix() {
            let input = tests::http_request("/api/v3/snapshot", &bytes);
            let (n, nwire) = tests::exchange(&normal, &input);
            let (o, owire) = tests::exchange(&owned, &input);
            n.unwrap();
            o.unwrap();
            assert_eq!(nwire, owire, "strict grammar HTTP {:?}", bytes);
            if let Err(original) = newchan_rust::session_protocol::strict_json(&bytes) {
                let (status, payload) = tests::http_parts(&nwire);
                let expected = failure(original.clone(), false);
                assert_eq!(status, expected.status);
                assert_eq!(payload["error"], expected.error);
                if bytes.is_empty() {
                    // Head先拒绝零长度；不能由后续JSON EOF接管首错。
                    assert_eq!(payload["detail"], "Content-Length超出i64正整数域");
                } else {
                    assert_eq!(payload["detail"], original);
                }
            }
        }
        assert_eq!(
            owned
                .executor
                .leases()
                .delivered_generation("runtime-client"),
            normal
                .executor
                .leases()
                .delivered_generation("runtime-client")
        );
        let live = budget.snapshot();
        assert!(live.live_requested_layout_bytes > budget.ledger_health().sqlite_live_layout_bytes);
        let old_delivery = owned
            .executor
            .leases()
            .delivered_generation("runtime-client");
        control.refuse_sqlite_allocations_for_testonly();
        let (failed, wire) =
            tests::exchange(&owned, &tests::http_request(routes[1].0, &routes[1].1));
        let (status, _) = tests::http_parts(&wire);
        assert_eq!(status, 503);
        assert!(failed.is_ok());
        assert!(budget.ledger_health().sqlite_rejected_layouts > 0);
        assert_eq!(
            owned
                .executor
                .leases()
                .delivered_generation("runtime-client"),
            old_delivery
        );
        control.clear_refusal_for_testonly();
        let (retry, wire) =
            tests::exchange(&owned, &tests::http_request(routes[1].0, &routes[1].1));
        retry.unwrap();
        assert_eq!(tests::http_parts(&wire).0, 200);
        drop(owned);
        drop(normal);
        drop(projection);
        drop(fixture);
        drop(resources);
        // SAFETY: both runtime worker pools joined, all SQLite connections are dropped.
        unsafe { guard.finish() }.unwrap();
        let final_meter = budget.snapshot();
        let final_health = budget.ledger_health();
        println!(
            "REAL_Q_HTTP_OWNER_RECEIPT {}",
            serde_json::json!({"scope":"actual worker/TCP three Q routes in explicit named fixture; no production/whole audit/RSS/IO claim","before_fault":live,"final":final_meter,"health":final_health,"whole_audit_allocation_admitted":false,"production_capture_enabled":false,"formal9011_complete":false,"serde_escaped_string_scratch_owned_budget_qualified":false,"serde_diagnostic_error_box_owned_budget_qualified":false})
        );
        assert_eq!(final_meter.live_requested_layout_bytes, 0);
        assert_eq!(final_meter.allocated_layouts, final_meter.released_layouts);
        assert!(!final_health.invariant_broken);
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod admission_family_tests {
    use super::super::business_json as bj;
    use super::*;
    use newchan_rust::s_content::capture_allocation::CaptureSlab;
    use newchan_rust::session_protocol::{canonical_json, strict_json, validate_envelope_typed};
    use serde_json::{json, Value};
    use std::io::{self, Write};
    use std::time::Duration;

    fn fixed() -> Value {
        json!({"session_id":"session😀", "session_generation":"1"})
    }
    fn payloads() -> [(&'static str, Value); 3] {
        [
            (
                "/api/v3/snapshot",
                json!({"op":"snapshot","session_id":"session😀","session_generation":"1","scope":{},"history_mode":"AsKnown","as_of_generation":null,"page_size":"1","order_version":"1"}),
            ),
            (
                "/api/v3/content",
                json!({"op":"content","anchor":{"kind":"projection","fixed_cut":{"cut_generation":"1","history_mode":"AsKnown"},"cut_projection_digest":"unchecked","projection_ref":{"format":"unchecked","root":"unchecked","kind":"object","extent":"0"}},"path":[],"unit":"items","offset":"0","limit":"1"}),
            ),
            (
                "/api/v3/watch",
                json!({"op":"watch","cursor":{"session_id":"session😀","session_generation":"1","catalog_revision":"1","scope":{},"after_generation":"0","base_cut":"base","index_frontier":"","last_seq":"-1","order_version":"1"},"max_batches":"1","client_id":"client😀"}),
            ),
        ]
    }
    fn owner(bytes: &[u8], r: &QResources, b: Option<&OwnedBudget>) -> bj::SharedEnvelope {
        let owner = q_wire::decode_owner(bytes, r, b).unwrap_or_else(|e| panic!("{e}"));
        validate_envelope_typed(owner.view(), "s-session/3", b).unwrap();
        bj::SharedEnvelope::try_new(owner, b).unwrap_or_else(|(e, _)| panic!("{e}"))
    }
    struct Out {
        bytes: Vec<u8>,
        flushed: bool,
        fail_flush: bool,
    }
    impl Write for Out {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flushed = true;
            if self.fail_flush {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }
    impl q_watch::TimedWrite for Out {
        fn set_timeout(&mut self, _: Duration) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn admission_family_matrix_keeps_error_priority_unicode_dynamic_json_status_and_drop() {
        let r = tests::resources();
        let leases = ClientLeases::new(&r);
        let bases = payloads();
        // bad format/root are intentionally accepted transport declarations at this stage.
        for (path, payload) in &bases {
            let bytes = tests::envelope(&fixed(), payload.clone());
            let b = OwnedBudget::new(100_000);
            let source = owner(&bytes, &r, Some(&b));
            let request = parse_admitted(path, &source, bytes.len(), &r, Some(&b))
                .unwrap_or_else(|e| panic!("{e}"));
            let live = b.snapshot().live_requested_layout_bytes;
            drop(request);
            assert!(b.snapshot().live_requested_layout_bytes > 0);
            assert!(b.snapshot().live_requested_layout_bytes <= live);
            drop(source);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
        let mut cases = Vec::new();
        let mut snap = bases[0].1.clone();
        snap["op"] = json!("wrong");
        snap["extra"] = json!(true);
        cases.push((
            bases[0].0,
            snap,
            "QSnapshotInvalid：op必须是snapshot".to_owned(),
        ));
        let long = "中文😀é".repeat(40);
        let mut snap = bases[0].1.clone();
        snap["as_of_generation"] = json!(long);
        snap["history_mode"] = json!("wrong");
        cases.push((bases[0].0,snap,format!("as_of_generation `{long}` 不是规范十进制整数（要求唯一表示：无 +、无空白、无前导零）")));
        let mut watch = bases[2].1.clone();
        watch["op"] = json!("wrong");
        watch["extra"] = json!(true);
        cases.push((bases[2].0, watch, "QWatchInvalid：exact字段不符".to_owned()));
        let number = "CaptureDeadline😀prototype_audit_allocation_limit";
        let mut watch = bases[2].1.clone();
        watch["max_batches"] = json!(number);
        watch["client_id"] = json!("");
        cases.push((
            bases[2].0,
            watch,
            format!(
                "max_batches `{number}` 不是规范十进制整数（要求唯一表示：无 +、无空白、无前导零）"
            ),
        ));
        for key in ["长键😀", "Incomplete", "prototype_audit_allocation_limit"] {
            let mut content = bases[1].1.clone();
            content["anchor"]["projection_ref"][key] = json!(1);
            content["unit"] = json!("wrong");
            cases.push((
                bases[1].0,
                content,
                format!("QContentInvalid：ref_field: ContentRef 出现 exact 四字段之外的键 `{key}`"),
            ));
        }
        for kind in [
            "Unknown😀",
            "CaptureBytes",
            "prototype_audit_allocation_incomplete",
        ] {
            let mut content = bases[1].1.clone();
            content["anchor"]["projection_ref"]["kind"] = json!(kind);
            content["anchor"]["projection_ref"]["extent"] = json!({"also":"wrong"});
            cases.push((
                bases[1].0,
                content,
                format!("QContentInvalid：kind: 未知 kind `{kind}`"),
            ));
        }
        let mut token = json!({"token_schema":"s-snapshot-token/2","session_id":"session😀","session_generation":"1","cut_generation":"bad","scope":{},"history_mode":"bad","structure_cut":"","catalog_revision":"","index_frontier":"","profile_hash":"","rule_revision":"","order_version":"","page_size":"bad","offset":"bad","cut_projection_digest":"","issued_capture_digest":"0".repeat(64),"token_checksum":"wrong"});
        cases.push((
            bases[0].0,
            json!({"op":"snapshot","snapshot_token":token.clone()}),
            "QSnapshotInvalid：snapshot_token checksum不符".to_owned(),
        ));
        token["issued_capture_digest"] = json!("bad");
        cases.push((
            bases[0].0,
            json!({"op":"snapshot","snapshot_token":token}),
            "QSnapshotInvalid：签发捕获摘要不是64位小写hex".to_owned(),
        ));
        let token = json!({"token_schema":"s-content-token/1","anchor":{},"path":null,"target_ref":null,"unit":"bad","offset":"bad","limit":"bad","issued_capture_digest":"0".repeat(64),"token_checksum":"wrong"});
        cases.push((
            bases[1].0,
            json!({"op":"content","content_token":token}),
            "QContentInvalid：content_token checksum不符".to_owned(),
        ));
        let extent = json!({"z":["中文😀\"",{"Complete":"atomic_delta_limit"}],"a":false});
        let mut content = bases[1].1.clone();
        content["anchor"]["projection_ref"]["extent"] = extent.clone();
        content["path"] = json!([{"bad":true}]);
        cases.push((bases[1].0,content,format!("QContentInvalid：extent: extent 必须是规范十进制字符串或 null（U64 长度按 §5.2 用十进制字符串传输），实为 {}",canonical_json(&extent))));
        let mut content = bases[1].1.clone();
        content["path"] = json!([{"index":"18446744073709551616"}]);
        content["offset"] = json!("bad");
        cases.push((
            bases[1].0,
            content,
            "QContentInvalid：length_overflow: index `18446744073709551616` 超出 u64 表达"
                .to_owned(),
        ));
        for (path, payload, expected) in cases {
            let bytes = tests::envelope(&fixed(), payload);
            let budget = OwnedBudget::new(100_000);
            for allocation in [None, Some(&budget)] {
                let source = owner(&bytes, &r, allocation);
                let reason = match parse_admitted(path, &source, bytes.len(), &r, allocation) {
                    Ok(_) => panic!("negative admitted"),
                    Err(e) => e,
                };
                assert_eq!(reason.to_string(), expected);
                let mut out = Out {
                    bytes: Vec::new(),
                    flushed: false,
                    fail_flush: false,
                };
                send_admission_failure(
                    &mut out,
                    &r,
                    &leases,
                    "1",
                    Some(source.view()),
                    &reason,
                    allocation,
                )
                .unwrap();
                assert!(out.flushed);
                let (status, reply) = tests::http_parts(&out.bytes);
                assert_eq!(
                    status,
                    if q_wire::legacy_budget(&reason) {
                        503
                    } else {
                        400
                    }
                );
                assert_eq!(reply["payload"]["detail"], expected);
                assert_eq!(
                    reply["payload"]["identity_binding"],
                    "request_echo_unverified"
                );
                assert_eq!(reply["causal_refs"][0]["message_id"], "request");
                assert_eq!(reply.as_object().unwrap().len(), 11);
                assert_eq!(
                    newchan_rust::session_protocol::sha256_hex(
                        canonical_json(&reply["payload"]).as_bytes()
                    ),
                    reply["payload_hash"].as_str().unwrap()
                );
                out.fail_flush = true;
                assert!(send_admission_failure(
                    &mut out,
                    &r,
                    &leases,
                    "1",
                    Some(source.view()),
                    &reason,
                    allocation
                )
                .is_err());
                drop(reason);
                drop(source);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            }
        }
    }
    #[test]
    fn admission_family_hash_required_actual_allocator_contention_and_shared_owner_rejection() {
        let r = tests::resources();
        let (_, payload) = &payloads()[0];
        let bytes = tests::envelope(&fixed(), payload.clone());
        let budget = OwnedBudget::new(100_000);
        let source =
            q_wire::decode_owner(&bytes, &r, Some(&budget)).unwrap_or_else(|e| panic!("{e}"));
        let remaining =
            budget.snapshot().limit_bytes - budget.snapshot().live_requested_layout_bytes;
        let occupied = CaptureSlab::new(0, remaining, &budget).unwrap();
        let error =
            validate_envelope_typed(source.view(), "s-session/3", Some(&budget)).unwrap_err();
        assert!(matches!(
            error,
            newchan_rust::session_protocol::ProtocolError::Allocation(_)
        ));
        let (error, source) = match bj::SharedEnvelope::try_new(source, Some(&budget)) {
            Ok(_) => panic!("shared allocation bypassed budget"),
            Err(pair) => pair,
        };
        assert_eq!(
            error.code(),
            newchan_rust::s_content::capture_allocation::LIMIT
        );
        assert_eq!(
            bj::field(source.view(), "message_id").as_str(),
            Some("request")
        );
        drop(source);
        drop(occupied);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let mut value = strict_json(&bytes).unwrap();
        value["payload_hash"] = Value::Null;
        value["payload"].as_object_mut().unwrap().remove("op");
        let raw = canonical_json(&value);
        let source = q_wire::decode_owner(raw.as_bytes(), &r, Some(&budget))
            .unwrap_or_else(|e| panic!("{e}"));
        let remaining =
            budget.snapshot().limit_bytes - budget.snapshot().live_requested_layout_bytes;
        let occupied = CaptureSlab::new(0, remaining, &budget).unwrap();
        let attempts = budget.snapshot().allocation_attempts;
        let reason =
            validate_envelope_typed(source.view(), "s-session/3", Some(&budget)).unwrap_err();
        assert_eq!(
            reason.to_string(),
            "SchemaUnsupported：缺非空字符串 payload_hash"
        );
        assert_eq!(budget.snapshot().allocation_attempts, attempts);
        drop(reason);
        drop(source);
        drop(occupied);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        // Client copying still precedes the Watch resource-limit rejection.
        let (_, mut payload) = payloads()[2].clone();
        payload["max_batches"] = json!("9223372036854775807");
        let bytes = tests::envelope(&fixed(), payload);
        let source = owner(&bytes, &r, Some(&budget));
        let remaining =
            budget.snapshot().limit_bytes - budget.snapshot().live_requested_layout_bytes;
        let occupied = CaptureSlab::new(0, remaining, &budget).unwrap();
        let reason = match parse_admitted("/api/v3/watch", &source, bytes.len(), &r, Some(&budget))
        {
            Ok(_) => panic!("client allocation bypassed budget"),
            Err(e) => e,
        };
        assert!(matches!(reason, q_wire::AdmissionError::Allocation(_)));
        drop(reason);
        drop(source);
        drop(occupied);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn admission_family_common_envelope_first_error_matrix_has_no_causal_echo() {
        let r = tests::resources();
        let leases = ClientLeases::new(&r);
        let bytes = tests::envelope(&fixed(), payloads()[0].1.clone());
        let base = strict_json(&bytes).unwrap();
        let mut cases = Vec::new();
        let mut v = base.clone();
        v["extra"] = json!(true);
        v["schema_revision"] = json!("wrong");
        cases.push((v, "SchemaUnsupported：公共头字段不完整/多余"));
        let mut v = base.clone();
        v["schema_revision"] = json!("wrong");
        v["session_id"] = json!("");
        cases.push((v, "SchemaUnsupported：仅s-session/3"));
        let mut v = base.clone();
        v["session_id"] = json!("");
        v["session_generation"] = json!("bad");
        cases.push((v, "SchemaUnsupported：缺非空字符串 session_id"));
        let mut v = base.clone();
        v["session_generation"] = json!("0");
        v["producer_epoch"] = json!("bad");
        v["causal_refs"] = Value::Null;
        cases.push((v, "InvalidDomain：session_generation 必须大于0"));
        let mut v = base.clone();
        v["causal_refs"] =
            json!([{"source_namespace":"","source_epoch":"","message_id":"","payload_hash":""}]);
        v["payload"] = Value::Null;
        cases.push((v, "SchemaUnsupported：缺非空字符串 source_namespace"));
        let mut v = base.clone();
        v["payload"] = Value::Null;
        v["payload_hash"] = Value::Null;
        cases.push((v, "SchemaUnsupported：payload必须对象"));
        let mut v = base.clone();
        v["payload"].as_object_mut().unwrap().remove("op");
        v["payload_hash"] = json!("wrong");
        cases.push((v, "IdentityConflict：payload_hash与规范业务内容不同"));
        for (value, expected) in cases {
            let raw = canonical_json(&value);
            let b = OwnedBudget::new(100_000);
            for allocation in [None, Some(&b)] {
                let source = q_wire::decode_owner(raw.as_bytes(), &r, allocation)
                    .unwrap_or_else(|e| panic!("{e}"));
                let reason =
                    validate_envelope_typed(source.view(), "s-session/3", allocation).unwrap_err();
                assert_eq!(reason.to_string(), expected);
                let reason = q_wire::AdmissionError::Protocol(reason);
                let mut out = Out {
                    bytes: Vec::new(),
                    flushed: false,
                    fail_flush: false,
                };
                send_admission_failure(&mut out, &r, &leases, "1", None, &reason, allocation)
                    .unwrap();
                let (status, reply) = tests::http_parts(&out.bytes);
                assert_eq!(status, 400);
                assert_eq!(reply["detail"], expected);
                assert!(reply.get("causal_refs").is_none());
                assert!(out.flushed);
                drop(reason);
                drop(source);
                assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
            }
        }
    }
}
