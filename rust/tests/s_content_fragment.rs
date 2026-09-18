#![cfg(feature = "s_session")]
//! #1394：范围证明只核该片，不代替完整图/来源验收；所有限额为显式测试输入。
use newchan_rust::s_content::{
    self as sc,
    fragment::{self, Fragment, Unit},
    job::{Existing, Job, JobLimits},
    path::PathStep,
    *,
};
use serde_json::{json, Value as J};
use std::time::Instant;

const REPLY: u64 = 262_144;
fn job() -> Job {
    let limits=JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"2097152","max_total_nodes":"10000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"10000","max_staged_bytes":"1048576","max_store_bytes":"4194304"})).unwrap();
    Job::new(
        limits,
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(Instant::now),
    )
    .unwrap()
}
fn f(s: &str) -> PathStep {
    PathStep::Field(s.into())
}
fn roundtrip(
    s: &mut MemoryNodes,
    r: &ContentRef,
    path: &[PathStep],
    u: Unit,
    o: u64,
    l: u64,
) -> Fragment {
    let part = fragment::read(s, r, path, u, o, l, REPLY, &mut job()).unwrap();
    fragment::verify(r, path, u, o, l, &part, REPLY, &mut job()).unwrap();
    assert!(part.nodes.windows(2).all(|p| p[0].root < p[1].root));
    assert!(serde_json::to_vec(&J::Object(part.fields())).unwrap().len() as u64 <= REPLY);
    part
}

#[test]
fn all_units_paths_and_empty_ranges_match_independent_values() {
    let mut s = MemoryNodes::new();
    let text = "a€x".repeat(2400);
    let root = sc::write_value(
        &mut s,
        &Value::Object(vec![
            ("text".into(), Value::string(text.clone())),
            (
                "array".into(),
                Value::Array(vec![
                    Value::Null,
                    Value::Bool(true),
                    Value::Number(NumberToken::parse("-1.25e+2").unwrap()),
                ]),
            ),
            (
                "view".into(),
                Value::JsonText(Box::new(JsonValue::Object(vec![(
                    "inside".into(),
                    JsonValue::String("完整".into()),
                )]))),
            ),
        ]),
    )
    .unwrap()
    .content_ref();
    sc::verify(&mut s, &root).unwrap();
    let object = roundtrip(&mut s, &root, &[], Unit::Items, 0, 9);
    assert_eq!(
        object
            .items
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["key"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["array", "text", "view"]
    );
    let array = roundtrip(&mut s, &root, &[f("array")], Unit::Items, 1, 1);
    assert_eq!(array.total, 3);
    assert_eq!(array.end, 2);
    assert!(!array.done);
    assert_eq!(array.items[0]["kind"], "bool");
    for offset in [0, 4094, 4096, 8190, text.len() as u64] {
        let part = roundtrip(&mut s, &root, &[f("text")], Unit::Bytes, offset, 8);
        let end = (offset + 8).min(text.len() as u64);
        let expected: String = text.as_bytes()[offset as usize..end as usize]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(part.items, json!(expected));
    }
    let number = roundtrip(
        &mut s,
        &root,
        &[f("array"), PathStep::Index(2)],
        Unit::Bytes,
        1,
        4,
    );
    assert_eq!(number.items, json!("312e3235"));
    let null = roundtrip(
        &mut s,
        &root,
        &[f("array"), PathStep::Index(0)],
        Unit::Value,
        0,
        1,
    );
    assert_eq!(null.total, 1);
    assert_eq!(null.items[0]["extent"], J::Null);
    let tail = roundtrip(
        &mut s,
        &root,
        &[f("array"), PathStep::Index(0)],
        Unit::Value,
        1,
        1,
    );
    assert_eq!(tail.items, json!([]));
    assert!(tail.done);
    let view = roundtrip(&mut s, &root, &[f("view")], Unit::Value, 0, 1);
    assert_eq!(view.items[0]["kind"], "json_text");
    let inner = roundtrip(
        &mut s,
        &root,
        &[f("view"), PathStep::JsonText, f("inside")],
        Unit::Bytes,
        0,
        20,
    );
    assert_eq!(
        inner.items,
        json!("完整"
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>())
    );
    for empty in [
        Value::Array(vec![]),
        Value::Object(vec![]),
        Value::string(""),
    ] {
        let root = sc::write_value(&mut s, &empty).unwrap().content_ref();
        let unit = if root.kind == Kind::String {
            Unit::Bytes
        } else {
            Unit::Items
        };
        let part = roundtrip(&mut s, &root, &[], unit, 0, 1);
        assert_eq!(part.end, 0);
        assert_eq!(part.total, 0);
        assert!(part.done);
    }
}

#[test]
fn proof_is_self_contained_and_rejects_tampering() {
    let mut s = MemoryNodes::new();
    let root = sc::write_value(
        &mut s,
        &Value::Array(vec![Value::string("a"), Value::string("b"), Value::Null]),
    )
    .unwrap()
    .content_ref();
    let part = roundtrip(&mut s, &root, &[], Unit::Items, 1, 1);
    let check =
        |p: &Fragment| fragment::verify(&root, &[], Unit::Items, 1, 1, p, REPLY, &mut job());
    for index in 0..part.nodes.len() {
        let mut p = part.clone();
        p.nodes.remove(index);
        assert!(check(&p).is_err(), "缺证明节点{index}须拒绝");
    }
    let mut p = part.clone();
    p.nodes[0].body_hex.push_str("00");
    assert!(check(&p).is_err());
    let mut p = part.clone();
    p.nodes.reverse();
    assert!(check(&p).is_err());
    let mut p = part.clone();
    p.nodes.insert(0, p.nodes[0].clone());
    assert!(check(&p).is_err());
    let mut p = part.clone();
    p.items[0]["extent"] = json!("2");
    assert!(check(&p).is_err());
    let mut p = part.clone();
    p.offset = 0;
    assert!(check(&p).is_err());
    let mut p = part.clone();
    p.done = true;
    assert!(check(&p).is_err());
    let wrong = sc::write_value(&mut s, &Value::Array(vec![Value::string("other")]))
        .unwrap()
        .content_ref();
    assert!(fragment::verify(&wrong, &[], Unit::Items, 1, 1, &part, REPLY, &mut job()).is_err());
    for value in [
        json!({"root":part.nodes[0].root,"tag":"0A","body_hex":""}),
        json!({"root":part.nodes[0].root,"tag":"00","body_hex":"0"}),
        json!({"root":part.nodes[0].root,"tag":"00","body_hex":"","extra":1}),
    ] {
        assert!(fragment::ProofNode::from_json(&value).is_err());
    }
}

#[test]
fn exact_path_units_overflow_and_template_boundaries() {
    for p in [
        json!({}),
        json!([{"index":0}]),
        json!([{"index":"01"}]),
        json!([{"field":"a","index":"0"}]),
        json!([{"json_text":false}]),
        json!([{"unknown":true}]),
    ] {
        assert!(fragment::parse_path(&p).is_err());
    }
    assert_eq!(
        fragment::parse_path(&json!([{"field":"a"},{"index":"0"},{"json_text":true}])).unwrap(),
        vec![f("a"), PathStep::Index(0), PathStep::JsonText]
    );
    let mut s = MemoryNodes::new();
    let r = sc::write_value(&mut s, &Value::Bool(true))
        .unwrap()
        .content_ref();
    for (u, o, l) in [
        (Unit::Items, 0, 1),
        (Unit::Value, 0, 0),
        (Unit::Value, 0, 2),
        (Unit::Value, 2, 1),
        (Unit::Value, MAX_FRAME_LEN, 1),
    ] {
        assert!(fragment::read(&mut s, &r, &[], u, o, l, REPLY, &mut job()).is_err());
    }
    let r = sc::write_template(&mut s, &Value::Array(vec![Value::BatchSelf]))
        .unwrap()
        .content_ref();
    assert!(fragment::read(&mut s, &r, &[], Unit::Items, 0, 1, REPLY, &mut job()).is_err());
    let nested = frame(tag::JSON_TEXT, &r.digest().unwrap());
    s.put(nested, tag::JSON_TEXT, &r.digest().unwrap()).unwrap();
    let r = ContentRef::new(Kind::JsonText, &nested, None);
    assert!(fragment::read(
        &mut s,
        &r,
        &[PathStep::JsonText],
        Unit::Items,
        0,
        1,
        REPLY,
        &mut job()
    )
    .is_err());
}

fn huge_array(s: &mut MemoryNodes, bits: u32) -> ContentRef {
    let value = sc::write_value(s, &Value::Bool(true)).unwrap();
    let leaf = frame(tag::SEQ_LEAF, &value.digest);
    s.put(leaf, tag::SEQ_LEAF, &value.digest).unwrap();
    let mut seq = SequenceRef::from_parts_unchecked(leaf, 1);
    for _ in 0..bits {
        seq = sequence_concat(s, &seq, &seq).unwrap();
    }
    let mut body = seq.count().to_be_bytes().to_vec();
    body.extend_from_slice(&seq.root());
    let root = frame(tag::ARRAY, &body);
    s.put(root, tag::ARRAY, &body).unwrap();
    ContentRef::new(Kind::Array, &root, Some(seq.count()))
}

#[test]
fn enormous_shared_array_reads_high_offset_without_expansion_and_caps_reply() {
    let mut s = MemoryNodes::new();
    let root = huge_array(&mut s, 51);
    let mut metered = job();
    let part = fragment::read(
        &mut s,
        &root,
        &[],
        Unit::Items,
        (1u64 << 51) - 2,
        1,
        REPLY,
        &mut metered,
    )
    .unwrap();
    assert!(part.nodes.len() < 200);
    assert_eq!(part.items[0]["kind"], "bool");
    fragment::verify(
        &root,
        &[],
        Unit::Items,
        (1u64 << 51) - 2,
        1,
        &part,
        REPLY,
        &mut job(),
    )
    .unwrap();
    let mut same = job();
    let a = fragment::read(&mut s, &root, &[], Unit::Items, 0, 1, REPLY, &mut same).unwrap();
    let before = same.snapshot();
    let b = fragment::read(&mut s, &root, &[], Unit::Items, 0, 1, REPLY, &mut same).unwrap();
    assert_eq!(a, b);
    assert_eq!(before.nodes, same.snapshot().nodes);
    assert_eq!(same.snapshot().wire_bytes, 0, "未发生实际I/O不伪记wire");
    assert_eq!(
        fragment::read(
            &mut s,
            &root,
            &[],
            Unit::Items,
            0,
            1 << 50,
            32_768,
            &mut job()
        )
        .unwrap_err()
        .code(),
        fragment::REPLY_LIMIT
    );
    let exact = serde_json::to_vec(&J::Object(part.fields())).unwrap().len() as u64;
    assert!(fragment::read(
        &mut s,
        &root,
        &[],
        Unit::Items,
        (1u64 << 51) - 2,
        1,
        exact,
        &mut job()
    )
    .is_ok());
    assert_eq!(
        fragment::read(
            &mut s,
            &root,
            &[],
            Unit::Items,
            (1u64 << 51) - 2,
            1,
            exact - 1,
            &mut job()
        )
        .unwrap_err()
        .code(),
        fragment::REPLY_LIMIT
    );
}

#[test]
fn selected_branch_canonical_shape_and_empty_sequence_are_checked() {
    let mut s = MemoryNodes::new();
    let value = sc::write_value(&mut s, &Value::Null).unwrap();
    let leaf = frame(tag::SEQ_LEAF, &value.digest);
    s.put(leaf, tag::SEQ_LEAF, &value.digest).unwrap();
    let mut bad = 0u64.to_be_bytes().to_vec();
    bad.extend_from_slice(&leaf);
    let digest = frame(tag::ARRAY, &bad);
    s.put(digest, tag::ARRAY, &bad).unwrap();
    let root = ContentRef::new(Kind::Array, &digest, Some(0));
    assert!(fragment::read(&mut s, &root, &[], Unit::Items, 0, 1, REPLY, &mut job()).is_err());
    let mut limited = job();
    let limits = limited.snapshot().limits;
    let limits = JobLimits {
        max_total_nodes: 1,
        ..limits
    };
    limited = Job::new(
        limits,
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(Instant::now),
    )
    .unwrap();
    let root = sc::write_value(&mut s, &Value::Array(vec![Value::Bool(true)]))
        .unwrap()
        .content_ref();
    assert!(fragment::read(&mut s, &root, &[], Unit::Items, 0, 1, REPLY, &mut limited).is_err());
    assert!(limited.snapshot().stopped.is_some());
}

#[test]
fn empty_end_range_at_largest_extent_checks_add_before_clamping() {
    let mut s = MemoryNodes::new();
    let leaf_value = sc::write_value(&mut s, &Value::Null).unwrap();
    let leaf = frame(tag::SEQ_LEAF, &leaf_value.digest);
    s.put(leaf, tag::SEQ_LEAF, &leaf_value.digest).unwrap();
    let empty = frame(tag::SEQ_EMPTY, &[]);
    s.put(empty, tag::SEQ_EMPTY, &[]).unwrap();
    let mut power = SequenceRef::from_parts_unchecked(leaf, 1);
    let mut all = SequenceRef::from_parts_unchecked(empty, 0);
    for i in 0..63 {
        all = sequence_concat(&mut s, &all, &power).unwrap();
        if i < 62 {
            power = sequence_concat(&mut s, &power, &power).unwrap();
        }
    }
    assert_eq!(all.count(), MAX_FRAME_LEN);
    let mut body = all.count().to_be_bytes().to_vec();
    body.extend_from_slice(&all.root());
    let digest = frame(tag::ARRAY, &body);
    s.put(digest, tag::ARRAY, &body).unwrap();
    let root = ContentRef::new(Kind::Array, &digest, Some(MAX_FRAME_LEN));
    let p = roundtrip(&mut s, &root, &[], Unit::Items, MAX_FRAME_LEN, 1);
    assert!(p.done);
    assert_eq!(p.end, MAX_FRAME_LEN);
    assert_eq!(p.items, json!([]));
}

#[test]
fn compact_huge_object_key_is_rejected_before_expansion() {
    let mut s = MemoryNodes::new();
    let block = vec![b'a'; 4096];
    let chunk = frame(tag::BYTES_CHUNK, &block);
    s.put(chunk, tag::BYTES_CHUNK, &block).unwrap();
    let leaf = frame(tag::SEQ_LEAF, &chunk);
    s.put(leaf, tag::SEQ_LEAF, &chunk).unwrap();
    let mut seq = SequenceRef::from_parts_unchecked(leaf, 1);
    for _ in 0..31 {
        seq = sequence_concat(&mut s, &seq, &seq).unwrap();
    }
    let mut text_body = (seq.count() * 4096).to_be_bytes().to_vec();
    text_body.extend_from_slice(&seq.root());
    let key = frame(tag::STRING, &text_body);
    s.put(key, tag::STRING, &text_body).unwrap();
    let value = sc::write_value(&mut s, &Value::Null).unwrap();
    let mut field_body = key.to_vec();
    field_body.extend_from_slice(&value.digest);
    let field = frame(tag::OBJECT_FIELD, &field_body);
    s.put(field, tag::OBJECT_FIELD, &field_body).unwrap();
    let leaf = frame(tag::SEQ_LEAF, &field);
    s.put(leaf, tag::SEQ_LEAF, &field).unwrap();
    let mut object_body = 1u64.to_be_bytes().to_vec();
    object_body.extend_from_slice(&leaf);
    let digest = frame(tag::OBJECT, &object_body);
    s.put(digest, tag::OBJECT, &object_body).unwrap();
    let root = ContentRef::new(Kind::Object, &digest, Some(1));
    // 保证旧实现负例也只读取有限次，不真的展开8TiB或依赖OOM/机器速度。
    struct Guard<'a> {
        source: &'a mut MemoryNodes,
        reads: usize,
    }
    impl NodeSource for Guard<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.reads += 1;
            if self.reads > 2000 {
                return Err(CodecError::new(
                    "test_expansion_guard",
                    "对象键已在回复预算前展开",
                ));
            }
            self.source.read(digest)
        }
    }
    let mut source = Guard {
        source: &mut s,
        reads: 0,
    };
    assert_eq!(
        fragment::read(
            &mut source,
            &root,
            &[],
            Unit::Items,
            0,
            1,
            REPLY,
            &mut job()
        )
        .unwrap_err()
        .code(),
        fragment::REPLY_LIMIT
    );
    assert!(source.reads < 2000);
}

#[test]
fn object_key_escaping_uses_serialized_reply_budget() {
    let mut source = MemoryNodes::new();
    let key = "\0".repeat(50_000);
    let root = sc::write_value(
        &mut source,
        &Value::Object(vec![(key.clone(), Value::Null)]),
    )
    .unwrap()
    .content_ref();
    // raw key 小于限额，但JSON中的每个NUL需要六字节，须在输出时限额。
    assert!(key.len() < REPLY as usize);
    assert_eq!(
        fragment::read(
            &mut source,
            &root,
            &[],
            Unit::Items,
            0,
            1,
            REPLY,
            &mut job()
        )
        .unwrap_err()
        .code(),
        fragment::REPLY_LIMIT
    );
    let part = fragment::read(
        &mut source,
        &root,
        &[],
        Unit::Items,
        0,
        1,
        1_048_576,
        &mut job(),
    )
    .unwrap();
    assert_eq!(part.items[0]["key"], key);
    let exact = serde_json::to_vec(&J::Object(part.fields())).unwrap().len() as u64;
    assert!(fragment::read(
        &mut source,
        &root,
        &[],
        Unit::Items,
        0,
        1,
        exact,
        &mut job()
    )
    .is_ok());
    assert_eq!(
        fragment::read(
            &mut source,
            &root,
            &[],
            Unit::Items,
            0,
            1,
            exact - 1,
            &mut job()
        )
        .unwrap_err()
        .code(),
        fragment::REPLY_LIMIT
    );
    fragment::verify(&root, &[], Unit::Items, 0, 1, &part, exact, &mut job()).unwrap();
}
