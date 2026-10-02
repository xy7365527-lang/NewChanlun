//! #1394 C §5.5/5.6.1：Watch/Gap、完整Delta门及发送后租约推进；普通/Owned共借用核。
use super::{
    audit_store::{CodecRead, RefValue},
    business_json::{self as bj, JsonOwner, TextValue},
    history::{self, PublishedCut},
    projection::{Projection, TransportView},
    q_delta,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
    q_snapshot, q_wire,
};
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::s_content::{
    self as sc,
    access::{Access, MeteredSource},
    capture_allocation::{
        audit_json::{self as aj, JsonView},
        OwnedBudget,
    },
    path::{lookup_path_refview, PathStep},
    CodecVec, NodeSource,
};
use newchan_rust::session_protocol::{
    nonnegative, positive, required_view, strict_json, validate_envelope_view_with_owner,
};
#[cfg(test)]
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
    envelope: JsonOwner,
    client: TextValue,
    after: i64,
    maximum: u64,
    bytes: usize,
}
impl Request {
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        Self::parse_owner(
            JsonOwner::Plain(q_wire::parse(bytes, resources)?),
            bytes.len(),
            resources,
            None,
        )
    }
    pub(super) fn parse_owned(
        bytes: &[u8],
        resources: &QResources,
        budget: &OwnedBudget,
    ) -> Result<Self, String> {
        if bytes.len() as u128 > u128::from(resources.limit(Limit::FrameBytes)) {
            return Err("QReplyInvalid：Incomplete：完整请求超过max_frame_bytes".into());
        }
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            let envelope = aj::OwnedJson::parse(bytes, budget).map_err(err)?;
            validate_envelope_view_with_owner(envelope.view(), "s-session/3", Some(budget))?;
            return Self::parse_owner(
                JsonOwner::Owned(envelope),
                bytes.len(),
                resources,
                Some(budget),
            );
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = budget;
            Err("prototype_audit_allocation_incomplete：owned Watch请求未启用".into())
        }
    }
    fn parse_owner(
        envelope: JsonOwner,
        bytes: usize,
        resources: &QResources,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        let payload = bj::field(envelope.view(), "payload");
        exact(payload, &["op", "cursor", "max_batches", "client_id"])?;
        if bj::field(payload, "op").as_str() != Some("watch") {
            return Err(err("op必须是watch"));
        }
        let cursor = bj::field(payload, "cursor");
        exact(cursor, &CURSOR_FIELDS)?;
        required_view(cursor, "session_id")?;
        positive(
            required_view(cursor, "session_generation")?,
            "session_generation",
        )?;
        for key in ["session_id", "session_generation"] {
            if bj::field(envelope.view(), key) != bj::field(cursor, key) {
                return Err(err("外层消息与cursor身份不同"));
            }
        }
        let after = nonnegative(
            required_view(cursor, "after_generation")?,
            "after_generation",
        )?;
        let maximum = positive(required_view(payload, "max_batches")?, "max_batches")? as u64;
        let client = TextValue::from_str(required_view(payload, "client_id")?, allocation)?;
        let request = Self {
            envelope,
            client,
            after,
            maximum,
            bytes,
        };
        request.validate_resources(resources)?;
        Ok(request)
    }
    pub(super) fn envelope_view(&self) -> JsonView<'_> {
        self.envelope.view()
    }
    pub(super) fn cursor_view(&self) -> JsonView<'_> {
        bj::field(bj::field(self.envelope.view(), "payload"), "cursor")
    }
    pub(super) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        match &self.envelope {
            JsonOwner::Plain(_) => None,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(v) => Some(v.budget()),
        }
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
    pub revision: TextValue,
    pub retain: i64,
    pub first: i64,
    pub head: i64,
}
pub(super) fn cursor_at_with_budget(
    current: &Projection,
    cuts: &[PublishedCut],
    generation: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<JsonOwner, String> {
    let fixed = bj::field(current.wire.view(), "fixed_cut");
    let last_decimal;
    let (base, index, last) = if generation == 0 {
        ("cut-0", "", "-1")
    } else {
        let cut = cuts
            .iter()
            .find(|c| c.metadata.generation == generation)
            .ok_or_else(|| err("cursor代号未发布"))?;
        last_decimal = bj::Decimal::new(cut.metadata.input_frontier);
        (
            cut.metadata.structure_cut.as_str(),
            cut.batch_id.as_str(),
            last_decimal.text(),
        )
    };
    let generation = bj::Decimal::new(generation);
    let fields = [
        ("session_id", bj::field(fixed, "session_id")),
        ("session_generation", bj::field(fixed, "session_generation")),
        ("catalog_revision", bj::field(fixed, "catalog_revision")),
        ("scope", bj::field(fixed, "scope")),
        ("after_generation", JsonView::String(generation.text())),
        ("base_cut", JsonView::String(base)),
        ("index_frontier", JsonView::String(index)),
        ("last_seq", JsonView::String(last)),
        (
            "order_version",
            bj::field(current.wire.view(), "order_version"),
        ),
    ];
    bj::json_from_view(JsonView::object_fields(&fields).map_err(err)?, allocation)
}
pub(super) fn cursor_at(
    current: &Projection,
    cuts: &[PublishedCut],
    generation: i64,
) -> Result<serde_json::Value, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if matches!(current.wire, JsonOwner::Owned(_)) {
        return Err(err("旧cursor_at仅支持None投影"));
    }
    match cursor_at_with_budget(current, cuts, generation, None)? {
        JsonOwner::Plain(v) => Ok(v),
        #[cfg(feature = "s_allocation_json_testonly")]
        JsonOwner::Owned(_) => Err(err("None cursor不能擦除Owned")),
    }
}
enum Source<'a, 'b> {
    Plain(&'a mut dyn NodeSource),
    Actual(&'a mut history::HashReader<'b>),
}
impl Source<'_, '_> {
    fn allocation(&self) -> Option<OwnedBudget> {
        match self {
            Self::Plain(_) => None,
            Self::Actual(g) => g.allocation_budget(),
        }
    }
    fn measure(
        &mut self,
        cut: &PublishedCut,
        resources: &QResources,
        job: &mut dyn Access,
    ) -> Result<(), String> {
        match self {
            Self::Plain(s) => {
                q_delta::measure(*s, &cut.delta, resources, job)?;
            }
            Self::Actual(g) => {
                let owner = g
                    .closed_owner_mut()
                    .map_err(err)?
                    .ok_or_else(|| err("owned Watch缺实际Closed源"))?;
                q_delta::measure_closed(owner, cut.delta.view(), resources, job)?;
            }
        }
        Ok(())
    }
    fn delta_wire(
        &mut self,
        cut: &PublishedCut,
        job: &mut dyn Access,
        allocation: Option<&OwnedBudget>,
    ) -> Result<JsonOwner, String> {
        let mut values = bj::new_work(allocation);
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
            let value = match self {
                Self::Plain(s) => {
                    let mut source = MeteredSource::new(job, *s);
                    let field = lookup_path_refview(
                        &mut source,
                        cut.delta.view(),
                        &[PathStep::Field(key.into())],
                    )
                    .map_err(err)?;
                    if key == "catalog_evidence" {
                        bj::json_from_view(TransportView::new((&field).into()).view(), None)?
                    } else {
                        let mut bytes = Vec::new();
                        sc::render_canonical(&mut source, &field, &mut bytes).map_err(err)?;
                        JsonOwner::Plain(strict_json(&bytes)?)
                    }
                }
                Self::Actual(g) => {
                    let mut access = WatchAccess { job, active: false };
                    let field = g.q_lookup_field(cut.delta.view(), key, &mut access);
                    let field = access.after(field).map_err(err)?;
                    if key == "catalog_evidence" {
                        bj::json_from_view(TransportView::new(field.view()).view(), allocation)?
                    } else {
                        let mut output = CodecBuffer {
                            bytes: bj::new_work(allocation),
                            cause: None,
                        };
                        let result = g.q_render_ref(field.view(), &mut output, &mut access);
                        let result = access.after(result);
                        if let Some(e) = output.cause {
                            return Err(err(e));
                        }
                        result.map_err(err)?;
                        decode(&output.bytes, allocation)?
                    }
                }
            };
            values.try_push((key, value)).map_err(err)?;
        }
        let transport = TransportView::new(cut.delta.view());
        let mut fields = bj::new_work(allocation);
        for (key, value) in &values {
            fields.try_push((*key, value.view())).map_err(err)?;
        }
        fields.try_push(("delta", transport.view())).map_err(err)?;
        bj::json_from_view(JsonView::object_fields(&fields).map_err(err)?, allocation)
    }
}
struct WatchAccess<'a> {
    job: &'a mut dyn Access,
    active: bool,
}
impl WatchAccess<'_> {
    fn after<T>(&mut self, result: Result<T, sc::CodecError>) -> Result<T, sc::CodecError> {
        if self.active
            && result
                .as_ref()
                .is_err_and(|e| e.code() == sc::code::NODE_MISSING)
        {
            self.active = false;
            self.job.pre_io_check()?;
        }
        result
    }
}
impl Access for WatchAccess<'_> {
    fn owner_job(&mut self) -> Option<&mut sc::job::Job> {
        self.job.owner_job()
    }
    fn pre_io_check(&mut self) -> Result<(), sc::CodecError> {
        if self.active {
            self.active = false;
            self.job.pre_io_check()
        } else {
            self.active = true;
            self.job.begin_first_read()
        }
    }
    fn accept_node(
        &mut self,
        d: &sc::NodeDigest,
        t: u8,
        b: &[u8],
    ) -> Result<sc::job::NodeAcceptance, sc::CodecError> {
        self.job.accept_node(d, t, b)
    }
}
struct CodecBuffer {
    bytes: CodecVec<u8>,
    cause: Option<sc::CodecError>,
}
impl Write for CodecBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.bytes.try_extend_bytes(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(e) => {
                self.cause = Some(e);
                Err(io::ErrorKind::Other.into())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn decode(bytes: &[u8], allocation: Option<&OwnedBudget>) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = allocation {
        return aj::OwnedJson::parse(bytes, b)
            .map(JsonOwner::Owned)
            .map_err(err);
    }
    if allocation.is_some() {
        return Err(err("owned Watch JSON未启用"));
    }
    strict_json(bytes).map(JsonOwner::Plain)
}
struct Candidate {
    wire: JsonOwner,
    cursor: JsonOwner,
    generation: i64,
}
struct Deltas<'a>(&'a [Candidate]);
impl aj::JsonArray for Deltas<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, i: usize) -> Option<JsonView<'_>> {
        self.0.get(i).map(|c| c.wire.view())
    }
}
struct Payload<'a> {
    deltas: &'a dyn aj::JsonArray,
    cursor: JsonView<'a>,
    fixed: JsonView<'a>,
    frontier: JsonView<'a>,
    has_more: bool,
    gap: JsonView<'a>,
    digest: &'a str,
}
impl Payload<'_> {
    fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("Watch八固定字段唯一")
    }
}
impl aj::JsonObject for Payload<'_> {
    fn len(&self) -> usize {
        8
    }
    fn field(&self, i: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match i {
            0 => ("deltas", JsonView::BorrowedArray(self.deltas)),
            1 => ("next_cursor", self.cursor),
            2 => ("observed_head", self.fixed),
            3 => ("delivery_frontier", self.frontier),
            4 => ("has_more", JsonView::Bool(self.has_more)),
            5 => ("gap", self.gap),
            6 => ("validated_capture_digest", JsonView::String(self.digest)),
            7 => ("ok", JsonView::Bool(true)),
            _ => return None,
        })
    }
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
    job: &mut dyn Access,
) -> Result<Reply, String> {
    prepare_core(
        request,
        current,
        cuts,
        Source::Plain(source),
        policy,
        capture_digest,
        resources,
        leases,
        epoch,
        job,
    )
}
pub(super) fn prepare_owned(
    request: &Request,
    current: Projection,
    cuts: &[PublishedCut],
    source: &mut history::HashReader<'_>,
    policy: Policy,
    capture_digest: &str,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    job: &mut dyn Access,
) -> Result<Reply, String> {
    if source.allocation_budget().is_none() {
        return Err(err("owned Watch缺实际预算"));
    }
    prepare_core(
        request,
        current,
        cuts,
        Source::Actual(source),
        policy,
        capture_digest,
        resources,
        leases,
        epoch,
        job,
    )
}
fn prepare_core(
    request: &Request,
    current: Projection,
    cuts: &[PublishedCut],
    mut source: Source<'_, '_>,
    policy: Policy,
    capture_digest: &str,
    resources: &QResources,
    leases: &ClientLeases,
    epoch: &str,
    job: &mut dyn Access,
) -> Result<Reply, String> {
    request.validate_resources(resources)?;
    if leases.resource_digest() != resources.digest() {
        return Err(err("租约与当前读取不是同一冻结资源配置"));
    }
    let allocation = source.allocation();
    let fixed = bj::field(current.wire.view(), "fixed_cut");
    let session = TextValue::from_str(required_view(fixed, "session_id")?, allocation.as_ref())?;
    let incarnation = TextValue::from_str(
        required_view(fixed, "session_generation")?,
        allocation.as_ref(),
    )?;
    let head = nonnegative(required_view(fixed, "cut_generation")?, "cut_generation")?;
    if policy.head != head
        || policy.first < 1
        || policy.retain < 1
        || policy.first > head.saturating_add(1)
    {
        return Err(err("已核交付策略与head不一致"));
    }
    if let Some(b) = allocation.as_ref() {
        if !request
            .allocation_budget()
            .is_some_and(|a| a.same_ledger(b))
            || !leases.allocation_budget().is_some_and(|a| a.same_ledger(b))
        {
            return Err(err(
                "prototype_audit_allocation_incomplete：Watch请求/lease与实际捕获不是同账",
            ));
        }
    }
    job.begin_first_read().map_err(err)?;
    let retain = bj::Decimal::new(policy.retain);
    let first = bj::Decimal::new(policy.first);
    let policy_head = bj::Decimal::new(policy.head);
    let frontier_fields = [
        ("policy_revision", JsonView::String(&policy.revision)),
        ("retain_generations", JsonView::String(retain.text())),
        ("first_available_generation", JsonView::String(first.text())),
        ("head_generation", JsonView::String(policy_head.text())),
    ];
    let frontier = JsonView::object_fields(&frontier_fields).map_err(err)?;
    let cursor = request.cursor_view();
    let same = bj::field(cursor, "session_id").as_str() == Some(session.as_str())
        && bj::field(cursor, "session_generation").as_str() == Some(incarnation.as_str());
    let (reason, missing_end) = if !same {
        (Some("session_identity_changed"), None)
    } else if request.after > head {
        return Err(err("同会话cursor超出永久发布历史"));
    } else {
        let actual = cursor_at_with_budget(&current, cuts, request.after, allocation.as_ref())?;
        if cursor != actual.view() {
            return Err(err(
                "cursor base/index/seq/catalog/scope/order与实际切面不符",
            ));
        }
        if request.after < policy.first - 1 {
            (Some("delivery_retention_gap"), Some(policy.first - 1))
        } else if leases.observe(
            &request.client,
            (&session, &incarnation),
            request.after,
            head,
            Instant::now(),
        )? {
            (Some("client_backlog_overflow"), Some(head))
        } else {
            (None, None)
        }
    };
    let empty = Deltas(&[]);
    if let Some(reason) = reason {
        let from = bj::Decimal::new(if missing_end.is_some() {
            request.after + 1
        } else {
            0
        });
        let to = missing_end.map(bj::Decimal::new);
        let missing_fields = [
            ("from_generation", JsonView::String(from.text())),
            (
                "to_generation",
                to.as_ref()
                    .map_or(JsonView::Null, |v| JsonView::String(v.text())),
            ),
        ];
        let missing = if missing_end.is_some() {
            JsonView::object_fields(&missing_fields).map_err(err)?
        } else {
            JsonView::Null
        };
        let token = q_snapshot::token_with_budget(
            &current,
            capture_digest,
            resources.limit(Limit::PageSize),
            0,
            allocation.as_ref(),
        )?;
        let gap_fields = [
            ("requested_identity", cursor),
            ("reason", JsonView::String(reason)),
            ("missing_range", missing),
            ("rebuild", fixed),
            ("snapshot_token", token.view()),
        ];
        let gap = JsonView::object_fields(&gap_fields).map_err(err)?;
        let payload = Payload {
            deltas: &empty,
            cursor,
            fixed,
            frontier,
            has_more: false,
            gap,
            digest: capture_digest,
        };
        let body = q_wire::reply_view(
            request.envelope_view(),
            payload.view(),
            &session,
            &incarnation,
            epoch,
            resources,
            job,
            allocation.as_ref(),
        )?;
        return Reply::new(body, None, resources, allocation.as_ref());
    }
    // 与旧路径相同，所有候选先过完整Delta三门，再尝试任何完整回复前缀。
    let mut candidates = bj::new_work(allocation.as_ref());
    for cut in cuts
        .iter()
        .filter(|c| c.metadata.generation > request.after)
        .take(request.maximum as usize)
    {
        source.measure(cut, resources, job)?;
        let wire = source.delta_wire(cut, job, allocation.as_ref())?;
        let cursor =
            cursor_at_with_budget(&current, cuts, cut.metadata.generation, allocation.as_ref())?;
        candidates
            .try_push(Candidate {
                wire,
                cursor,
                generation: cut.metadata.generation,
            })
            .map_err(err)?;
    }
    let mut accepted = None;
    let mut through = request.after;
    if candidates.is_empty() {
        let payload = Payload {
            deltas: &empty,
            cursor,
            fixed,
            frontier,
            has_more: false,
            gap: JsonView::Null,
            digest: capture_digest,
        };
        accepted = Some(q_wire::reply_view(
            request.envelope_view(),
            payload.view(),
            &session,
            &incarnation,
            epoch,
            resources,
            job,
            allocation.as_ref(),
        )?);
    } else {
        for (i, candidate) in candidates.iter().enumerate() {
            let deltas = Deltas(&candidates[..=i]);
            let payload = Payload {
                deltas: &deltas,
                cursor: candidate.cursor.view(),
                fixed,
                frontier,
                has_more: candidate.generation < head,
                gap: JsonView::Null,
                digest: capture_digest,
            };
            match q_wire::try_reply_view(
                request.envelope_view(),
                payload.view(),
                &session,
                &incarnation,
                epoch,
                resources,
                job,
                allocation.as_ref(),
            )? {
                Some(body) => {
                    accepted = Some(body);
                    through = candidate.generation;
                }
                None if accepted.is_none() => {
                    return Err(err("Incomplete：首批完整Watch消息体超过max_reply_bytes"))
                }
                None => break,
            }
        }
    }
    job.pre_io_check().map_err(err)?;
    Reply::new(
        accepted.ok_or_else(|| err("无完整Watch回复"))?,
        Some(Delivery {
            client: TextValue::from_str(&request.client, allocation.as_ref())?,
            session,
            incarnation,
            after: through,
        }),
        resources,
        allocation.as_ref(),
    )
}
struct Delivery {
    client: TextValue,
    session: TextValue,
    incarnation: TextValue,
    after: i64,
}
pub(super) struct Reply {
    body: CodecVec<u8>,
    delivery: Option<Delivery>,
    resource_digest: TextValue,
    write_ms: u64,
}
impl Reply {
    fn new(
        body: CodecVec<u8>,
        delivery: Option<Delivery>,
        resources: &QResources,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        Ok(Self {
            body,
            delivery,
            resource_digest: TextValue::from_str(resources.digest(), allocation)?,
            write_ms: resources.limit(Limit::SocketWriteMs),
        })
    }
    pub(super) fn plain(body: Vec<u8>, resources: &QResources) -> Result<Self, String> {
        if body.len() as u128 > u128::from(resources.limit(Limit::ReplyBytes)) {
            return Err(err("完整Q回复超过max_reply_bytes"));
        }
        Self::new(CodecVec::from_plain(body), None, resources, None)
    }
    pub(super) fn owned(
        body: CodecVec<u8>,
        resources: &QResources,
        allocation: &OwnedBudget,
    ) -> Result<Self, String> {
        if body.len() as u128 > u128::from(resources.limit(Limit::ReplyBytes)) {
            return Err(err("完整Q回复超过max_reply_bytes"));
        }
        if !body
            .allocation_budget()
            .is_some_and(|b| b.same_ledger(allocation))
        {
            return Err(err("owned回复body与原Profile预算不同"));
        }
        Self::new(body, None, resources, Some(allocation))
    }
    pub(super) fn body(&self) -> &[u8] {
        &self.body
    }
    /// 唯一送达入口：实际写入量逐次计账；完整body及flush成功且未过期才推进。
    pub(super) fn send_body<W: TimedWrite>(
        self,
        writer: &mut W,
        leases: &ClientLeases,
        job: &mut dyn Access,
    ) -> Result<(), String> {
        if leases.resource_digest() != self.resource_digest.as_str() {
            return Err(err("送达租约与编码时资源不同"));
        }
        let local = Instant::now()
            .checked_add(Duration::from_millis(self.write_ms))
            .ok_or_else(|| err("写期限溢出"))?;
        let deadline = job
            .read_deadline()
            .map_err(err)?
            .map_or(local, |d| local.min(d));
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
fn exact(value: JsonView<'_>, fields: &[&str]) -> Result<(), String> {
    if value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k)))
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
    fn plain_wire(p: &Projection) -> &Value {
        match &p.wire {
            JsonOwner::Plain(v) => v,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(_) => panic!("普通oracle仅Plain"),
        }
    }
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
        let c = &plain_wire(&p)["fixed_cut"];
        json!({"session_id":c["session_id"],"session_generation":c["session_generation"],"catalog_revision":c["catalog_revision"],"scope":c["scope"],
            "after_generation":g.to_string(),"base_cut":c["structure_cut"],"index_frontier":c["index_frontier"],"last_seq":c["input_frontier"],"order_version":plain_wire(&p)["order_version"]})
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
        job: &mut dyn Access,
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
    fn send(reply: Reply, leases: &ClientLeases, job: &mut dyn Access) -> Vec<u8> {
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
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_watch_two_input_full_delta_reply_and_delivered_lease_match_plain_and_drop() {
        use super::super::{capture::AuditReader, q_allocation::Profile};
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let r = q_resources::tests::resources();
        let c = cursor(&f, &r, 0);
        let normal_leases = ClientLeases::new(&r);
        let normal_request = request(&c, 2, "owner-watch", &r);
        let expected = capture_configured(&f.db_path(), &r, &mut job())
            .unwrap()
            .watch(&normal_request, &normal_leases, "4", &mut job())
            .unwrap();
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let leases = ClientLeases::with_allocation(&r, Some(&budget)).unwrap();
        let mut reader = AuditReader::new_with_allocation(
            f.db_path(),
            &r,
            Some(Profile {
                budget: budget.clone(),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            }),
        );
        let bytes = canonical_json(&envelope(
            &c,
            json!({"op":"watch","cursor":c,"max_batches":"2","client_id":"owner-watch"}),
        ))
        .into_bytes();
        let owned_request = Request::parse_owned(&bytes, &r, &budget).unwrap();
        let actual = reader
            .with_capture(&r, &mut job(), |capture, job| {
                capture.watch(&owned_request, &leases, "4", job)
            })
            .unwrap();
        assert_eq!(actual.body(), expected.body());
        assert_eq!(leases.delivered_generation("owner-watch"), Some(0));
        let (mut tx, mut rx) = UnixStream::pair().unwrap();
        actual.send_body(&mut tx, &leases, &mut job()).unwrap();
        tx.shutdown(Shutdown::Write).unwrap();
        let mut delivered = Vec::new();
        rx.read_to_end(&mut delivered).unwrap();
        assert_eq!(delivered, expected.body());
        assert_eq!(leases.delivered_generation("owner-watch"), Some(2));
        drop(owned_request);
        drop(reader);
        drop(leases);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
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
