#![cfg(feature = "s_session")]
//! #1394：完整 root[] 与旧逐下标定位对拍，真实节点读取次数按逻辑位置计。
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_array_items, lookup_path, PathStep},
    CodecError, ContentRef, Kind, MemoryNodes, NodeDigest, NodeRead, NodeSink, NodeSource,
    NumberToken, Value,
};

#[derive(Default)]
struct Source {
    nodes: MemoryNodes,
    reads: usize,
    indices: Vec<u64>,
}
impl NodeSource for Source {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.reads += 1;
        self.nodes.read(d)
    }
    fn before_path_index(&mut self, prefix: &[PathStep], i: u64) -> Result<(), CodecError> {
        if prefix.is_empty() {
            self.indices.push(i);
        }
        Ok(())
    }
}
impl NodeSink for Source {
    fn put(&mut self, d: NodeDigest, t: u8, body: &[u8]) -> Result<(), CodecError> {
        self.nodes.put(d, t, body)
    }
}
fn old(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Vec<ContentRef>, CodecError> {
    (0..r.extent.unwrap())
        .map(|i| lookup_path(s, r, &[PathStep::Index(i)]))
        .collect()
}

#[test]
fn complete_array_matches_every_ref_and_reads_each_logical_position_linearly() {
    for n in [16, 0, 1, 3, 7, 65, 257] {
        let mut s = Source::default();
        let values = (0..n)
            .map(|i| match i % 5 {
                0 => Value::Null,
                1 => Value::Array(vec![Value::string("nested"); 100]),
                2 => Value::Object(vec![("a".into(), Value::Bool(true))]),
                3 => Value::Number(NumberToken::parse("12.5").unwrap()),
                _ => Value::string("repeated"),
            })
            .collect();
        let root = sc::write_value(&mut s, &Value::Array(values))
            .unwrap()
            .content_ref();
        let expected = old(&mut s, &root).unwrap();
        let old_reads = s.reads;
        s.reads = 0;
        s.indices.clear();
        let actual = lookup_array_items(&mut s, &root).unwrap();
        assert_eq!(actual, expected, "n={n}：逐 Ref/order/kind/extent");
        assert_eq!(s.indices, (0..n).collect::<Vec<_>>());
        eprintln!("ARRAY_READS n={n} old={old_reads} linear={}", s.reads);
        assert_eq!(
            s.reads,
            if n == 0 { 2 } else { 3 * n as usize },
            "不展开嵌套值，不按摘要去重逻辑位置"
        );
        if n >= 16 {
            assert!(s.reads * 4 < old_reads);
        }
    }
}

fn node(s: &mut Source, t: u8, body: &[u8]) -> NodeDigest {
    let d = sc::frame(t, body);
    s.put(d, t, body).unwrap();
    d
}
fn array(s: &mut Source, seq: NodeDigest, count: u64) -> ContentRef {
    let mut body = count.to_be_bytes().to_vec();
    body.extend_from_slice(&seq);
    ContentRef::new(Kind::Array, &node(s, sc::tag::ARRAY, &body), Some(count))
}
fn branch(s: &mut Source, left: NodeDigest, right: NodeDigest, n: u64) -> NodeDigest {
    let mut body = n.to_be_bytes().to_vec();
    body.extend_from_slice(&left);
    body.extend_from_slice(&right);
    node(s, sc::tag::SEQ_BRANCH, &body)
}
fn same_rejection(s: &mut Source, r: &ContentRef, code: &str) {
    assert_eq!(old(s, r).unwrap_err().code(), code);
    assert_eq!(lookup_array_items(s, r).unwrap_err().code(), code);
}

#[test]
fn complete_array_rejects_actual_bad_sequence_headers_roles_and_child_extents() {
    let mut s = Source::default();
    let null = node(&mut s, sc::tag::NULL, &[]);
    let leaf = node(&mut s, sc::tag::SEQ_LEAF, &null);
    let two = branch(&mut s, leaf, leaf, 2);
    // 非二幂 n=3 的唯一 k=2；交换两侧的合法 F 仍必须拒绝。
    let wrong_k = branch(&mut s, leaf, two, 3);
    let r = array(&mut s, wrong_k, 3);
    same_rejection(&mut s, &r, sc::code::SEQUENCE_SHAPE);
    let wrong_count = branch(&mut s, leaf, leaf, 3);
    let r = array(&mut s, wrong_count, 2);
    same_rejection(&mut s, &r, sc::code::SEQUENCE_SHAPE);
    for sibling in [sc::frame(sc::tag::BOOL_FALSE, &[]), null] {
        let seq = branch(&mut s, leaf, sibling, 2);
        let r = array(&mut s, seq, 2);
        same_rejection(
            &mut s,
            &r,
            if sibling == null {
                sc::code::SEQUENCE_SHAPE
            } else {
                sc::code::NODE_MISSING
            },
        );
    }
    // 叶不能是 M 序列或字段；值头也须检查 body/extent，而非只取 kind。
    for (t, body, code) in [
        (sc::tag::SEQ_EMPTY, vec![], sc::code::SEQUENCE_SHAPE),
        (sc::tag::OBJECT_FIELD, vec![0; 64], sc::code::VALUE_KIND),
        (sc::tag::ARRAY, vec![0; 39], sc::code::BODY_SHAPE),
        (
            sc::tag::STRING,
            [u64::MAX.to_be_bytes().as_slice(), &[0; 32]].concat(),
            sc::code::LENGTH_OVERFLOW,
        ),
    ] {
        let value = node(&mut s, t, &body);
        let seq = node(&mut s, sc::tag::SEQ_LEAF, &value);
        let r = array(&mut s, seq, 1);
        same_rejection(&mut s, &r, code);
    }
    let seq = branch(&mut s, leaf, leaf, 2);
    let r = array(&mut s, seq, 2);
    s.nodes.insert_unchecked(null, sc::tag::NULL, vec![1]);
    same_rejection(&mut s, &r, sc::code::NODE_DIGEST);
}

#[test]
fn root_array_value_mode_preserves_json_text_and_public_template_boundaries() {
    let mut s = Source::default();
    let root = sc::write_template(&mut s, &Value::Array(vec![Value::BatchSelf]))
        .unwrap()
        .content_ref();
    assert_eq!(
        lookup_array_items(&mut s, &root).unwrap(),
        old(&mut s, &root).unwrap()
    );
    sc::verify_template(&mut s, &root).unwrap();
    assert!(
        sc::verify(&mut s, &root).is_err(),
        "公开 0x08 仍由原完整 verify 拒绝"
    );
    let null = node(&mut s, sc::tag::NULL, &[]);
    let text = node(&mut s, sc::tag::JSON_TEXT, &null);
    let batch = node(&mut s, sc::tag::BATCH_SELF, &[]);
    for value in [batch, text] {
        let seq = node(&mut s, sc::tag::SEQ_LEAF, &value);
        let inner = array(&mut s, seq, 1);
        let view = node(&mut s, sc::tag::JSON_TEXT, &inner.digest().unwrap());
        let view = ContentRef::new(Kind::JsonText, &view, None);
        let detached = lookup_path(&mut s, &view, &[PathStep::JsonText]).unwrap();
        // Ref 不携带 JsonInner；同旧 root[]，不能凭引用额外推测上下文。
        assert_eq!(
            lookup_array_items(&mut s, &detached).unwrap(),
            old(&mut s, &detached).unwrap()
        );
        assert_eq!(
            lookup_path(&mut s, &view, &[PathStep::JsonText, PathStep::Index(0)])
                .unwrap_err()
                .code(),
            sc::code::JSON_TEXT_INTERNAL_KIND
        );
        assert_eq!(
            sc::verify(&mut s, &view).unwrap_err().code(),
            sc::code::JSON_TEXT_INTERNAL_KIND
        );
        assert_eq!(
            sc::verify_template(&mut s, &view).unwrap_err().code(),
            sc::code::JSON_TEXT_INTERNAL_KIND
        );
    }
}

#[test]
fn complete_array_checks_root_and_empty_originals() {
    let mut s = Source::default();
    let root = sc::write_value(&mut s, &Value::Array(vec![]))
        .unwrap()
        .content_ref();
    for bad in [
        ContentRef {
            kind: Kind::Object,
            ..root.clone()
        },
        ContentRef {
            extent: Some(1),
            ..root.clone()
        },
        ContentRef {
            format: "wrong".into(),
            ..root.clone()
        },
    ] {
        assert!(lookup_array_items(&mut s, &bad).is_err());
    }
    let missing = sc::frame(sc::tag::BOOL_FALSE, &[]);
    let root = array(&mut s, missing, 0);
    assert_eq!(
        lookup_array_items(&mut s, &root).unwrap_err().code(),
        sc::code::NODE_MISSING
    );
}
