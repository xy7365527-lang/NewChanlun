#![cfg(feature = "s_session")]
//! #1394：同调用短键复用；真实 Merkle 节点、逐读取 F 与 Ref 对拍。
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_object_fields, lookup_object_fields_subset, lookup_path, PathStep},
    CodecError, ContentRef, Kind, MemoryNodes, NodeDigest, NodeRead, NodeSink, NodeSource, Value,
};

#[derive(Default)]
struct Source {
    nodes: MemoryNodes,
    opt_in: bool,
    reads: Vec<NodeDigest>,
    stop: Option<usize>,
    missing: Option<usize>,
    corrupt: Option<usize>,
}
impl NodeSource for Source {
    fn audit_full_array_scan(&self) -> bool {
        self.opt_in
    }
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.reads.push(*d);
        if self.stop == Some(self.reads.len()) {
            return Err(CodecError::new("test_stop", "本次实际源读取拒绝"));
        }
        if self.missing == Some(self.reads.len()) {
            return Ok(NodeRead::Missing);
        }
        if self.corrupt == Some(self.reads.len()) {
            return Ok(NodeRead::Found {
                tag: sc::tag::NULL,
                body: vec![1],
            });
        }
        self.nodes.read(d)
    }
}
impl NodeSink for Source {
    fn put(&mut self, d: NodeDigest, t: u8, body: &[u8]) -> Result<(), CodecError> {
        self.nodes.put(d, t, body)
    }
}
fn measured(
    s: &mut Source,
    r: &ContentRef,
    names: &[&str],
    exact: bool,
) -> (
    Result<std::collections::BTreeMap<String, ContentRef>, CodecError>,
    usize,
    u64,
) {
    s.reads.clear();
    let f = sc::frame_calls_on_thread();
    let result = if exact {
        lookup_object_fields(s, r, names)
    } else {
        lookup_object_fields_subset(s, r, names)
    };
    (result, s.reads.len(), sc::frame_calls_on_thread() - f)
}
#[test]
fn subset_short_keys_reduce_actual_merkle_reads_15_and_20_take_10() {
    let mut reductions = Vec::new();
    for n in [15, 20] {
        let mut s = Source::default();
        let names = (0..n).map(|i| format!("field-{i:02}")).collect::<Vec<_>>();
        let root = sc::write_value(
            &mut s,
            &Value::Object(
                names
                    .iter()
                    .map(|k| {
                        (
                            k.clone(),
                            Value::Array(vec![Value::string("unexpanded"); 64]),
                        )
                    })
                    .collect(),
            ),
        )
        .unwrap()
        .content_ref();
        let wanted = names[..10].iter().map(String::as_str).collect::<Vec<_>>();
        let legacy = measured(&mut s, &root, &wanted, false);
        let refs = wanted
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    lookup_path(&mut s, &root, &[PathStep::Field((*name).into())]).unwrap(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(legacy.0.as_ref().unwrap(), &refs);
        assert_eq!(legacy.2, legacy.1 as u64);
        s.opt_in = true;
        let actual = measured(&mut s, &root, &wanted, false);
        eprintln!(
            "SUBSET_READS fields={n} take=10 legacy={} actual={} F={}",
            legacy.1, actual.1, actual.2
        );
        assert_eq!(
            actual.0.unwrap(),
            refs,
            "每项 Ref/kind/extent/root/format 等价"
        );
        assert_eq!(actual.2, actual.1 as u64, "每次实际读核 F");
        reductions.push(actual.1 < legacy.1);
    }
    assert!(
        reductions.iter().all(|reduced| *reduced),
        "真实读取计数红测试"
    );
}

#[test]
fn exact_and_ordinary_path_ignore_opt_in_and_default_subset_trace_is_frozen() {
    let mut s = Source::default();
    let names = (0..15).map(|i| format!("field-{i:02}")).collect::<Vec<_>>();
    let root = sc::write_value(
        &mut s,
        &Value::Object(names.iter().map(|k| (k.clone(), Value::Null)).collect()),
    )
    .unwrap()
    .content_ref();
    let keys = names.iter().map(String::as_str).collect::<Vec<_>>();
    let subset = measured(&mut s, &root, &keys[..10], false);
    assert_eq!((subset.1, subset.2), (199, 199), "默认false原读取和F锁");
    {
        let exact = true;
        let old = measured(&mut s, &root, &keys, exact);
        let trace = s.reads.clone();
        s.opt_in = true;
        assert_eq!(measured(&mut s, &root, &keys, exact), old);
        assert_eq!(s.reads, trace);
        // Exact 的每次瞬时源错误位置与 F 保持一致。
        for stop in 1..=old.1 {
            s.stop = Some(stop);
            s.opt_in = false;
            let expected = measured(&mut s, &root, &keys, exact);
            let trace = s.reads.clone();
            s.opt_in = true;
            assert_eq!(measured(&mut s, &root, &keys, exact), expected);
            assert_eq!(s.reads, trace);
        }
        s.stop = None;
        s.opt_in = false;
    }
    for name in &keys {
        s.reads.clear();
        let f = sc::frame_calls_on_thread();
        let old = lookup_path(&mut s, &root, &[PathStep::Field((*name).into())]);
        let frames = sc::frame_calls_on_thread() - f;
        let trace = s.reads.clone();
        s.reads.clear();
        s.opt_in = true;
        let f = sc::frame_calls_on_thread();
        assert_eq!(
            lookup_path(&mut s, &root, &[PathStep::Field((*name).into())]),
            old
        );
        assert_eq!(s.reads, trace);
        assert_eq!(sc::frame_calls_on_thread() - f, frames);
        s.opt_in = false;
    }
}

fn node(s: &mut Source, t: u8, body: &[u8]) -> NodeDigest {
    let d = sc::frame(t, body);
    s.put(d, t, body).unwrap();
    d
}
fn branch(s: &mut Source, a: NodeDigest, b: NodeDigest, n: u64) -> NodeDigest {
    node(
        s,
        sc::tag::SEQ_BRANCH,
        &[n.to_be_bytes().as_slice(), &a, &b].concat(),
    )
}
fn string(s: &mut Source, seq: NodeDigest, n: u64) -> NodeDigest {
    node(
        s,
        sc::tag::STRING,
        &[n.to_be_bytes().as_slice(), &seq].concat(),
    )
}
fn key(s: &mut Source, bytes: &[u8]) -> NodeDigest {
    if bytes.is_empty() {
        let empty = node(s, sc::tag::SEQ_EMPTY, &[]);
        return string(s, empty, 0);
    }
    let chunk = node(s, sc::tag::BYTES_CHUNK, bytes);
    let seq = node(s, sc::tag::SEQ_LEAF, &chunk);
    string(s, seq, bytes.len() as u64)
}
fn object(s: &mut Source, fields: &[(NodeDigest, NodeDigest)]) -> ContentRef {
    let leaves = fields
        .iter()
        .map(|(k, v)| {
            let f = node(
                s,
                sc::tag::OBJECT_FIELD,
                &[k.as_slice(), v.as_slice()].concat(),
            );
            node(s, sc::tag::SEQ_LEAF, &f)
        })
        .collect::<Vec<_>>();
    fn seq(s: &mut Source, leaves: &[NodeDigest]) -> NodeDigest {
        match leaves.len() {
            0 => node(s, sc::tag::SEQ_EMPTY, &[]),
            1 => leaves[0],
            n => {
                let k = sc::split_index(n as u64).unwrap() as usize;
                let a = seq(s, &leaves[..k]);
                let b = seq(s, &leaves[k..]);
                branch(s, a, b, n as u64)
            }
        }
    }
    let seq = seq(s, &leaves);
    let root = node(
        s,
        sc::tag::OBJECT,
        &[(fields.len() as u64).to_be_bytes().as_slice(), &seq].concat(),
    );
    ContentRef::new(Kind::Object, &root, Some(fields.len() as u64))
}

#[test]
fn short_boundaries_unicode_mixed_long_unknown_and_shared_digests_return_same_refs() {
    for names in [
        vec!["".into(), "a".into(), "é".into(), "缠".into()],
        vec!["".into(), "a".repeat(4096), "b".repeat(4097), "z".into()],
        vec!["a".repeat(4097), "b".into(), "c".repeat(8193)],
    ] {
        let mut s = Source::default();
        // 相同键摘要同时作为未展开嵌套对象的键，不能跨对象/调用复用。
        let root = sc::write_value(
            &mut s,
            &Value::Object(
                names
                    .iter()
                    .map(|name| {
                        (
                            name.clone(),
                            Value::Object(vec![(name.clone(), Value::Null)]),
                        )
                    })
                    .collect(),
            ),
        )
        .unwrap()
        .content_ref();
        sc::verify(&mut s, &root).unwrap();
        for requested in [
            names.clone(),
            vec![],
            vec![names[0].clone()],
            vec![names.last().unwrap().clone()],
        ] {
            let wanted = requested.iter().map(String::as_str).collect::<Vec<_>>();
            s.opt_in = false;
            let expected = measured(&mut s, &root, &wanted, false).0.unwrap();
            for _ in 0..2 {
                s.opt_in = true;
                assert_eq!(measured(&mut s, &root, &wanted, false).0.unwrap(), expected);
            }
        }
        assert!(lookup_object_fields_subset(&mut s, &root, &["missing"]).is_err());
        assert_eq!(
            lookup_object_fields_subset(&mut s, &root, &["a", "a"])
                .unwrap_err()
                .code(),
            sc::code::DUPLICATE_KEY
        );
    }
}

#[test]
fn short_key_rejects_branch_extra_blocks_wrong_lengths_and_all_utf8() {
    let mut s = Source {
        opt_in: true,
        ..Source::default()
    };
    let v = node(&mut s, sc::tag::NULL, &[]);
    let c = node(&mut s, sc::tag::BYTES_CHUNK, b"x");
    let leaf = node(&mut s, sc::tag::SEQ_LEAF, &c);
    let two = branch(&mut s, leaf, leaf, 2);
    let wrong_tag = node(&mut s, sc::tag::NULL, &[]);
    let wrong_leaf = node(&mut s, sc::tag::SEQ_LEAF, &wrong_tag);
    let over = node(&mut s, sc::tag::BYTES_CHUNK, &vec![b'x'; 4097]);
    let over = node(&mut s, sc::tag::SEQ_LEAF, &over);
    let empty_chunk = node(&mut s, sc::tag::BYTES_CHUNK, &[]);
    let empty_chunk = node(&mut s, sc::tag::SEQ_LEAF, &empty_chunk);
    let invalid = key(&mut s, b"a\xff");
    let truncated = key(&mut s, b"a\xe4\xb8");
    let mut cases = vec![
        (string(&mut s, two, 1), sc::code::SEQUENCE_SHAPE),
        (string(&mut s, two, 2), sc::code::SEQUENCE_SHAPE),
        (string(&mut s, leaf, 0), sc::code::SEQUENCE_SHAPE),
        (string(&mut s, leaf, 2), sc::code::BYTE_EXTENT),
        (string(&mut s, wrong_leaf, 1), sc::code::BYTE_CHUNK_SHAPE),
        (string(&mut s, over, 4096), sc::code::BYTE_CHUNK_SHAPE),
        (string(&mut s, empty_chunk, 1), sc::code::BYTE_CHUNK_SHAPE),
        (invalid, sc::code::UTF8),
        (truncated, sc::code::UTF8),
    ];
    // 非 STRING 键，即使请求名在首差处已不匹配也拒绝。
    cases.push((v, sc::code::VALUE_KIND));
    for (k, code) in cases {
        let root = object(&mut s, &[(k, v)]);
        let error = lookup_object_fields_subset(&mut s, &root, &["z"]).unwrap_err();
        assert_eq!(error.code(), code, "{error:?}");
        assert!(
            sc::verify(&mut s, &root).is_err(),
            "完整审计必须拒绝；不声称首错位置等价"
        );
    }
}

#[test]
fn unknown_order_duplicates_missing_and_selected_value_headers_still_reject() {
    let mut s = Source {
        opt_in: true,
        ..Source::default()
    };
    let v = node(&mut s, sc::tag::NULL, &[]);
    let a = key(&mut s, b"a");
    let b = key(&mut s, b"b");
    let c = key(&mut s, b"c");
    for (fields, code) in [
        (vec![(a, v), (c, v), (b, v)], sc::code::OBJECT_KEY_ORDER),
        (vec![(a, v), (b, v), (b, v)], sc::code::DUPLICATE_KEY),
    ] {
        let root = object(&mut s, &fields);
        assert_eq!(
            lookup_object_fields_subset(&mut s, &root, &["a"])
                .unwrap_err()
                .code(),
            code
        );
        assert!(sc::verify(&mut s, &root).is_err());
    }
    let root = object(&mut s, &[(a, v), (c, v)]);
    assert_eq!(
        lookup_object_fields_subset(&mut s, &root, &["b"])
            .unwrap_err()
            .code(),
        sc::path::path_code::PATH_FIELD_MISSING
    );
    assert!(lookup_object_fields_subset(&mut s, &root, &["z"]).is_err());
    for (t, body) in [
        (sc::tag::ARRAY, vec![0; 39]),
        (sc::tag::OBJECT_FIELD, vec![0; 64]),
    ] {
        let bad = node(&mut s, t, &body);
        let root = object(&mut s, &[(a, bad), (b, v)]);
        assert!(lookup_object_fields_subset(&mut s, &root, &["a"]).is_err());
        // 未选值不展开，不擅自代替闭包核验。
        assert!(lookup_object_fields_subset(&mut s, &root, &["b"]).is_ok());
        assert!(sc::verify(&mut s, &root).is_err());
    }
    // 请求匹配后仍须核该短键全部字节及声明长度。
    let matched = key(&mut s, b"a\xff");
    let root = object(&mut s, &[(matched, v)]);
    assert_eq!(
        lookup_object_fields_subset(&mut s, &root, &["a"])
            .unwrap_err()
            .code(),
        sc::code::UTF8
    );
}

#[test]
fn repeated_logical_keys_and_new_calls_never_fallback_to_saved_success() {
    let mut s = Source {
        opt_in: true,
        ..Source::default()
    };
    let v = node(&mut s, sc::tag::NULL, &[]);
    let k = key(&mut s, b"a");
    let r = object(&mut s, &[(k, v)]);
    let first = measured(&mut s, &r, &["a"], false);
    let trace = s.reads.clone();
    let second = measured(&mut s, &r, &["a"], false);
    assert_eq!(first, second);
    assert_eq!(s.reads, trace);
    // 每一次实际读取失败都拒绝，没有旧条目兜底。
    for stop in 1..=first.1 {
        s.stop = Some(stop);
        assert_eq!(
            measured(&mut s, &r, &["a"], false).0.unwrap_err().code(),
            "test_stop"
        );
    }
    s.stop = None;
    for stop in 1..=first.1 {
        s.missing = Some(stop);
        assert_eq!(
            measured(&mut s, &r, &["a"], false).0.unwrap_err().code(),
            sc::code::NODE_MISSING
        );
        s.missing = None;
        s.corrupt = Some(stop);
        assert_eq!(
            measured(&mut s, &r, &["a"], false).0.unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        s.corrupt = None;
    }
    let duplicate = object(&mut s, &[(k, v), (k, v)]);
    assert_eq!(
        measured(&mut s, &duplicate, &["a"], false)
            .0
            .unwrap_err()
            .code(),
        sc::code::DUPLICATE_KEY
    );
    assert_eq!(
        s.reads.iter().filter(|d| **d == k).count(),
        2,
        "相同摘要仍按两个逻辑位置读"
    );
    // 本调用新原件变化或丢失，不能借上次成功。
    s.nodes.insert_unchecked(k, sc::tag::STRING, vec![0]);
    assert_eq!(
        lookup_object_fields_subset(&mut s, &r, &["a"])
            .unwrap_err()
            .code(),
        sc::code::NODE_DIGEST
    );
}

#[test]
fn subset_keeps_count_k_root_and_detached_value_context_boundaries() {
    let mut s = Source {
        opt_in: true,
        ..Source::default()
    };
    let empty = sc::write_value(&mut s, &Value::Object(vec![]))
        .unwrap()
        .content_ref();
    assert!(lookup_object_fields_subset(&mut s, &empty, &[])
        .unwrap()
        .is_empty());
    let root = sc::write_value(
        &mut s,
        &Value::Object(vec![
            ("a".into(), Value::Null),
            ("b".into(), Value::Bool(true)),
            ("c".into(), Value::Null),
        ]),
    )
    .unwrap()
    .content_ref();
    let (_, body) = s.nodes.get(&root.digest().unwrap()).unwrap();
    let seq: NodeDigest = body[8..40].try_into().unwrap();
    let (_, body) = s.nodes.get(&seq).unwrap();
    let mut body = body.to_vec();
    let left = body[8..40].to_vec();
    let right = body[40..72].to_vec();
    body[8..40].copy_from_slice(&right);
    body[40..72].copy_from_slice(&left);
    let bad_seq = node(&mut s, sc::tag::SEQ_BRANCH, &body);
    let bad_root = node(
        &mut s,
        sc::tag::OBJECT,
        &[3u64.to_be_bytes().as_slice(), &bad_seq].concat(),
    );
    let bad_root = ContentRef::new(Kind::Object, &bad_root, Some(3));
    assert_eq!(
        lookup_object_fields_subset(&mut s, &bad_root, &["a"])
            .unwrap_err()
            .code(),
        sc::code::SEQUENCE_SHAPE
    );
    for wrong in [
        ContentRef {
            kind: Kind::Array,
            ..root.clone()
        },
        ContentRef {
            extent: Some(2),
            ..root.clone()
        },
        ContentRef {
            format: "bad".into(),
            ..root.clone()
        },
    ] {
        assert!(lookup_object_fields_subset(&mut s, &wrong, &["a"]).is_err());
    }
    let k = key(&mut s, b"a");
    let batch = node(&mut s, sc::tag::BATCH_SELF, &[]);
    let object_batch = object(&mut s, &[(k, batch)]);
    let view = node(&mut s, sc::tag::JSON_TEXT, &object_batch.digest().unwrap());
    let view = ContentRef::new(Kind::JsonText, &view, None);
    let detached = lookup_path(&mut s, &view, &[PathStep::JsonText]).unwrap();
    let wanted = lookup_path(&mut s, &detached, &[PathStep::Field("a".into())]).unwrap();
    assert_eq!(
        lookup_object_fields_subset(&mut s, &detached, &["a"]).unwrap()["a"],
        wanted
    );
    assert_eq!(
        lookup_path(
            &mut s,
            &view,
            &[PathStep::JsonText, PathStep::Field("a".into())]
        )
        .unwrap_err()
        .code(),
        sc::code::JSON_TEXT_INTERNAL_KIND
    );
    sc::verify_template(&mut s, &object_batch).unwrap();
    assert!(sc::verify(&mut s, &object_batch).is_err());
    assert!(sc::verify_template(&mut s, &view).is_err());
}
