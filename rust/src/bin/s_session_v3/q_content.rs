//! #1394 C §5.6：typed anchor内容片段与content-token/1的请求/回复合同。
//! 实际根/保留窗口由ConfiguredCapture核验；checksum不是权限凭据或已安装前缀证书。
use super::{
    projection::HistoryMode,
    q_resources::{Limit, QResources},
    q_wire,
};
use newchan_rust::s_content::access::Access;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::{
    s_content::{
        fragment::{self, Fragment, Unit},
        path::PathStep,
        ContentRef, RefView,
    },
    session_protocol::{canonical_json, nonnegative, positive, required, sha256_hex},
};
use serde_json::{json, Value};

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
    envelope: Value,
    bytes: usize,
    anchor: Value,
    anchor_kind: Anchor,
    path_wire: Value,
    path: Vec<PathStep>,
    unit: Unit,
    offset: u64,
    limit: u64,
    target: Option<ContentRef>,
    issued: Option<String>,
}
impl Request {
    pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Self, String> {
        let envelope = q_wire::parse(bytes, resources)?;
        let payload = &envelope["payload"];
        if payload["op"] != "content" {
            return Err(err("op必须是content"));
        }
        let (body, issued, target) = if payload.get("content_token").is_some() {
            exact(payload, &["op", "content_token"])?;
            let token = &payload["content_token"];
            exact(token, &TOKEN_FIELDS)?;
            if token["token_schema"] != "s-content-token/1" {
                return Err(err("content_token版本不符"));
            }
            let issued = required(token, "issued_capture_digest")?;
            if issued.len() != 64
                || !issued
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(err("签发摘要须为64位小写hex"));
            }
            let mut unsigned = token.clone();
            unsigned.as_object_mut().unwrap().remove("token_checksum");
            if token["token_checksum"] != sha256_hex(canonical_json(&unsigned).as_bytes()) {
                return Err(err("content_token checksum不符"));
            }
            (
                token,
                Some(issued.to_owned()),
                Some(ContentRef::from_transport_json(&token["target_ref"]).map_err(err)?),
            )
        } else {
            exact(
                payload,
                &["op", "anchor", "path", "unit", "offset", "limit"],
            )?;
            (payload, None, None)
        };
        let anchor = body["anchor"].clone();
        let anchor_kind = match required(&anchor, "kind")? {
            "projection" => {
                exact(
                    &anchor,
                    &[
                        "kind",
                        "fixed_cut",
                        "cut_projection_digest",
                        "projection_ref",
                    ],
                )?;
                ContentRef::from_transport_json(&anchor["projection_ref"]).map_err(err)?;
                let fixed = &anchor["fixed_cut"];
                let generation = nonnegative(required(fixed, "cut_generation")?, "cut_generation")?;
                let mode = match required(fixed, "history_mode")? {
                    "AsKnown" => HistoryMode::AsKnown,
                    "RecomputedWithRevision" => HistoryMode::RecomputedWithRevision,
                    _ => return Err(err("history_mode不符")),
                };
                Anchor::Projection { generation, mode }
            }
            "delta" => {
                exact(
                    &anchor,
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
                ContentRef::from_transport_json(&anchor["delta_ref"]).map_err(err)?;
                positive(required(&anchor, "generation")?, "generation")?;
                Anchor::Delta
            }
            _ => return Err(err("anchor kind不符；不可用裸Hash访问")),
        };
        let path_wire = body["path"].clone();
        let path = fragment::parse_path(&path_wire).map_err(err)?;
        let unit = Unit::parse(required(body, "unit")?).map_err(err)?;
        let offset = nonnegative(required(body, "offset")?, "offset")? as u64;
        let limit = positive(required(body, "limit")?, "limit")? as u64;
        offset
            .checked_add(limit)
            .ok_or_else(|| err("offset+limit溢出"))?;
        Ok(Self {
            envelope,
            bytes: bytes.len(),
            anchor,
            anchor_kind,
            path_wire,
            path,
            unit,
            offset,
            limit,
            target,
            issued,
        })
    }
    pub(super) fn validate_resources(&self, r: &QResources) -> Result<(), String> {
        if self.bytes as u128 > u128::from(r.limit(Limit::FrameBytes)) {
            return Err(err("Incomplete：完整请求超过max_frame_bytes"));
        }
        Ok(())
    }
    pub(super) fn anchor_kind(&self) -> &Anchor {
        &self.anchor_kind
    }
    pub(super) fn anchor(&self) -> &Value {
        &self.anchor
    }
    pub(super) fn validate_identity(&self, session: &str, incarnation: &str) -> Result<(), String> {
        if self.envelope["session_id"] != session
            || self.envelope["session_generation"] != incarnation
        {
            return Err(err("外层session/化身与实际已核内容会话不符"));
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
        let fragment = fragment::read_refview(
            source,
            root.into(),
            &self.path,
            self.unit,
            self.offset,
            self.limit,
            resources.limit(Limit::ReplyBytes),
            job,
        )
        .map_err(err)?;
        if self
            .target
            .as_ref()
            .is_some_and(|target| *target != fragment.target_ref)
        {
            return Err(err("content_token target_ref与实际根和路径的目标不同"));
        }
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
        self.validate_resources(resources)?;
        self.validate_identity(session, incarnation)?;
        let issued = self.issued.as_deref().unwrap_or(capture_digest);
        let next = if fragment.done {
            Value::Null
        } else {
            let mut token = json!({"token_schema":"s-content-token/1","anchor":self.anchor,"path":self.path_wire,
                "target_ref":fragment.target_ref.to_transport_json(),"unit":self.unit.name(),"offset":fragment.end.to_string(),
                "limit":self.limit.to_string(),"issued_capture_digest":issued});
            token["token_checksum"] = json!(sha256_hex(canonical_json(&token).as_bytes()));
            token
        };
        let mut payload = Value::Object(fragment.fields());
        payload["anchor"] = self.anchor.clone();
        payload["path"] = self.path_wire.clone();
        payload["next_token"] = next;
        payload["validated_capture_digest"] = json!(capture_digest);
        payload["ok"] = json!(true);
        q_wire::reply(
            &self.envelope,
            payload,
            session,
            incarnation,
            epoch,
            resources,
            job,
        )
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
        capture
            .delta_references(g - 1, 1, &mut job())
            .unwrap()
            .anchors
            .remove(0)
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
        assert_eq!(bytes, canonical_json(&value).as_bytes());
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
