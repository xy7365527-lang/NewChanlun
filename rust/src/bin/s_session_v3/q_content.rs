//! #1394 C §5.6：typed anchor内容片段与content-token/1的请求/回复合同。
//! 实际根/保留窗口由ConfiguredCapture核验；checksum不是权限凭据或已安装前缀证书。
use super::{
    business_json as bj,
    projection::HistoryMode,
    q_resources::{Limit, QResources},
    q_wire,
};
use newchan_rust::s_content::access::Access;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::{
    s_content::{
        capture_allocation::{audit_json::JsonView, OwnedBudget},
        fragment::{self, Fragment, OwnedFragment, Unit},
        path::PathStep,
        ClosedCapturedOwner, CodecVec, ContentRef, RefView, TransportRef,
    },
    session_protocol::{canonical_hash_view_with_owner, canonical_json, required_view, sha256_hex},
};
use serde_json::{json, Value};

use super::q_wire::{AdmissionError, RouteFixed};
use newchan_rust::session_protocol::{
    canonical_hash_view_typed, nonnegative_typed, positive_typed, required_view_typed,
};

const TOKEN_FIELDS: [&str; 9] = [
    "token_schema",
    "anchor",
    "path",
    "target_ref",
    "unit",
    "offset",
    "limit",
    "issued_capture_digest",
    "token_checksum",
];
pub(super) enum Anchor {
    Projection { generation: i64, mode: HistoryMode },
    Delta,
}
pub(super) struct Request {
    envelope: bj::SharedEnvelope,
    bytes: usize,
    anchor_kind: Anchor,
    unit: Unit,
    offset: u64,
    limit: u64,
    continuation: bool,
}
impl Request {
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        Self::parse_with_allocation(bytes, resources, None)
    }
    pub(super) fn parse_owned(
        bytes: &[u8],
        resources: &QResources,
        budget: &OwnedBudget,
    ) -> Result<Self, String> {
        Self::parse_with_allocation(bytes, resources, Some(budget))
    }
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
        _resources: &QResources,
        _allocation: Option<&OwnedBudget>,
    ) -> Result<Self, AdmissionError<'a>> {
        let allocation = envelope
            .view()
            .allocation_budget()
            .map_err(AdmissionError::ContentAllocation)?;
        let payload = child(envelope.view(), "payload");
        if child(payload, "op").as_str() != Some("content") {
            return Err(AdmissionError::Fixed(RouteFixed::ContentOp));
        }
        let continuation = payload.get("content_token").is_some();
        let body = if continuation {
            exact_view_typed(payload, &["op", "content_token"])?;
            let token = child(payload, "content_token");
            exact_view_typed(token, &TOKEN_FIELDS)?;
            if child(token, "token_schema").as_str() != Some("s-content-token/1") {
                return Err(AdmissionError::Fixed(RouteFixed::ContentVersion));
            }
            let issued = required_view_typed(token, "issued_capture_digest")?;
            if issued.len() != 64
                || !issued
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(AdmissionError::Fixed(RouteFixed::ContentDigest));
            }
            let unsigned_fields = [
                ("token_schema", child(token, "token_schema")),
                ("anchor", child(token, "anchor")),
                ("path", child(token, "path")),
                ("target_ref", child(token, "target_ref")),
                ("unit", child(token, "unit")),
                ("offset", child(token, "offset")),
                ("limit", child(token, "limit")),
                (
                    "issued_capture_digest",
                    child(token, "issued_capture_digest"),
                ),
            ];
            let unsigned =
                JsonView::object_fields(&unsigned_fields).map_err(AdmissionError::ContentFields)?;
            let checksum = canonical_hash_view_typed(unsigned, allocation)?;
            if child(token, "token_checksum").as_str() != Some(checksum.as_str()) {
                return Err(AdmissionError::Fixed(RouteFixed::ContentChecksum));
            }
            TransportRef::from_json_view_typed(child(token, "target_ref"))
                .map_err(AdmissionError::Ref)?;
            token
        } else {
            exact_view_typed(
                payload,
                &["op", "anchor", "path", "unit", "offset", "limit"],
            )?;
            payload
        };
        let anchor = child(body, "anchor");
        let anchor_kind = match required_view_typed(anchor, "kind")? {
            "projection" => {
                exact_view_typed(
                    anchor,
                    &[
                        "kind",
                        "fixed_cut",
                        "cut_projection_digest",
                        "projection_ref",
                    ],
                )?;
                TransportRef::from_json_view_typed(child(anchor, "projection_ref"))
                    .map_err(AdmissionError::Ref)?;
                let fixed = child(anchor, "fixed_cut");
                let generation = nonnegative_typed(
                    required_view_typed(fixed, "cut_generation")?,
                    "cut_generation",
                )?;
                let mode = match required_view_typed(fixed, "history_mode")? {
                    "AsKnown" => HistoryMode::AsKnown,
                    "RecomputedWithRevision" => HistoryMode::RecomputedWithRevision,
                    _ => return Err(AdmissionError::Fixed(RouteFixed::ContentMode)),
                };
                Anchor::Projection { generation, mode }
            }
            "delta" => {
                exact_view_typed(
                    anchor,
                    &[
                        "kind",
                        "session_id",
                        "session_generation",
                        "generation",
                        "base_cut",
                        "next_cut",
                        "index_frontier",
                        "delta_ref",
                    ],
                )?;
                TransportRef::from_json_view_typed(child(anchor, "delta_ref"))
                    .map_err(AdmissionError::Ref)?;
                positive_typed(required_view_typed(anchor, "generation")?, "generation")?;
                Anchor::Delta
            }
            _ => return Err(AdmissionError::Fixed(RouteFixed::ContentAnchor)),
        };
        fragment::parse_path_view_typed(child(body, "path")).map_err(AdmissionError::Path)?;
        let unit =
            Unit::parse_typed(required_view_typed(body, "unit")?).map_err(AdmissionError::Path)?;
        let offset = nonnegative_typed(required_view_typed(body, "offset")?, "offset")? as u64;
        let limit = positive_typed(required_view_typed(body, "limit")?, "limit")? as u64;
        offset
            .checked_add(limit)
            .ok_or_else(|| AdmissionError::Fixed(RouteFixed::ContentOverflow))?;
        Ok(Self {
            envelope: envelope.clone(),
            bytes,
            anchor_kind,
            unit,
            offset,
            limit,
            continuation,
        })
    }
    pub(super) fn allocation_budget(&self) -> Result<Option<&OwnedBudget>, String> {
        self.envelope.view().allocation_budget().map_err(err)
    }
    fn body_view(&self) -> JsonView<'_> {
        let payload = child(self.envelope.view(), "payload");
        if self.continuation {
            child(payload, "content_token")
        } else {
            payload
        }
    }
    pub(super) fn validate_resources(&self, resources: &QResources) -> Result<(), String> {
        if self.bytes as u128 > u128::from(resources.limit(Limit::FrameBytes)) {
            return Err(err("Incomplete：完整请求超过max_frame_bytes"));
        }
        Ok(())
    }
    pub(super) fn anchor_kind(&self) -> &Anchor {
        &self.anchor_kind
    }
    pub(super) fn anchor_view(&self) -> JsonView<'_> {
        child(self.body_view(), "anchor")
    }
    pub(super) fn anchor(&self) -> &Value {
        let bj::JsonOwner::Plain(envelope) = &*self.envelope else {
            panic!("owned request须借anchor_view，不能擦std")
        };
        let payload = &envelope["payload"];
        if self.continuation {
            &payload["content_token"]["anchor"]
        } else {
            &payload["anchor"]
        }
    }
    pub(super) fn validate_identity(&self, session: &str, incarnation: &str) -> Result<(), String> {
        if child(self.envelope.view(), "session_id").as_str() != Some(session)
            || child(self.envelope.view(), "session_generation").as_str() != Some(incarnation)
        {
            return Err(err("外层session/化身与实际已核内容会话不符"));
        }
        Ok(())
    }
    fn check_target(&self, actual: RefView<'_>) -> Result<(), String> {
        if self.continuation {
            let target =
                TransportRef::from_json_view(child(self.body_view(), "target_ref")).map_err(err)?;
            if target.format() != actual.format()
                || target.root() != actual.root()
                || target.kind() != actual.kind()
                || target.extent() != actual.extent()
            {
                return Err(err("content_token target_ref与实际根和路径的目标不同"));
            }
        }
        Ok(())
    }
    pub(super) fn read<'a>(
        &self,
        source: &mut dyn newchan_rust::s_content::NodeSource,
        root: impl Into<RefView<'a>>,
        resources: &QResources,
        job: &mut dyn Access,
    ) -> Result<Fragment, String> {
        let path = fragment::parse_path_view(child(self.body_view(), "path")).map_err(err)?;
        let fragment = fragment::read_view(
            source,
            root.into(),
            &path,
            self.unit,
            self.offset,
            self.limit,
            resources.limit(Limit::ReplyBytes),
            job,
        )
        .map_err(err)?;
        self.check_target(fragment.target_ref.view())?;
        fragment.into_plain().map_err(err)
    }
    pub(super) fn read_owned<'a>(
        &self,
        source: &mut ClosedCapturedOwner<'_>,
        root: impl Into<RefView<'a>>,
        resources: &QResources,
        job: &mut dyn Access,
    ) -> Result<OwnedFragment, String> {
        let path = fragment::parse_path_view(child(self.body_view(), "path")).map_err(err)?;
        let fragment = fragment::read_owned(
            source,
            root.into(),
            &path,
            self.unit,
            self.offset,
            self.limit,
            resources.limit(Limit::ReplyBytes),
            job,
        )
        .map_err(err)?;
        self.check_target(fragment.target_ref.view())?;
        Ok(fragment)
    }
    pub(super) fn reply(
        &self,
        fragment: Fragment,
        capture_digest: &str,
        session: &str,
        incarnation: &str,
        epoch: &str,
        resources: &QResources,
        job: &mut dyn Access,
    ) -> Result<Vec<u8>, String> {
        let fields = Value::Object(fragment.fields());
        self.reply_core(
            (&fields).into(),
            fragment.done,
            fragment.end,
            capture_digest,
            session,
            incarnation,
            epoch,
            resources,
            job,
            None,
        )?
        .try_into_plain()
        .map_err(err)
    }
    pub(super) fn reply_owned(
        &self,
        fragment: OwnedFragment,
        capture_digest: &str,
        session: &str,
        incarnation: &str,
        epoch: &str,
        resources: &QResources,
        job: &mut dyn Access,
    ) -> Result<CodecVec<u8>, String> {
        let allocation = fragment
            .allocation_budget()
            .ok_or_else(|| err("owned fragment缺真实来源账本"))?;
        self.reply_core(
            fragment.fields_view().map_err(err)?,
            fragment.done,
            fragment.end,
            capture_digest,
            session,
            incarnation,
            epoch,
            resources,
            job,
            Some(allocation),
        )
    }
    fn reply_core(
        &self,
        fields: JsonView<'_>,
        done: bool,
        end: u64,
        capture_digest: &str,
        session: &str,
        incarnation: &str,
        epoch: &str,
        resources: &QResources,
        job: &mut dyn Access,
        allocation: Option<&OwnedBudget>,
    ) -> Result<CodecVec<u8>, String> {
        self.validate_resources(resources)?;
        self.validate_identity(session, incarnation)?;
        let body = self.body_view();
        let issued = if self.continuation {
            required_view(body, "issued_capture_digest")?
        } else {
            capture_digest
        };
        let offset = bj::Decimal::new(end as i64);
        let limit = bj::Decimal::new(self.limit as i64);
        let unsigned_fields = [
            ("token_schema", JsonView::String("s-content-token/1")),
            ("anchor", self.anchor_view()),
            ("path", child(body, "path")),
            ("target_ref", child(fields, "target_ref")),
            ("unit", JsonView::String(self.unit.name())),
            ("offset", JsonView::String(offset.text())),
            ("limit", JsonView::String(limit.text())),
            ("issued_capture_digest", JsonView::String(issued)),
        ];
        let unsigned = JsonView::object_fields(&unsigned_fields).map_err(err)?;
        // done分支与旧行为相同：不生成、也不核下一个token的checksum。
        let checksum = if done {
            None
        } else {
            Some(canonical_hash_view_with_owner(unsigned, allocation)?)
        };
        let token_fields = [
            unsigned_fields[0],
            unsigned_fields[1],
            unsigned_fields[2],
            unsigned_fields[3],
            unsigned_fields[4],
            unsigned_fields[5],
            unsigned_fields[6],
            unsigned_fields[7],
            (
                "token_checksum",
                checksum
                    .as_ref()
                    .map_or(JsonView::Null, |v| JsonView::String(v.as_str())),
            ),
        ];
        let next = if done {
            JsonView::Null
        } else {
            JsonView::object_fields(&token_fields).map_err(err)?
        };
        let payload_fields = [
            ("target_ref", child(fields, "target_ref")),
            ("unit", child(fields, "unit")),
            ("offset", child(fields, "offset")),
            ("end", child(fields, "end")),
            ("total", child(fields, "total")),
            ("items", child(fields, "items")),
            ("nodes", child(fields, "nodes")),
            ("done", child(fields, "done")),
            ("anchor", self.anchor_view()),
            ("path", child(body, "path")),
            ("next_token", next),
            ("validated_capture_digest", JsonView::String(capture_digest)),
            ("ok", JsonView::Bool(true)),
        ];
        q_wire::reply_view(
            self.envelope.view(),
            JsonView::object_fields(&payload_fields).map_err(err)?,
            session,
            incarnation,
            epoch,
            resources,
            job,
            allocation,
        )
    }
}
fn child<'a>(value: JsonView<'a>, key: &str) -> JsonView<'a> {
    value.get(key).unwrap_or(JsonView::Null)
}
fn exact_view(value: JsonView<'_>, fields: &[&str]) -> Result<(), String> {
    exact_view_typed(value, fields).map_err(|e| e.to_string())
}
fn exact_view_typed(value: JsonView<'_>, fields: &[&str]) -> Result<(), AdmissionError<'static>> {
    if value
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k)))
    {
        Ok(())
    } else {
        Err(AdmissionError::Fixed(RouteFixed::ContentExact))
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QContentInvalid：{e}")
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::{
        capture::{capture_configured, ConfiguredCapture},
        prepared::tests::Fixture,
        q_resources,
    };
    use super::*;
    use newchan_rust::{
        s_content::{
            fragment::ProofNode,
            job::{Existing, JobLimits},
        },
        session_protocol::strict_json,
    };
    use std::time::Instant;
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"100","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn projection(capture: &mut ConfiguredCapture<'_>, g: i64) -> Value {
        capture
            .projection(g, HistoryMode::AsKnown, &mut job())
            .unwrap()
            .anchor()
    }
    fn delta(capture: &mut ConfiguredCapture<'_>, g: i64) -> Value {
        let owner = capture
            .delta_references(g - 1, 1, &mut job())
            .unwrap()
            .anchors
            .into_iter()
            .next()
            .unwrap();
        match owner {
            bj::JsonOwner::Plain(value) => value,
            #[cfg(feature = "s_allocation_json_testonly")]
            bj::JsonOwner::Owned(_) => panic!("测试oracle只接普通delta"),
        }
    }
    fn envelope(anchor: &Value, payload: Value) -> Value {
        let id = if anchor["kind"] == "projection" {
            &anchor["fixed_cut"]
        } else {
            anchor
        };
        json!({"schema_revision":"s-session/3","session_id":id["session_id"],"session_generation":id["session_generation"],
            "source_namespace":"test.content/重","source_epoch":"1","message_id":"fragment-1","producer_id":"content-test",
            "producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }
    fn first(anchor: &Value, path: Value, unit: &str, offset: u64, limit: u64) -> Value {
        envelope(
            anchor,
            json!({"op":"content","anchor":anchor,"path":path,"unit":unit,"offset":offset.to_string(),"limit":limit.to_string()}),
        )
    }
    fn run(
        capture: &mut ConfiguredCapture<'_>,
        resources: &QResources,
        incoming: &Value,
        job: &mut dyn Access,
    ) -> Result<Value, String> {
        let request = Request::parse(canonical_json(incoming).as_bytes(), resources)?;
        let bytes = capture.content(&request, "4", job)?;
        let value = strict_json(&bytes)?;
        assert_eq!(&bytes[..], canonical_json(&value).as_bytes());
        assert_eq!(value.as_object().unwrap().len(), 11);
        assert_eq!(value["payload"].as_object().unwrap().len(), 13);
        assert_eq!(
            value["payload_hash"],
            sha256_hex(canonical_json(&value["payload"]).as_bytes())
        );
        Ok(value)
    }
    fn proof(page: &Value) -> Fragment {
        let n = |k: &str| page[k].as_str().unwrap().parse().unwrap();
        Fragment {
            target_ref: ContentRef::from_transport_json(&page["target_ref"]).unwrap(),
            unit: Unit::parse(page["unit"].as_str().unwrap()).unwrap(),
            offset: n("offset"),
            end: n("end"),
            total: n("total"),
            items: page["items"].clone(),
            done: page["done"].as_bool().unwrap(),
            nodes: page["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| ProofNode::from_json(v).unwrap())
                .collect(),
        }
    }
    fn verify(page: &Value, limit: u64) -> Fragment {
        let anchor = &page["anchor"];
        let key = if anchor["kind"] == "projection" {
            "projection_ref"
        } else {
            "delta_ref"
        };
        let root = ContentRef::from_transport_json(&anchor[key]).unwrap();
        let fragment = proof(page);
        fragment::verify(
            &root,
            &fragment::parse_path(&page["path"]).unwrap(),
            fragment.unit,
            fragment.offset,
            limit,
            &fragment,
            8388608,
            &mut job(),
        )
        .unwrap();
        fragment
    }
    fn resign(token: &mut Value) {
        token.as_object_mut().unwrap().remove("token_checksum");
        token["token_checksum"] = json!(sha256_hex(canonical_json(token).as_bytes()));
    }
    #[test]
    fn projection_continuation_after_source_commit_carries_independent_proof_and_old_anchor() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let anchor = projection(
            &mut capture_configured(&f.db_path(), &resources, &mut job()).unwrap(),
            1,
        );
        let mut incoming = first(&anchor, json!([{"field":"rows"}]), "items", 0, 1);
        let mut job = job();
        let mut end = 0;
        let mut count = 0;
        let mut issued = String::new();
        let mut first_proof = None;
        loop {
            let mut capture = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
            let response = run(&mut capture, &resources, &incoming, &mut job).unwrap();
            let page = &response["payload"];
            assert_eq!(page["anchor"], anchor);
            let fragment = verify(page, 1);
            assert_eq!(fragment.offset, end);
            assert!(fragment
                .nodes
                .iter()
                .any(|n| n.root == anchor["projection_ref"]["root"]));
            end = fragment.end;
            count += 1;
            if count == 1 {
                issued = page["validated_capture_digest"].as_str().unwrap().into();
                first_proof = Some(fragment.clone());
                drop(capture);
                publish(&mut f, 2);
            } else {
                assert_ne!(page["validated_capture_digest"], issued);
            }
            if fragment.done {
                assert!(page["next_token"].is_null());
                assert_eq!(end, fragment.total);
                break;
            }
            let token = &page["next_token"];
            assert_eq!(token.as_object().unwrap().len(), 9);
            assert_eq!(token["issued_capture_digest"], issued);
            assert_eq!(token["offset"], end.to_string());
            incoming = envelope(&anchor, json!({"op":"content","content_token":token}));
            assert!(count < 30);
        }
        assert!(count > 1);
        assert_eq!(job.snapshot().requests, count);
        assert_eq!(job.snapshot().wire_bytes, 0);
        let mut missing = first_proof.unwrap();
        missing.nodes.clear();
        assert!(fragment::verify(
            &ContentRef::from_transport_json(&anchor["projection_ref"]).unwrap(),
            &[PathStep::Field("rows".into())],
            Unit::Items,
            0,
            1,
            &missing,
            8388608,
            &mut self::job()
        )
        .is_err());
    }
    #[test]
    fn delta_token_rechecks_target_root_session_and_atomic_limits() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let anchor = delta(&mut capture, 1);
        let first = first(&anchor, json!([{"field":"session_id"}]), "bytes", 0, 2);
        let response = run(&mut capture, &resources, &first, &mut job()).unwrap();
        let page = &response["payload"];
        verify(page, 2);
        let token = page["next_token"].clone();
        assert!(token.is_object());
        let mut target = token.clone();
        target["target_ref"] = anchor["delta_ref"].clone();
        resign(&mut target);
        let wrong = envelope(&anchor, json!({"op":"content","content_token":target}));
        assert!(run(&mut capture, &resources, &wrong, &mut job())
            .unwrap_err()
            .contains("target_ref"));
        let mut path = token.clone();
        path["path"] = json!([{"field":"base_cut"}]);
        resign(&mut path);
        assert!(run(
            &mut capture,
            &resources,
            &envelope(&anchor, json!({"op":"content","content_token":path})),
            &mut job()
        )
        .is_err());
        let mut root = token.clone();
        root["anchor"]["delta_ref"]["root"] = json!(format!("sc1-{}", "0".repeat(64)));
        resign(&mut root);
        assert!(run(
            &mut capture,
            &resources,
            &envelope(&anchor, json!({"op":"content","content_token":root})),
            &mut job()
        )
        .unwrap_err()
        .contains("IdentityConflict"));
        let mut outer = first.clone();
        outer["session_id"] = json!("foreign");
        assert!(run(&mut capture, &resources, &outer, &mut job())
            .unwrap_err()
            .contains("外层session"));
        for key in [
            "max_atomic_delta_nodes",
            "max_atomic_delta_node_bytes",
            "max_atomic_delta_records",
        ] {
            let mut config = q_resources::tests::input();
            config["limits"][key] = json!("1");
            let tiny = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
            let mut restricted = capture_configured(&f.db_path(), &tiny, &mut job()).unwrap();
            assert!(
                run(&mut restricted, &tiny, &first, &mut job())
                    .unwrap_err()
                    .contains(key),
                "{key}"
            );
        }
        publish(&mut f, 2);
        let next = envelope(&anchor, json!({"op":"content","content_token":token}));
        let result = run(
            &mut capture_configured(&f.db_path(), &resources, &mut job()).unwrap(),
            &resources,
            &next,
            &mut job(),
        )
        .unwrap();
        assert_eq!(verify(&result["payload"], 2).offset, 2);
        assert_ne!(
            result["payload"]["validated_capture_digest"],
            page["validated_capture_digest"]
        );
    }
    #[test]
    fn retained_old_projection_remains_readable_while_expired_delta_and_corrupt_state_fail() {
        let mut f = Fixture::with_retain(1);
        publish(&mut f, 1);
        let resources = q_resources::tests::resources();
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let old_projection = projection(&mut capture, 1);
        let old_delta = delta(&mut capture, 1);
        publish(&mut f, 2);
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let old = first(&old_delta, json!([]), "items", 0, 1);
        assert!(run(&mut capture, &resources, &old, &mut job())
            .unwrap_err()
            .contains("ContentAnchorUnavailable"));
        let old = first(&old_projection, json!([{"field":"rows"}]), "items", 0, 1);
        verify(
            &run(&mut capture, &resources, &old, &mut job()).unwrap()["payload"],
            1,
        );
        f.conn
            .execute(
                "UPDATE catalog SET run_status='not_run' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        assert!(
            capture_configured(&f.db_path(), &resources, &mut job()).is_err(),
            "当前完整状态破坏，不能只凭旧根继续新请求"
        );
    }
    #[test]
    fn terminal_ranges_and_strict_requests_and_full_envelope_bytes_are_bounded() {
        let f = Fixture::new();
        let resources = q_resources::tests::resources();
        let mut capture = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let anchor = capture
            .projection(0, HistoryMode::RecomputedWithRevision, &mut job())
            .unwrap()
            .anchor();
        let scalar = first(
            &anchor,
            json!([{"field":"fixed_cut"},{"field":"as_of_generation"}]),
            "value",
            0,
            1,
        );
        let page = run(&mut capture, &resources, &scalar, &mut job()).unwrap();
        assert_eq!(
            verify(&page["payload"], 1).items.as_array().unwrap().len(),
            1
        );
        let terminal = first(
            &anchor,
            json!([{"field":"fixed_cut"},{"field":"as_of_generation"}]),
            "value",
            1,
            1,
        );
        let page = run(&mut capture, &resources, &terminal, &mut job()).unwrap();
        assert_eq!(page["payload"]["items"], json!([]));
        assert_eq!(page["payload"]["done"], true);
        let beyond = first(
            &anchor,
            json!([{"field":"fixed_cut"},{"field":"as_of_generation"}]),
            "value",
            2,
            1,
        );
        assert!(run(&mut capture, &resources, &beyond, &mut job()).is_err());
        let bytes = canonical_json(&page).into_bytes();
        let request_bytes = canonical_json(&terminal).into_bytes();
        for (name, n) in [
            ("max_reply_bytes", bytes.len()),
            ("max_frame_bytes", request_bytes.len()),
        ] {
            let mut config = q_resources::tests::input();
            config["limits"][name] = json!(n.to_string());
            let exact = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
            assert_eq!(
                run(
                    &mut capture_configured(&f.db_path(), &exact, &mut job()).unwrap(),
                    &exact,
                    &terminal,
                    &mut job()
                )
                .unwrap(),
                page
            );
            config["limits"][name] = json!((n - 1).to_string());
            let short = QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap();
            assert!(run(
                &mut capture_configured(&f.db_path(), &short, &mut job()).unwrap(),
                &short,
                &terminal,
                &mut job()
            )
            .is_err());
        }
        for bad in [
            json!([{"index":"01"}]),
            json!([{"field":"x","index":"0"}]),
            json!([{"json_text":false}]),
        ] {
            let request = first(&anchor, bad, "items", 0, 1);
            assert!(Request::parse(canonical_json(&request).as_bytes(), &resources).is_err());
        }
        let duplicated =
            canonical_json(&terminal).replacen("{", "{\"schema_revision\":\"s-session/3\",", 1);
        assert!(Request::parse(duplicated.as_bytes(), &resources).is_err());
        let mut legacy = terminal;
        legacy["schema_revision"] = json!("s-session/2");
        assert!(Request::parse(canonical_json(&legacy).as_bytes(), &resources).is_err());
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod owned_request_tests {
    use super::*;
    use newchan_rust::s_content::{write_value, MemoryNodes, Value as ContentValue};
    #[test]
    fn content_owned_request_borrows_path_token_and_keeps_original_owner_to_drop0() {
        let resources = super::super::q_resources::tests::resources();
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &ContentValue::Object(vec![]))
            .unwrap()
            .content_ref();
        let anchor = json!({"kind":"projection","fixed_cut":{"session_id":"s","session_generation":"1","cut_generation":"0","history_mode":"AsKnown"},"cut_projection_digest":"a".repeat(64),"projection_ref":root.to_transport_json()});
        let path = json!([{"field":"汉\\\""},{"index":"0"},{"json_text":true}]);
        let payload = json!({"op":"content","anchor":anchor,"path":path,"unit":"value","offset":"0","limit":"1"});
        let incoming = json!({"schema_revision":"s-session/3","session_id":"s","session_generation":"1","source_namespace":"source","source_epoch":"1","message_id":"m","producer_id":"p","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
        let bytes = canonical_json(&incoming).into_bytes();
        let normal = Request::parse(&bytes, &resources).unwrap();
        let budget = OwnedBudget::new(100_000);
        let request = Request::parse_owned(&bytes, &resources, &budget).unwrap();
        assert_eq!(
            serde_json::to_value(request.envelope.view()).unwrap(),
            incoming
        );
        assert_eq!(
            serde_json::to_value(request.anchor_view()).unwrap(),
            *normal.anchor()
        );
        let path = fragment::parse_path_view(child(request.body_view(), "path")).unwrap();
        use newchan_rust::s_content::path::{PathStepView, PathView};
        assert_eq!(path.len(), 3);
        assert!(matches!(path.step(0), Some(PathStepView::Field("汉\\\""))));
        assert!(request
            .envelope
            .view()
            .allocation_budget()
            .unwrap()
            .is_some());
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        request.validate_identity("s", "1").unwrap();
        assert!(request.validate_identity("wrong", "1").is_err());
        drop(request);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let first = &incoming["payload"];
        let mut token = json!({"token_schema":"s-content-token/1","anchor":first["anchor"],"path":first["path"],"target_ref":root.to_transport_json(),"unit":"value","offset":"1","limit":"1","issued_capture_digest":"b".repeat(64)});
        token["token_checksum"] = json!(sha256_hex(canonical_json(&token).as_bytes()));
        let mut continued = incoming.clone();
        continued["payload"] = json!({"op":"content","content_token":token});
        continued["payload_hash"] =
            json!(sha256_hex(canonical_json(&continued["payload"]).as_bytes()));
        let continued_bytes = canonical_json(&continued).into_bytes();
        let normal = Request::parse(&continued_bytes, &resources).unwrap();
        let owned = Request::parse_owned(&continued_bytes, &resources, &budget).unwrap();
        assert_eq!(
            serde_json::to_value(owned.envelope.view()).unwrap(),
            continued
        );
        assert_eq!(
            serde_json::to_value(owned.anchor_view()).unwrap(),
            *normal.anchor()
        );
        assert!(owned.continuation);
        assert_eq!(
            child(owned.body_view(), "issued_capture_digest").as_str(),
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        continued["payload"]["content_token"]["token_checksum"] = json!("bad");
        continued["payload_hash"] =
            json!(sha256_hex(canonical_json(&continued["payload"]).as_bytes()));
        let invalid = canonical_json(&continued).into_bytes();
        let normal_error = Request::parse(&invalid, &resources).err().unwrap();
        let owned_error = Request::parse_owned(&invalid, &resources, &budget)
            .err()
            .unwrap();
        assert_eq!(owned_error, normal_error);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(0);
        assert!(Request::parse_owned(&bytes, &resources, &low).is_err());
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
}
