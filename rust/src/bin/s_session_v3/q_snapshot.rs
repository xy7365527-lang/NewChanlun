//! #1394 C §5.5：固定切面分页与snapshot-token/2；普通/Owned共享完整借用判据。
use super::q_wire::{AdmissionError, RouteFixed};
use super::{
    business_json::{self as bj, JsonOwner, TextValue},
    projection::{HistoryMode, Projection},
    q_resources::{Limit, QResources},
};
use newchan_rust::s_content::{
    self as sc,
    access::Access,
    capture_allocation::{
        audit_json::{self as aj, JsonView},
        OwnedBudget,
    },
    CodecVec,
};
use newchan_rust::session_protocol::{
    canonical_hash_view_typed, nonnegative_typed, positive_typed, required_view_typed,
};
use newchan_rust::session_protocol::{canonical_hash_view_with_owner, positive, required_view};
#[cfg(test)]
use newchan_rust::{
    s_content::job::Job,
    session_protocol::{canonical_json, sha256_hex, strict_json},
};
#[cfg(test)]
use serde_json::{json, Value};

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
    envelope: bj::SharedEnvelope,
    bytes: usize,
    generation: Option<i64>,
    mode: HistoryMode,
    page_size: u64,
    offset: u64,
    issued: Option<TextValue>,
}
impl Request {
    #[cfg(test)]
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        Self::parse_with_allocation(bytes, resources, None)
    }
    #[cfg_attr(not(all(test, feature = "s_capture_native_testonly")), allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用"))]
    pub(super) fn parse_owned(
        bytes: &[u8],
        resources: &QResources,
        budget: &OwnedBudget,
    ) -> Result<Self, String> {
        Self::parse_with_allocation(bytes, resources, Some(budget))
    }
    #[cfg(test)]
    fn parse_with_allocation(
        bytes: &[u8],
        resources: &QResources,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        let envelope = super::q_wire::parse_owner(bytes, resources, allocation)?;
        let envelope =
            bj::SharedEnvelope::try_new(envelope, allocation).map_err(|(e, _)| e.to_string())?;
        Self::parse_admitted(&envelope, bytes.len(), resources, allocation)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn parse_admitted<'a>(
        envelope: &'a bj::SharedEnvelope,
        bytes: usize,
        resources: &QResources,
        _allocation: Option<&OwnedBudget>,
    ) -> Result<Self, AdmissionError<'a>> {
        let allocation = envelope.budget();
        let payload = bj::field(envelope.view(), "payload");
        if bj::field(payload, "op").as_str() != Some("snapshot") {
            return Err(AdmissionError::Fixed(RouteFixed::SnapshotOp));
        }
        let (generation, mode, page_size, offset, issued) =
            if payload.get("snapshot_token").is_some() {
                exact_typed(payload, &["op", "snapshot_token"])?;
                let token = bj::field(payload, "snapshot_token");
                exact_typed(token, &TOKEN_FIELDS)?;
                if bj::field(token, "token_schema").as_str() != Some("s-snapshot-token/2") {
                    return Err(AdmissionError::Fixed(RouteFixed::SnapshotVersion));
                }
                let issued = required_view_typed(token, "issued_capture_digest")?;
                if issued.len() != 64
                    || !issued
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(AdmissionError::Fixed(RouteFixed::SnapshotDigest));
                }
                let mut unsigned_fields = [("", JsonView::Null); 16];
                for (i, key) in TOKEN_FIELDS
                    .iter()
                    .filter(|k| **k != "token_checksum")
                    .enumerate()
                {
                    unsigned_fields[i] = (*key, bj::field(token, key));
                }
                let unsigned = JsonView::object_fields(&unsigned_fields)
                    .map_err(AdmissionError::SnapshotFields)?;
                let checksum = canonical_hash_view_typed(unsigned, allocation)?;
                if bj::field(token, "token_checksum").as_str() != Some(checksum.as_str()) {
                    return Err(AdmissionError::Fixed(RouteFixed::SnapshotChecksum));
                }
                (
                    Some(nonnegative_typed(
                        required_view_typed(token, "cut_generation")?,
                        "cut_generation",
                    )?),
                    mode_typed(required_view_typed(token, "history_mode")?)?,
                    positive_typed(required_view_typed(token, "page_size")?, "page_size")? as u64,
                    nonnegative_typed(required_view_typed(token, "offset")?, "offset")? as u64,
                    Some(
                        TextValue::from_str_typed(issued, allocation)
                            .map_err(AdmissionError::Allocation)?,
                    ),
                )
            } else {
                exact_typed(payload, &FIRST_FIELDS)?;
                let generation = if bj::field(payload, "as_of_generation").is_null() {
                    None
                } else {
                    Some(nonnegative_typed(
                        required_view_typed(payload, "as_of_generation")?,
                        "as_of_generation",
                    )?)
                };
                (
                    generation,
                    mode_typed(required_view_typed(payload, "history_mode")?)?,
                    positive_typed(required_view_typed(payload, "page_size")?, "page_size")? as u64,
                    0,
                    None,
                )
            };
        let request = Self {
            envelope: envelope.clone(),
            bytes,
            generation,
            mode,
            page_size,
            offset,
            issued,
        };
        request.validate_resources_typed(resources)?;
        Ok(request)
    }
    pub(super) fn envelope_view(&self) -> JsonView<'_> {
        self.envelope.view()
    }
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub(super) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        match &*self.envelope {
            JsonOwner::Plain(_) => None,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(v) => Some(v.budget()),
        }
    }
    pub(super) fn validate_resources(&self, resources: &QResources) -> Result<(), String> {
        self.validate_resources_typed(resources)
            .map_err(|e| e.to_string())
    }
    fn validate_resources_typed(
        &self,
        resources: &QResources,
    ) -> Result<(), AdmissionError<'static>> {
        if self.bytes as u128 > u128::from(resources.limit(Limit::FrameBytes))
            || self.page_size > resources.limit(Limit::PageSize)
        {
            return Err(AdmissionError::Fixed(RouteFixed::SnapshotLimit));
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
#[allow(dead_code, reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删")]
fn mode(value: &str) -> Result<HistoryMode, String> {
    mode_typed(value).map_err(|e| e.to_string())
}
fn mode_typed(value: &str) -> Result<HistoryMode, AdmissionError<'static>> {
    match value {
        "AsKnown" => Ok(HistoryMode::AsKnown),
        "RecomputedWithRevision" => Ok(HistoryMode::RecomputedWithRevision),
        _ => Err(AdmissionError::Fixed(RouteFixed::SnapshotMode)),
    }
}

#[allow(dead_code, reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删")]
fn exact(value: JsonView<'_>, fields: &[&str]) -> Result<(), String> {
    exact_typed(value, fields).map_err(|e| e.to_string())
}
fn exact_typed(value: JsonView<'_>, fields: &[&str]) -> Result<(), AdmissionError<'static>> {
    if value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k)))
    {
        Ok(())
    } else {
        Err(AdmissionError::Fixed(RouteFixed::SnapshotExact))
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QSnapshotInvalid：{e}")
}
// page_size/offset沿旧u64字符串域，不能转i64把理论上界截窄。
struct Decimal {
    bytes: [u8; 20],
    start: usize,
}
impl Decimal {
    fn new(mut n: u64) -> Self {
        let mut bytes = [0; 20];
        let mut start = 20;
        loop {
            start -= 1;
            bytes[start] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        Self { bytes, start }
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[self.start..]).unwrap()
    }
}
pub(super) fn token_with_budget(
    projection: &Projection,
    issued: &str,
    page_size: u64,
    offset: u64,
    allocation: Option<&OwnedBudget>,
) -> Result<JsonOwner, String> {
    let fixed = bj::field(projection.wire.view(), "fixed_cut");
    let page = Decimal::new(page_size);
    let offset = Decimal::new(offset);
    let mut unsigned = [("", JsonView::Null); 16];
    for (i, key) in [
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
    ]
    .iter()
    .enumerate()
    {
        unsigned[i] = (*key, bj::field(fixed, key));
    }
    unsigned[10] = ("token_schema", JsonView::String("s-snapshot-token/2"));
    unsigned[11] = (
        "order_version",
        bj::field(projection.wire.view(), "order_version"),
    );
    unsigned[12] = ("page_size", JsonView::String(page.text()));
    unsigned[13] = ("offset", JsonView::String(offset.text()));
    unsigned[14] = (
        "cut_projection_digest",
        JsonView::String(&projection.digest),
    );
    unsigned[15] = ("issued_capture_digest", JsonView::String(issued));
    let checksum = canonical_hash_view_with_owner(
        JsonView::object_fields(&unsigned).map_err(err)?,
        allocation,
    )?;
    let mut fields = [("", JsonView::Null); 17];
    fields[..16].copy_from_slice(&unsigned);
    fields[16] = ("token_checksum", JsonView::String(checksum.as_str()));
    bj::json_from_view(JsonView::object_fields(&fields).map_err(err)?, allocation)
}
#[allow(dead_code, reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删")]
pub(super) fn token(
    projection: &Projection,
    issued: &str,
    page_size: u64,
    offset: u64,
) -> serde_json::Value {
    #[cfg(feature = "s_allocation_json_testonly")]
    if matches!(projection.wire, JsonOwner::Owned(_)) {
        panic!("旧token只支持None投影，请使用token_with_budget");
    }
    match token_with_budget(projection, issued, page_size, offset, None).expect("None token同核")
    {
        JsonOwner::Plain(v) => v,
        #[cfg(feature = "s_allocation_json_testonly")]
        JsonOwner::Owned(_) => unreachable!(),
    }
}
struct PageRows<'a> {
    rows: aj::ArrayView<'a>,
    start: usize,
    end: usize,
}
impl aj::JsonArray for PageRows<'_> {
    fn len(&self) -> usize {
        self.end - self.start
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        if index >= self.len() {
            None
        } else {
            self.rows.get(self.start + index)
        }
    }
}
pub(super) fn reply(
    request: &Request,
    projection: Projection,
    capture_digest: &str,
    resources: &QResources,
    producer_epoch: &str,
    job: &mut dyn Access,
) -> Result<Vec<u8>, String> {
    reply_core(
        request,
        projection,
        capture_digest,
        resources,
        producer_epoch,
        job,
        None,
    )?
    .try_into_plain()
    .map_err(err)
}
pub(super) fn reply_owned(
    request: &Request,
    projection: Projection,
    capture_digest: &str,
    resources: &QResources,
    producer_epoch: &str,
    job: &mut dyn Access,
    allocation: &OwnedBudget,
) -> Result<CodecVec<u8>, String> {
    reply_core(
        request,
        projection,
        capture_digest,
        resources,
        producer_epoch,
        job,
        Some(allocation),
    )
}
fn reply_core(
    request: &Request,
    projection: Projection,
    capture_digest: &str,
    resources: &QResources,
    producer_epoch: &str,
    job: &mut dyn Access,
    allocation: Option<&OwnedBudget>,
) -> Result<CodecVec<u8>, String> {
    request.validate_resources(resources)?;
    positive(producer_epoch, "Q producer_epoch")?;
    job.pre_io_check().map_err(err)?;
    let wire = projection.wire.view();
    let fixed = bj::field(wire, "fixed_cut");
    let session = required_view(fixed, "session_id")?;
    let incarnation = required_view(fixed, "session_generation")?;
    for key in ["session_id", "session_generation"] {
        if bj::field(request.envelope_view(), key) != bj::field(fixed, key) {
            return Err(err("外层session/化身与当前已核会话不符"));
        }
    }
    let issued = request.issued.as_deref().unwrap_or(capture_digest);
    if request.issued.is_some() {
        let token = token_with_budget(
            &projection,
            issued,
            request.page_size,
            request.offset,
            allocation,
        )?;
        if bj::field(
            bj::field(request.envelope_view(), "payload"),
            "snapshot_token",
        ) != token.view()
        {
            return Err(err("snapshot_token与本次已核指定历史切面的实际身份/根不符"));
        }
    } else {
        let payload = bj::field(request.envelope_view(), "payload");
        for key in ["session_id", "session_generation", "scope"] {
            if bj::field(payload, key) != bj::field(fixed, key) {
                return Err(err(format!("首请求{key}与当前已核会话不符")));
            }
        }
        if bj::field(payload, "order_version") != bj::field(wire, "order_version") {
            return Err(err("order_version不符"));
        }
    }
    let rows = bj::field(wire, "rows")
        .as_array()
        .ok_or_else(|| err("投影rows不是数组"))?;
    let total = rows.len() as u64;
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
        None
    } else {
        Some(token_with_budget(
            &projection,
            issued,
            request.page_size,
            end,
            allocation,
        )?)
    };
    let page_rows = PageRows {
        rows,
        start: request.offset as usize,
        end: end as usize,
    };
    let offset = Decimal::new(request.offset);
    let fields = [
        ("fixed_cut", bj::field(wire, "fixed_cut")),
        ("catalog", bj::field(wire, "catalog")),
        ("rows", JsonView::BorrowedArray(&page_rows)),
        ("counts", bj::field(wire, "counts")),
        ("scope", bj::field(wire, "scope")),
        ("omissions", bj::field(wire, "omissions")),
        ("order_version", bj::field(wire, "order_version")),
        ("offset", JsonView::String(offset.text())),
        ("done", JsonView::Bool(done)),
        (
            "next_token",
            next.as_ref().map_or(JsonView::Null, JsonOwner::view),
        ),
        (
            "cut_projection_digest",
            JsonView::String(&projection.digest),
        ),
        ("validated_capture_digest", JsonView::String(capture_digest)),
        ("ok", JsonView::Bool(true)),
    ];
    super::q_wire::reply_view(
        request.envelope_view(),
        JsonView::object_fields(&fields).map_err(err)?,
        session,
        incarnation,
        producer_epoch,
        resources,
        job,
        allocation,
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{capture::capture_configured, prepared::tests::Fixture, q_resources};
    use super::*;
    use newchan_rust::s_content::job::{Existing, JobLimits};
    use std::{path::Path, process::Command, time::Instant};
    fn plain_wire(p: &Projection) -> &Value {
        match &p.wire {
            JsonOwner::Plain(v) => v,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(_) => panic!("普通oracle只接Plain"),
        }
    }
    // TestOnly，不是产品期限（#1394 B1）：功能测试 job 的 max_elapsed_ms 放长到 600000，不随 runner 速度超期；测 job 期限的用例自带受控时钟。
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"100","max_elapsed_ms":"600000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(Instant::now)).unwrap()
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
        let fixed = &plain_wire(&projection)["fixed_cut"];
        envelope(
            fixed,
            json!({"op":"snapshot","session_id":fixed["session_id"],"session_generation":fixed["session_generation"],
            "scope":fixed["scope"],"history_mode":mode,"as_of_generation":as_of,"page_size":size.to_string(),
            "order_version":plain_wire(&projection)["order_version"]}),
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
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_snapshot_two_input_first_and_continuation_match_complete_plain_reply_and_drop() {
        use super::super::{
            capture::{capture_verified, capture_verified_with_allocation},
            q_allocation::Profile,
        };
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let r = q_resources::tests::functional_resources();
        let mut normal = capture_verified(&f.db_path(), r.capture_limits()).unwrap();
        let p = normal
            .projection(
                2,
                HistoryMode::AsKnown,
                r.limit(Limit::CaptureBytes),
                &mut job(),
            )
            .unwrap();
        let mut incoming = first(&p, 1, "AsKnown", Value::Null);
        drop(p);
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let mut owned = capture_verified_with_allocation(
            &f.db_path(),
            r.capture_limits(),
            Some(Profile {
                budget: budget.clone(),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            }),
        )
        .unwrap();
        for _ in 0..2 {
            let raw = canonical_json(&incoming).into_bytes();
            let normal_request = Request::parse(&raw, &r).unwrap();
            let owned_request = Request::parse_owned(&raw, &r, &budget).unwrap();
            let normal_projection = normal
                .projection(
                    2,
                    HistoryMode::AsKnown,
                    r.limit(Limit::CaptureBytes),
                    &mut job(),
                )
                .unwrap();
            let owned_projection = owned
                .projection(
                    2,
                    HistoryMode::AsKnown,
                    r.limit(Limit::CaptureBytes),
                    &mut job(),
                )
                .unwrap();
            let expected = reply(
                &normal_request,
                normal_projection,
                normal.digest(),
                &r,
                "7",
                &mut job(),
            )
            .unwrap();
            let actual = reply_owned(
                &owned_request,
                owned_projection,
                owned.digest(),
                &r,
                "7",
                &mut job(),
                &budget,
            )
            .unwrap();
            assert_eq!(&*actual, &expected);
            assert!(actual.allocation_budget().unwrap().same_ledger(&budget));
            let response = strict_json(&expected).unwrap();
            assert_eq!(response["payload"].as_object().unwrap().len(), 13);
            assert!(!response["payload"]["done"].as_bool().unwrap());
            incoming = envelope(
                &response["payload"]["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":response["payload"]["next_token"]}),
            );
        }
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn real_pages_rebuild_old_cut_after_commit_and_restart_preserving_issued_digest() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::functional_resources();
        let expected = fixture_projection(&f, &resources, 1);
        let wanted = plain_wire(&expected)["rows"].as_array().unwrap();
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
            let before_projection = job.snapshot().records;
            let bytes = capture
                .snapshot(&request(&incoming, &resources), "7", &mut job)
                .unwrap();
            assert_eq!(
                job.snapshot().records - before_projection,
                wanted.len() as u64
            );
            let response = strict_json(&bytes).unwrap();
            assert_eq!(&bytes[..], canonical_json(&response).as_bytes());
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
            assert_eq!(page["fixed_cut"], plain_wire(&expected)["fixed_cut"]);
            assert_eq!(page["counts"], plain_wire(&expected)["counts"]);
            assert_eq!(page["cut_projection_digest"], expected.digest.as_str());
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
                &plain_wire(&expected)["fixed_cut"],
                json!({"op":"snapshot","snapshot_token":next}),
            );
            assert!(pages <= wanted.len());
        }
        assert_eq!(&rows, wanted);
        // 每页投影仍访问完整六族一次；同一job还累计每次当前完整历史审计的访问。
        assert!(job.snapshot().records > (pages * wanted.len()) as u64);
        assert_eq!(job.snapshot().requests, pages as u64);
        assert_eq!(job.snapshot().wire_bytes, 0, "编码不冒充真实发送");
    }
    #[test]
    fn zero_cut_and_original_mode_generation_combinations_are_preserved() {
        let mut f = Fixture::new();
        let resources = q_resources::tests::functional_resources();
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
        let resources = q_resources::tests::functional_resources();
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
                &plain_wire(&expected)["fixed_cut"],
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
            &plain_wire(&expected)["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":corrupted}),
        );
        assert!(Request::parse(canonical_json(&incoming).as_bytes(), &resources).is_err());
        let mut old = token.clone();
        old["token_schema"] = json!("s-snapshot-token/1");
        resign(&mut old);
        let incoming = envelope(
            &plain_wire(&expected)["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":old}),
        );
        assert!(Request::parse(canonical_json(&incoming).as_bytes(), &resources).is_err());
        // 原合同允许合法非整页倍数作为独立区间，不能将校验和升级为服务端授权签名。
        let mut standalone = token.clone();
        standalone["page_size"] = json!("2");
        standalone["offset"] = json!("1");
        resign(&mut standalone);
        let incoming = envelope(
            &plain_wire(&expected)["fixed_cut"],
            json!({"op":"snapshot","snapshot_token":standalone}),
        );
        let page = strict_json(
            &capture
                .snapshot(&request(&incoming, &resources), "1", &mut job())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(page["payload"]["offset"], "1");
        let total = plain_wire(&expected)["rows"].as_array().unwrap().len();
        let mut terminal = token;
        terminal["offset"] = json!(total.to_string());
        resign(&mut terminal);
        let incoming = envelope(
            &plain_wire(&expected)["fixed_cut"],
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
        let resources = q_resources::tests::functional_resources();
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
        assert_eq!(&bytes[..], &oracle.stdout[..]);
        assert!(!bytes.ends_with(b"\n"));
        for (name, n) in [
            ("max_frame_bytes", request_bytes.len()),
            ("max_reply_bytes", bytes.len()),
        ] {
            let mut config = q_resources::tests::functional_input();
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
