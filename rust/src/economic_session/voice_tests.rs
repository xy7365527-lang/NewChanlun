//! 单位验证使用明确虚构的受信 authority；不能作为独立定义 γ 或正式 Voice 成功证据。
use super::{book, evidence, service, store, voice};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct UnitSpace(PathBuf);
impl UnitSpace {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "nc1374-voice-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn write(&self, name: &str, value: &Value) -> (String, String, usize) {
        let path = self.0.join(name);
        let raw = canonical_json(value);
        std::fs::write(&path, &raw).unwrap();
        (
            path.to_str().unwrap().into(),
            sha256_hex(raw.as_bytes()),
            raw.len(),
        )
    }
}
impl Drop for UnitSpace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn config() -> store::Config {
    store::Config {
        manifest: json!({}),
        manifest_hash: "unit".into(),
        db: PathBuf::new(),
        domain: "B:opaque-one".into(),
        session: "unit".into(),
        generation: "1".into(),
        producer: "unit".into(),
        socket: PathBuf::new(),
    }
}
fn state() -> (Value, Value, Value) {
    let doc = book::tests::document();
    let event = &doc["events"][0];
    let raw = canonical_json(&doc);
    let raw_sha = sha256_hex(raw.as_bytes());
    let key = book::fact_key(event).unwrap();
    let fact_hash = book::hash(
        &json!({"raw_sha256":raw_sha,"json_pointer":"/events/0","event":event,"units":doc["units"]}),
    );
    let fact = json!({"raw_utf8":raw,"json_pointer":"/events/0","fact_key":key,"fact_hash":fact_hash,"raw_sha256":raw_sha,"event":event,"target_domain":"B:opaque-one","cut":{"domain_id":"E","commit_seq":"1","root_hash":"unit"}});
    let image = book::apply(&book::empty_image(), &fact, 1, "20").unwrap().0;
    let reference = |name| json!({"authority_id":"unit","object_id":name,"revision":"0","content_hash":book::hash(&json!(name))});
    let request = json!({"op":"BindVoice","binding_id":"first","evidence_id":"unit-first","carrier_ref":reference("carrier"),"gamma_ref":reference("gamma"),"parent_voice_id":null,"sigma":"Short","n":"0","opening_basis":{"fact_key":key,"expected_fact_hash":fact_hash,"allocation_id":"allocation-one"}});
    let binding = json!({"evidence_id":"unit-first","instrument":"TEST","chong_id":"opaque-one","operation_level":"2","carrier_ref":request["carrier_ref"],"gamma_ref":request["gamma_ref"],"sigma":"Short","n":"0","parent_voice_id":null,"structure_view":{"profile_id":"unit","execution_level":"2","confirmation_level":"3","nest_depth":"1"},"opening_source":{"fact_key":key,"fact_hash":fact_hash,"allocation_id":"allocation-one","raw_sha256":raw_sha,"json_pointer":"/events/0","quantity":"8","units":doc["units"]}});
    (image, request, binding)
}
#[test]
fn immutable_binding_preserves_economic_core_and_tuple_hash() {
    let (image, p, b) = state();
    let verified = evidence::unit_verified(b);
    let (next, record) =
        voice::bind_image(&image, &config(), &p, &verified, "25", "26", "27", 2).unwrap();
    let mut economic = next.clone();
    economic["book"]["voice"] = image["book"]["voice"].clone();
    assert_eq!(economic, image);
    assert_eq!(record["identity"]["n"], "0");
    assert_eq!(record["business_lifecycle"]["status"], "NotRecorded");
    assert_eq!(record["responsibility_tail"]["status"], "Unknown");
    assert_eq!(
        record["voice_id"],
        book::hash(
            &json!({"schema":"voice-identity/1","domain_id":"B:opaque-one","session_id":"unit","session_generation":"1","chong_id":"opaque-one","identity":record["identity"]})
        )
    );
    let mut retry = p.clone();
    retry["binding_id"] = json!("second-id");
    assert!(
        voice::bind_image(&next, &config(), &retry, &verified, "28", "29", "30", 3)
            .unwrap_err()
            .starts_with("IdentityConflict")
    );
}
#[test]
fn distinct_axes_and_authoritative_basis_are_checked() {
    let (image, p, b) = state();
    for (field, value) in [
        ("instrument", json!("OTHER")),
        ("operation_level", json!("3")),
        ("chong_id", json!("other")),
    ] {
        let mut bad = b.clone();
        bad[field] = value;
        assert!(voice::bind_image(
            &image,
            &config(),
            &p,
            &evidence::unit_verified(bad),
            "25",
            "26",
            "27",
            2
        )
        .is_err());
    }
    let mut bad = b.clone();
    bad["opening_source"]["quantity"] = json!("12");
    assert!(voice::bind_image(
        &image,
        &config(),
        &p,
        &evidence::unit_verified(bad),
        "25",
        "26",
        "27",
        2
    )
    .is_err());
    let mut bad = b.clone();
    bad["structure_view"]["nest_depth"] = json!("2");
    assert!(voice::bind_image(
        &image,
        &config(),
        &p,
        &evidence::unit_verified(bad),
        "25",
        "26",
        "27",
        2
    )
    .is_err());
    for key in ["verified", "nest_confirmed", "certificate"] {
        let mut bad = p.clone();
        bad[key] = json!(true);
        assert!(voice::validate_request(&bad).is_err());
    }
    assert!(voice::bind_image(
        &image,
        &config(),
        &p,
        &evidence::unit_verified(b),
        "10",
        "11",
        "27",
        2
    )
    .is_err());
}
#[test]
fn parent_voice_relation_is_not_nest_parent_or_reopen_generation() {
    let (image, p, b) = state();
    let (image, parent) = voice::bind_image(
        &image,
        &config(),
        &p,
        &evidence::unit_verified(b.clone()),
        "25",
        "26",
        "27",
        2,
    )
    .unwrap();
    let mut child = p.clone();
    child["binding_id"] = json!("child");
    child["evidence_id"] = json!("unit-child");
    child["parent_voice_id"] = parent["voice_id"].clone();
    child["n"] = json!("1");
    child["sigma"] = json!("Long");
    let mut claim = b.clone();
    for k in ["evidence_id", "parent_voice_id", "n", "sigma"] {
        claim[k] = child[k].clone();
    }
    let (_, record) = voice::bind_image(
        &image,
        &config(),
        &child,
        &evidence::unit_verified(claim.clone()),
        "28",
        "29",
        "30",
        3,
    )
    .unwrap();
    assert_eq!(record["identity"]["n"], "1");
    for (key, value) in [
        ("commit_seq", json!("3")),
        ("commit_ns", json!("31")),
        ("first_known_ns", json!("31")),
    ] {
        let mut wrong = image.clone();
        wrong["book"]["voice"]["records"][0][key] = value;
        assert!(voice::bind_image(
            &wrong,
            &config(),
            &child,
            &evidence::unit_verified(claim.clone()),
            "28",
            "29",
            "30",
            3
        )
        .is_err());
    }
    for (key, value) in [
        ("n", json!("2")),
        ("sigma", json!("Short")),
        ("parent_voice_id", json!("a".repeat(64))),
    ] {
        let mut request = child.clone();
        let mut wrong = claim.clone();
        request[key] = value.clone();
        wrong[key] = value;
        assert!(voice::bind_image(
            &image,
            &config(),
            &request,
            &evidence::unit_verified(wrong),
            "28",
            "29",
            "30",
            3
        )
        .is_err());
    }
    let mut request = p.clone();
    let mut wrong = b.clone();
    request["n"] = json!("1");
    wrong["n"] = json!("1");
    assert!(voice::bind_image(
        &image,
        &config(),
        &request,
        &evidence::unit_verified(wrong),
        "28",
        "29",
        "30",
        3
    )
    .is_err());
}
#[test]
fn unconfigured_authority_never_accepts_client_evidence() {
    assert!(evidence::load(&config(), "any", "unit")
        .err()
        .unwrap()
        .starts_with("MissingDependency"));
    assert!(evidence::verify_witness(&config(), &json!({"verified":true})).is_err());
}
fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":"unit","session_generation":"1","message_id":message,"source_namespace":"test","source_epoch":"one","producer_id":"unit-producer","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}
fn setup_authority(space: &UnitSpace, binding: Value) -> Value {
    let mut artifacts = Vec::new();
    for name in [
        "input",
        "controlled_gammas",
        "carrier_objects",
        "instrument_declaration",
        "opening_raw",
    ] {
        let value = if name == "opening_raw" {
            book::tests::document()
        } else {
            json!({"scope":"unit trust-root fixture, not a real gamma proof","name":name})
        };
        let (path, sha, n) = space.write(&format!("{name}.json"), &value);
        artifacts.push(json!({"name":name,"path":path,"sha256":sha,"bytes":n.to_string()}));
    }
    let bundle = json!({"schema":"voice-binding-authority/1","authority_id":"unit","source":{"instrument":"TEST","input_sha256":artifacts[0]["sha256"],"profile_id":"unit","first_confirming_observation_source":"25","clock_mapping":{"kind":"TestOnlyAffineOrdinalToNs","source_origin":"0","nanosecond_origin":"0","nanoseconds_per_ordinal":"1"}},"artifacts":artifacts,"bindings":[binding]});
    let (bp, bh, _) = space.write("authority.json", &bundle);
    let review = json!({"schema":"voice-binding-independent-verification/1","verdict":"APPROVE_CONTROLLED_VOICE_BINDING","authority_id":"unit","bundle_sha256":bh,"artifacts":bundle["artifacts"],"verified_evidence_ids":["unit-first"],"scope":"unit trusted receipt fixture only; no actual gamma verification claim"});
    let (rp, rh, _) = space.write("review.json", &review);
    json!({"schema_revision":"voice-binding/1","authorities":[{"authority_id":"unit","bundle_path":bp,"bundle_sha256":bh,"verification_path":rp,"verification_sha256":rh,"max_bytes":"1048576"}]})
}
#[test]
fn durable_voice_commit_replay_original_receipt_and_strong_audit() {
    let space = UnitSpace::new();
    let (_, binding_request, binding) = state();
    let authority = setup_authority(&space, binding);
    let manifest = json!({"schema_revision":"economic-manifest/1","session_id":"unit","session_generation":"1","e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},"chongs":[{"chong_id":"opaque-one","symbol":"TEST","operation_level":"2","fixed_q":"12","db":space.0.join("b.db"),"socket":space.0.join("b.sock")}],"authorized_sources":[{"source_namespace":"test","source_epoch":"one","producer_id":"unit-producer","producer_epoch":"1","operations":["CaptureExternal","ApplyAllocation","BindVoice","ReadView","QueryVoice"]}],"resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},"clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["40"],"messages":{"capture":{"received_ns":"10","first_known_ns":"11","commit_ns":"12"},"apply":{"received_ns":"20","first_known_ns":"21","commit_ns":"22"},"bind":{"received_ns":"30","first_known_ns":"31","commit_ns":"32"}}}},"voice_binding":authority});
    let (path, _, _) = space.write("manifest.json", &manifest);
    let ec = store::Config::load(std::path::Path::new(&path), &space.0.join("e.db"), None).unwrap();
    let bc = store::Config::load(
        std::path::Path::new(&path),
        &space.0.join("b.db"),
        Some("opaque-one"),
    )
    .unwrap();
    store::init(&ec).unwrap();
    store::init(&bc).unwrap();
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let cap = envelope(
        "capture",
        json!({"op":"CaptureExternal","raw_utf8":canonical_json(&book::tests::document()),"json_pointer":"/events/0","token":"unit-token","source_event_time":null}),
    );
    let captured = service::handle(&mut e, &ec, 1, 1, &json!({}), &cap);
    assert_eq!(captured["payload"]["kind"], "Recorded", "{captured}");
    let application = envelope(
        "apply",
        json!({"op":"ApplyAllocation","fact_key":captured["payload"]["fact_key"],"allocation_id":"allocation-one","expected_fact_hash":captured["payload"]["fact_hash"]}),
    );
    let applied = service::handle(&mut b, &bc, 1, 1, &json!({}), &application);
    assert_eq!(applied["payload"]["kind"], "Applied", "{applied}");
    let before = store::latest(&b).unwrap();
    let request = envelope("bind", binding_request.clone());
    let bound = service::handle(&mut b, &bc, 1, 1, &json!({}), &request);
    assert_eq!(bound["payload"]["kind"], "VoiceBound", "{bound}");
    store::audit(&b, &bc).unwrap();
    let after = store::latest(&b).unwrap();
    assert_eq!(after.0, before.0 + 1);
    assert_eq!(after.3["recorded"], before.3["recorded"]);
    assert_eq!(after.3["applied"], before.3["applied"]);
    let old_cut = service::handle(
        &mut b,
        &bc,
        1,
        1,
        &json!({}),
        &envelope(
            "old-read",
            json!({"op":"ReadView","cut":null,"as_known_ns":"30"}),
        ),
    );
    assert_eq!(old_cut["payload"]["image"], before.3);
    let query = service::handle(
        &mut b,
        &bc,
        1,
        1,
        &json!({}),
        &envelope("query", json!({"op":"QueryVoice","binding_id":"first"})),
    );
    assert_eq!(query["payload"]["receipt"], bound["payload"]);
    // 固定外部证据稍后不可读，永久原提交仍从已保存原字节强审计并原样可查。
    std::fs::remove_file(space.0.join("authority.json")).unwrap();
    store::audit(&b, &bc).unwrap();
    assert_eq!(
        service::handle(&mut b, &bc, 1, 1, &json!({}), &request),
        bound
    );
    let alias = service::handle(
        &mut b,
        &bc,
        1,
        1,
        &json!({}),
        &envelope("alias", binding_request.clone()),
    );
    assert_eq!(alias["payload"], bound["payload"]);
    store::audit(&b, &bc).unwrap();
    let mut changed = binding_request.clone();
    changed["sigma"] = json!("Long");
    let conflict = service::handle(&mut b, &bc, 1, 1, &json!({}), &envelope("bind", changed));
    assert!(conflict["payload"]["error"]
        .as_str()
        .unwrap()
        .starts_with("IdentityConflict"));
    assert_eq!(store::latest(&b).unwrap(), after);
    // 非默认 Voice 元数据/只物化或只回执篡改均不得被新增类型分派放过。
    for sql in ["UPDATE book SET json=json_set(json,'$.voice.records[0].identity.n','7')", "UPDATE receipts SET json=json_set(json,'$.record.identity.n','7') WHERE k LIKE 'voice-binding/1:%'", "DELETE FROM messages WHERE request_json LIKE '%\"message_id\":\"bind\"%'", "DELETE FROM meta WHERE k='voice_binding_revision'", "INSERT INTO receipts VALUES('orphan','{}')"] {
        b.execute_batch("SAVEPOINT corrupt").unwrap();b.execute_batch(sql).unwrap();assert!(store::audit(&b,&bc).is_err(),"{sql}");b.execute_batch("ROLLBACK TO corrupt; RELEASE corrupt").unwrap();store::audit(&b,&bc).unwrap();
    }
    drop(b);
    store::recover(&bc).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let recovered = service::handle(&mut b, &bc, 2, 1, &json!({}), &request);
    assert_eq!(recovered["payload"], bound["payload"]);
    assert_eq!(recovered["producer_epoch"], "2");
    assert_eq!(store::latest(&b).unwrap(), after);
    store::audit(&b, &bc).unwrap();
    drop(b);
    drop(e);
}
#[test]
fn authority_pin_and_original_bytes_are_required() {
    let space = UnitSpace::new();
    let (_, _, binding) = state();
    let mut cfg = config();
    cfg.manifest =
        json!({"clock":{"kind":"TestOnly"},"voice_binding":setup_authority(&space,binding)});
    let verified = evidence::load(&cfg, "unit-first", "unit").unwrap();
    let witness = verified.witness();
    evidence::verify_witness(&cfg, &witness).unwrap();
    let mut bad = witness.clone();
    bad["verification_raw_utf8"] = json!("{\"verified\":true}");
    assert!(evidence::verify_witness(&cfg, &bad).is_err());
    std::fs::write(space.0.join("input.json"), "{} ").unwrap();
    assert!(evidence::load(&cfg, "unit-first", "unit").is_err());
    // durable witness 可在外部失联后复核固定 pin；任一原始字节改动不再可复核。
    evidence::verify_witness(&cfg, &witness).unwrap();
    let mut bad = witness;
    let raw = bad["bundle_raw_utf8"].as_str().unwrap().to_owned() + " ";
    bad["bundle_raw_utf8"] = json!(raw);
    assert!(evidence::verify_witness(&cfg, &bad).is_err());
}
#[test]
fn unrelated_evidence_authority_failure_does_not_block_selected_authority() {
    let space = UnitSpace::new();
    let (_, _, binding) = state();
    let mut cfg = config();
    cfg.manifest =
        json!({"clock":{"kind":"TestOnly"},"voice_binding":setup_authority(&space,binding)});
    let mut unrelated = cfg.manifest["voice_binding"]["authorities"][0].clone();
    unrelated["authority_id"] = json!("unrelated");
    unrelated["bundle_path"] = json!(space.0.join("absent"));
    cfg.manifest["voice_binding"]["authorities"]
        .as_array_mut()
        .unwrap()
        .insert(0, unrelated);
    assert!(evidence::load(&cfg, "unit-first", "unit").is_ok());
    assert!(evidence::load(&cfg, "unit-first", "unrelated").is_err());
}
#[cfg(unix)]
#[test]
fn authority_pipe_is_rejected_without_waiting_for_a_writer() {
    let space = UnitSpace::new();
    let (_, _, binding) = state();
    let mut cfg = config();
    cfg.manifest =
        json!({"clock":{"kind":"TestOnly"},"voice_binding":setup_authority(&space,binding)});
    let path = space.0.join("input.json");
    std::fs::remove_file(&path).unwrap();
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    assert!(evidence::load(&cfg, "unit-first", "unit")
        .err()
        .unwrap()
        .contains("普通文件"));
}
#[test]
fn explicit_gamma_clock_mapping_prevents_backdating_binding() {
    let space = UnitSpace::new();
    let (image, p, binding) = state();
    let mut cfg = config();
    cfg.manifest =
        json!({"clock":{"kind":"TestOnly"},"voice_binding":setup_authority(&space,binding)});
    let verified = evidence::load(&cfg, "unit-first", "unit").unwrap();
    assert_eq!(verified.observation_ns(), 25);
    assert!(
        voice::bind_image(&image, &cfg, &p, &verified, "21", "22", "23", 2)
            .unwrap_err()
            .contains("γ 确认观察")
    );
    assert!(voice::bind_image(&image, &cfg, &p, &verified, "25", "26", "27", 2).is_ok());
}
#[test]
fn pinned_invalid_metadata_is_rejected_identically_live_and_replay() {
    let space = UnitSpace::new();
    let (_, _, binding) = state();
    let mut original = config();
    original.manifest =
        json!({"clock":{"kind":"TestOnly"},"voice_binding":setup_authority(&space,binding)});
    let base: Value =
        serde_json::from_slice(&std::fs::read(space.0.join("authority.json")).unwrap()).unwrap();
    let original_review: Value =
        serde_json::from_slice(&std::fs::read(space.0.join("review.json")).unwrap()).unwrap();
    for change in ["single_limit", "total_limit", "relative_path"] {
        let mut bundle = base.clone();
        let mut cfg = original.clone();
        match change {
            "single_limit" => {
                bundle["artifacts"][0]["bytes"] = json!((64_u64 * 1024 * 1024 + 1).to_string())
            }
            "total_limit" => {
                for i in 0..3 {
                    bundle["artifacts"][i]["bytes"] = json!((64_u64 * 1024 * 1024).to_string());
                }
            }
            "relative_path" => bundle["artifacts"][0]["path"] = json!("relative-input"),
            _ => unreachable!(),
        }
        // 人工重固定单位信任根，让反例实际触及元数据规则，而非先死于摘要不等。
        let (bp, bh, _) = space.write("authority.json", &bundle);
        let mut review = original_review.clone();
        review["bundle_sha256"] = json!(bh);
        review["artifacts"] = bundle["artifacts"].clone();
        let (rp, rh, _) = space.write("review.json", &review);
        cfg.manifest["voice_binding"]["authorities"][0]["bundle_path"] = json!(bp);
        cfg.manifest["voice_binding"]["authorities"][0]["bundle_sha256"] = json!(bh);
        cfg.manifest["voice_binding"]["authorities"][0]["verification_path"] = json!(rp);
        cfg.manifest["voice_binding"]["authorities"][0]["verification_sha256"] = json!(rh);
        let witness = json!({"authority_id":"unit","evidence_id":"unit-first","bundle_raw_utf8":canonical_json(&bundle),"verification_raw_utf8":canonical_json(&review)});
        let live = evidence::load(&cfg, "unit-first", "unit").err().unwrap();
        let replay = evidence::verify_witness(&cfg, &witness).err().unwrap();
        assert_eq!(live, replay, "{change}");
        assert!(
            live.contains(if change == "relative_path" {
                "绝对路径"
            } else {
                "明确上限"
            }),
            "{live}"
        );
    }
}
